use super::phases::*;
use crate::DriverError;
use btmesh_common::crypto::device::DeviceKey;
use btmesh_pdu::provisioning::{
    Confirmation, ErrorCode, Failed, Invite, ProvisioningData, ProvisioningPDU,
};
use rand_core::{CryptoRng, RngCore};

pub enum Provisioner {
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
    pub fn new(invitation: &Invite) -> Result<Self, DriverError> {
        Ok(Self::Invitation(Phase::<Invitation>::new(invitation)?))
    }

    pub fn next<RNG: RngCore + CryptoRng>(
        self,
        pdu: &ProvisioningPDU,
        rng: &mut RNG,
    ) -> Result<(Self, ResponsePDU), DriverError> {
        match (self, pdu) {
            // CAPABILITIES
            (Provisioner::Invitation(mut phase), ProvisioningPDU::Capabilities(caps)) => {
                let (start, pk) = phase.capabilities(caps, rng)?;
                Ok((
                    Provisioner::KeyExchange(phase.into()),
                    ResponsePDU::Two([
                        ProvisioningPDU::Start(start),
                        ProvisioningPDU::PublicKey(pk),
                    ]),
                ))
            }
            // PUBLIC KEY
            (Provisioner::KeyExchange(mut phase), ProvisioningPDU::PublicKey(peer_key)) => {
                // TODO: OOB capabilities should determine whether we
                // return a Confirmation here or wait for the device
                // to send us an InputComplete

                // TODO: Deal better with ErrorCode / ParseError / DriverError

                // match phase.calculate_ecdh_provisioner(peer_key, rng) {
                //     Ok(_key) => Ok((Provisioner::Authentication(phase.into()), ResponsePDU::None)),
                //     Err(DriverError::Parse(_)) => Provisioner::fail(ErrorCode::InvalidFormat),
                //     Err(_) => Provisioner::fail(ErrorCode::UnexpectedError),
                // }

                // TODO: this better, i.e. calculate sets random_provisioner and then returns it... wtf?
                let random = phase.calculate_ecdh_provisioner(peer_key, rng)?;
                let phase: Phase<Authentication> = phase.into();
                let confirmation = phase.confirm(&random)?;
                let pdu = ProvisioningPDU::Confirmation(Confirmation { confirmation });
                Ok((Provisioner::Authentication(phase), ResponsePDU::One(pdu)))
            }
            // CONFIRMATION
            (Provisioner::Authentication(mut phase), ProvisioningPDU::Confirmation(value)) => {
                let response = phase.provisioner_confirmation(value)?;
                Ok((
                    Provisioner::Authentication(phase),
                    ResponsePDU::One(ProvisioningPDU::Random(response)),
                ))
            }
            // RANDOM
            (Provisioner::Authentication(mut phase), ProvisioningPDU::Random(value)) => {
                match phase.provisioner_check(value) {
                    Ok(_) => Ok((
                        Provisioner::DataDistribution(phase.into()),
                        ResponsePDU::None, // TODO: not this
                    )),
                    Err(_) => Provisioner::fail(ErrorCode::ConfirmationFailed),
                }
            }
            (current, _) => {
                // if it's an invalid PDU, assume it's just a wayward PDU and ignore, don't break.
                Ok((current, ResponsePDU::None))
            }
        }
    }
    fn fail(error_code: ErrorCode) -> Result<(Provisioner, ResponsePDU), DriverError> {
        Ok((
            Provisioner::Failure,
            ResponsePDU::One(ProvisioningPDU::Failed(Failed { error_code })),
        ))
    }
}
