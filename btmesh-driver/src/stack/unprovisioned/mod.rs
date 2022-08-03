use crate::stack::unprovisioned::provisionee::Provisionee;
use crate::util::hash::FnvHasher;
use crate::DriverError;
use btmesh_common::crypto::device::DeviceKey;
use btmesh_pdu::provisioning::{Capabilities, ProvisioningData, ProvisioningPDU};
use core::hash::{Hash, Hasher};
use embassy::time::{Duration, Instant};
use rand_core::{CryptoRng, RngCore};
use crate::util::deadline::{Deadline, DeadlineFuture};

mod auth_value;
mod provisionee;
mod provisioner;
mod transcript;

pub enum ProvisioningState {
    Response(ProvisioningPDU),
    Data(DeviceKey, ProvisioningData),
    Failed,
}

pub struct UnprovisionedStack {
    provisionee: Option<Provisionee>,
    last_transmit_hash: Option<u64>,
    beacon: Deadline,
}

impl UnprovisionedStack {
    pub fn new(capabilities: Capabilities) -> Self {
        Self {
            provisionee: Some(Provisionee::new(capabilities)),
            last_transmit_hash: None,
            beacon: Deadline::new(Duration::from_secs(3), true),
        }
    }

    pub fn in_progress(&self) -> bool {
        if let Some(provisionee) = &self.provisionee {
            provisionee.in_progress()
        } else {
            false
        }
    }

    pub fn next_beacon_deadline(&self) -> Option<DeadlineFuture<'_>> {
        Some(self.beacon.next())
    }

    pub fn process<RNG: RngCore + CryptoRng>(
        &mut self,
        pdu: &ProvisioningPDU,
        rng: &mut RNG,
    ) -> Result<Option<ProvisioningState>, DriverError> {
        let mut hasher = FnvHasher::default();
        pdu.hash(&mut hasher);
        let hash = hasher.finish();

        if let Some(last_transmit_hash) = self.last_transmit_hash {
            // if the inbound matches the last inbound we responded to,
            // just send off the previous response without mucking with
            // state machine or calculating a new response.
            if last_transmit_hash == hash {
                return match &self.provisionee {
                    Some(p) => match p.response() {
                        Some(pdu) => Ok(Some(ProvisioningState::Response(pdu))),
                        None => Err(DriverError::InvalidState),
                    },
                    None => Err(DriverError::InvalidState),
                };
            }
        }

        if let Some(current_state) = self.provisionee.take() {
            let next_state = current_state.next(pdu, rng)?;

            self.provisionee.replace(next_state);

            match &self.provisionee {
                Some(Provisionee::Complete(device_key, provisioning_data)) => Ok(Some(
                    ProvisioningState::Data(*device_key, *provisioning_data),
                )),
                Some(Provisionee::Failure(..)) => Ok(Some(ProvisioningState::Failed)),
                Some(p) => match p.response() {
                    Some(response) => {
                        self.last_transmit_hash.replace(hash);
                        Ok(Some(ProvisioningState::Response(response)))
                    }
                    None => {
                        self.last_transmit_hash.take();
                        Ok(None)
                    }
                },
                None => unreachable!(),
            }
        } else {
            Err(DriverError::InvalidState)
        }
    }
}
