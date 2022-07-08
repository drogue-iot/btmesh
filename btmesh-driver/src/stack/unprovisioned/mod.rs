use crate::stack::unprovisioned::provisionee::Provisionee;
use crate::util::hash::FnvHasher;
use crate::DriverError;
use btmesh_common::crypto::device::DeviceKey;
use btmesh_pdu::provisioning::{Capabilities, ProvisioningData, ProvisioningPDU};
use core::hash::{Hash, Hasher};
use embassy::time::Instant;
use rand_core::{CryptoRng, RngCore};

mod auth_value;
mod provisionee;
mod transcript;

pub enum ProvisioningState {
    Response(ProvisioningPDU),
    Data(DeviceKey, ProvisioningData),
    Failed,
}

struct LastTransmit {
    pdu: ProvisioningPDU,
    in_response_to_hash: u64,
}

pub struct UnprovisionedStack {
    provisionee: Option<Provisionee>,
    last_transmit: Option<LastTransmit>,
}

impl UnprovisionedStack {
    pub fn new(capabilities: Capabilities) -> Self {
        Self {
            provisionee: Some(Provisionee::new(capabilities)),
            last_transmit: None,
        }
    }

    pub fn in_progress(&self) -> bool {
        if let Some(provisionee) = &self.provisionee {
            provisionee.in_progress()
        } else {
            false
        }
    }

    pub fn next_beacon_deadline(&self) -> Option<Instant> {
        None
    }

    pub fn process<RNG: RngCore + CryptoRng>(
        &mut self,
        pdu: &ProvisioningPDU,
        rng: &mut RNG,
    ) -> Result<Option<ProvisioningState>, DriverError> {
        let mut hasher = FnvHasher::default();
        pdu.hash(&mut hasher);
        let hash = hasher.finish();

        if let Some(last_transmit) = self.last_transmit.as_ref() {
            // if the inbound matches the last inbound we responded to,
            // just send off the previous response without mucking with
            // state machine or calculating a new response.
            if last_transmit.in_response_to_hash == hash {
                return Ok(Some(ProvisioningState::Response(last_transmit.pdu.clone())));
            }
        }

        if let Some(current_state) = self.provisionee.take() {
            let (next_state, response) = current_state.next(pdu, rng)?;

            self.provisionee.replace(next_state);

            if let Some(Provisionee::Complete(device_key, provisioning_data)) = &self.provisionee {
                Ok(Some(ProvisioningState::Data(
                    *device_key,
                    *provisioning_data,
                )))
            } else if let Some(Provisionee::Failure) = &self.provisionee {
                Ok(Some(ProvisioningState::Failed))
            } else if let Some(response) = response {
                // stash our response in case we need to retransmit
                self.last_transmit.replace(LastTransmit {
                    pdu: response.clone(),
                    in_response_to_hash: hash,
                });
                Ok(Some(ProvisioningState::Response(response)))
            } else {
                Ok(None)
            }
        } else {
            Err(DriverError::InvalidState)
        }
    }
}
