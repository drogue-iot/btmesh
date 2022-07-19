use super::phases::*;
use crate::DriverError;
use btmesh_common::crypto::device::DeviceKey;
use btmesh_pdu::provisioning::{ProvisioningData, ProvisioningPDU};
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

impl Provisioner {
    pub fn new() -> Self {
        Self::Invitation(Phase::<Invitation>::default())
    }

    pub fn next<RNG: RngCore + CryptoRng>(
        self,
        pdu: &ProvisioningPDU,
        rng: &mut RNG,
    ) -> Result<(Self, impl Iterator<Item = ProvisioningPDU>), DriverError> {
        match (self, pdu) {
            // CAPABILITIES
            (Provisioner::Invitation(mut device), ProvisioningPDU::Capabilities(caps)) => {
                let response = device.capabilities(caps, rng)?;
                Ok((Provisioner::KeyExchange(device.into()), response))
            }
            (_current, _) => Err(DriverError::InvalidState),
        }
    }
}
