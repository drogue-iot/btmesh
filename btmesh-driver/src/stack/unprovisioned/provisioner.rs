use super::phases::*;
use crate::DriverError;
use btmesh_common::crypto::device::DeviceKey;
use btmesh_pdu::provisioning::{ErrorCode, Failed, ProvisioningData, ProvisioningPDU};
use rand_core::{CryptoRng, RngCore};

pub enum Provisioner {
    Beaconing(Phase<Beaconing>),
    Invitation(Phase<Invitation>),
    KeyExchange(Phase<KeyExchange>),
    Authentication(Phase<Authentication>),
    DataDistribution(Phase<DataDistribution>),
    Complete(DeviceKey, ProvisioningData),
    Failure,
}

pub enum ResponsePDU {
    Two([ProvisioningPDU; 2]),
    One(ProvisioningPDU),
    None,
}

impl Provisioner {
    pub fn new() -> Self {
        Self::Invitation(Phase::<Invitation>::default())
    }

    pub fn next<RNG: RngCore + CryptoRng>(
        self,
        pdu: &ProvisioningPDU,
        rng: &mut RNG,
    ) -> Result<(Self, ResponsePDU), DriverError> {
        match (self, pdu) {
            // CAPABILITIES
            (Provisioner::Invitation(mut prvnr), ProvisioningPDU::Capabilities(caps)) => {
                let response = prvnr.capabilities(caps, rng)?;
                Ok((
                    Provisioner::KeyExchange(prvnr.into()),
                    ResponsePDU::Two(response),
                ))
            }
            (Provisioner::KeyExchange(mut prvnr), ProvisioningPDU::PublicKey(peer_key)) => {
                match prvnr.calculate_ecdh_provisioner(peer_key) {
                    Ok(_key) => Ok((Provisioner::Authentication(prvnr.into()), ResponsePDU::None)),
                    Err(DriverError::Parse(_)) => Provisioner::fail(ErrorCode::InvalidFormat),
                    Err(_) => Provisioner::fail(ErrorCode::UnexpectedError),
                }
            }
            (_current, _) => Err(DriverError::InvalidState),
        }
    }
    fn fail(error_code: ErrorCode) -> Result<(Provisioner, ResponsePDU), DriverError> {
        Ok((
            Provisioner::Failure,
            ResponsePDU::One(ProvisioningPDU::Failed(Failed { error_code })),
        ))
    }
}
