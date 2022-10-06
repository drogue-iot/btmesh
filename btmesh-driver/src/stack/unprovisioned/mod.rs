use crate::stack::unprovisioned::provisionee::Provisionee;
use crate::util::deadline::{Deadline, DeadlineFuture};
use crate::util::hash::FnvHasher;
use crate::DriverError;
use btmesh_common::crypto::device::DeviceKey;
use btmesh_pdu::provisioning::{Capabilities, ProvisioningData, ProvisioningPDU};
use core::future::Future;
use core::hash::{Hash, Hasher};
use embassy_time::{Duration, Timer};
use rand_core::{CryptoRng, RngCore};

mod auth_value;
mod provisionee;
mod provisioner;
mod transcript;

pub enum ProvisioningState {
    Response(ProvisioningPDU),
    Data(DeviceKey, ProvisioningData, ProvisioningPDU),
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
        self.provisionee.as_ref().map_or(false, |p| p.in_progress())
    }

    pub fn next_beacon_deadline(&self) -> Option<DeadlineFuture<'_>> {
        if self.in_progress() {
            None
        } else {
            Some(self.beacon.next())
        }
    }

    pub fn next_retransmit(&self) -> Option<impl Future<Output = ()>> {
        if self.in_progress() {
            Some(Timer::after(Duration::from_millis(1000)))
        } else {
            None
        }
    }

    pub fn retransmit(&self) -> Option<ProvisioningPDU> {
        if let Some(fsm) = &self.provisionee {
            fsm.response()
        } else {
            None
        }
    }

    pub fn process<RNG: RngCore + CryptoRng>(
        &mut self,
        pdu: &ProvisioningPDU,
        rng: &mut RNG,
    ) -> Result<Option<ProvisioningState>, DriverError> {
        let mut hasher = FnvHasher::default();
        pdu.hash(&mut hasher);
        let hash = hasher.finish();

        if let (Some(last_hash), Some(fsm)) = (self.last_transmit_hash, &self.provisionee) {
            // if the inbound matches the last inbound we responded to,
            // just send off the previous response without mucking with
            // state machine or calculating a new response.
            if last_hash == hash {
                return match fsm.response() {
                    Some(pdu) => Ok(Some(ProvisioningState::Response(pdu))),
                    None => Err(DriverError::InvalidState),
                };
            }
        }

        if let Some(current_state) = self.provisionee.take() {
            let next_state = current_state.next(pdu, rng)?;

            self.provisionee.replace(next_state);

            match &self.provisionee {
                Some(p @ Provisionee::Complete(device_key, provisioning_data)) => {
                    Ok(Some(ProvisioningState::Data(
                        *device_key,
                        *provisioning_data,
                        p.response().ok_or(DriverError::InvalidState)?,
                    )))
                }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn in_progress() {
        let unprov = UnprovisionedStack::new(Default::default());
        assert_eq!(unprov.in_progress(), false);
    }
}
