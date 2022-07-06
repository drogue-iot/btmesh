use crate::stack::unprovisioned::provisionee::Provisionee;
use crate::DriverError;
use btmesh_common::crypto::device::DeviceKey;
use btmesh_pdu::provisioning::{Capabilities, ProvisioningData, ProvisioningPDU};
use rand_core::{CryptoRng, RngCore};

mod auth_value;
mod provisionee;
mod transcript;

pub enum ProvisioningState {
    Response(ProvisioningPDU),
    Data(DeviceKey, ProvisioningData),
}

pub struct UnprovisionedStack {
    provisionee: Option<Provisionee>,
}

impl UnprovisionedStack {
    pub fn new(capabilities: Capabilities) -> Self {
        Self {
            provisionee: Some(Provisionee::new(capabilities)),
        }
    }

    pub fn process<RNG: RngCore + CryptoRng>(
        &mut self,
        pdu: &ProvisioningPDU,
        rng: &mut RNG,
    ) -> Result<Option<ProvisioningState>, DriverError> {
        if let Some(current_state) = self.provisionee.take() {
            let (next_state, response) = current_state.next(pdu, rng)?;

            self.provisionee.replace(next_state);

            if let Some(Provisionee::Complete(device_key, provisioning_data)) = &self.provisionee {
                Ok(Some(ProvisioningState::Data(
                    *device_key,
                    *provisioning_data,
                )))
            } else if let Some(response) = response {
                Ok(Some(ProvisioningState::Response(response)))
            } else {
                Ok(None)
            }
        } else {
            Err(DriverError::InvalidState)
        }
    }
}
