#![cfg_attr(not(test), no_std)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(associated_type_defaults)]
#![allow(dead_code)]

use btmesh_bearer::beacon::Beacon;
use btmesh_common::{Seq, Uuid};
use btmesh_pdu::provisioning::Capabilities;
use btmesh_pdu::PDU;
use core::future::{pending, Future};
use embassy::time::Timer;
use embassy::util::{select, Either};
use rand_core::{CryptoRng, RngCore};

mod error;
pub mod stack;

mod util;

use crate::stack::interface::NetworkInterfaces;
use crate::stack::provisioned::network::DeviceInfo;
use crate::stack::provisioned::secrets::Secrets;
use crate::stack::provisioned::sequence::Sequence;
use crate::stack::provisioned::system::UpperMetadata;
use crate::stack::provisioned::{NetworkState, ProvisionedStack};
use crate::stack::unprovisioned::{ProvisioningState, UnprovisionedStack};
use crate::stack::Stack;
pub use error::DriverError;

pub struct Driver<N: NetworkInterfaces, R: RngCore + CryptoRng> {
    stack: Stack,
    network: N,
    rng: R,
    // --
    capabilities: Capabilities,
}

impl<N: NetworkInterfaces, R: RngCore + CryptoRng> Driver<N, R> {
    pub fn new_unprovisioned(network: N, rng: R, capabilities: Capabilities, uuid: Uuid) -> Self {
        let num_elements = capabilities.number_of_elements;
        Self {
            stack: Stack::Unprovisioned {
                stack: UnprovisionedStack::new(capabilities.clone()),
                num_elements,
                uuid,
            },
            rng,
            network,
            capabilities,
        }
    }

    pub fn new_provisioned(
        network: N,
        rng: R,
        device_info: DeviceInfo,
        secrets: Secrets,
        network_state: NetworkState,
        sequence: Sequence,
        capabilities: Capabilities,
    ) -> Self {
        Self {
            stack: Stack::Provisioned {
                stack: ProvisionedStack::new(device_info, secrets, network_state),
                sequence,
            },
            rng,
            network,
            capabilities,
        }
    }

    async fn receive_pdu(&mut self, pdu: &PDU) -> Result<(), DriverError> {
        match (&pdu, &mut self.stack) {
            (
                PDU::Provisioning(pdu),
                Stack::Unprovisioned {
                    stack,
                    num_elements,
                    uuid,
                },
            ) => {
                if let Some(provisioning_state) = stack.process(pdu, &mut self.rng)? {
                    match provisioning_state {
                        ProvisioningState::Failed => {
                            self.stack = Stack::Unprovisioned {
                                stack: UnprovisionedStack::new(self.capabilities.clone()),
                                num_elements: self.capabilities.number_of_elements,
                                uuid: *uuid,
                            };
                        }
                        ProvisioningState::Response(pdu) => {
                            self.network.transmit(&PDU::Provisioning(pdu)).await?;
                        }
                        ProvisioningState::Data(device_key, provisioning_data) => {
                            let primary_unicast_addr = provisioning_data.unicast_address;
                            let device_info = DeviceInfo::new(primary_unicast_addr, *num_elements);
                            let secrets = (device_key, provisioning_data).into();
                            let network_state = provisioning_data.into();

                            self.stack = Stack::Provisioned {
                                stack: ProvisionedStack::new(device_info, secrets, network_state),
                                sequence: Sequence::new(Seq::new(800)),
                            };
                        }
                    }
                }
            }
            (PDU::Network(pdu), Stack::Provisioned { stack, sequence }) => {
                if let Some(result) = stack.process_inbound_network_pdu(pdu)? {
                    if let Some((block_ack, meta)) = result.block_ack {
                        // send outbound block-ack
                        for network_pdu in
                            stack.process_outbound_block_ack(sequence, block_ack, meta)?
                        {
                            // don't error if we can't send.
                            self.network.transmit(&PDU::Network(network_pdu)).await.ok();
                        }
                    }

                    if let Some(_message) = result.message {
                        // dispatch to element(s)
                    }
                }
            }
            _ => {
                // PDU incompatible with stack state; ignore.
            }
        }
        Ok(())
    }

    async fn send_beacon(&self) -> Result<(), DriverError> {
        match &self.stack {
            Stack::Unprovisioned { uuid, .. } => {
                self.network.beacon(Beacon::Unprovisioned(*uuid)).await?;
            }

            Stack::Provisioned { stack, .. } => {
                let network_id = stack.secrets().network_key_by_index(0)?.network_id();
                self.network.beacon(Beacon::Provisioned(network_id)).await?;
            }
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), DriverError> {
        let device_state = self.stack.device_state();

        loop {
            let receive_fut = self.network.receive(&device_state);
            let beacon_fut = self.next_beacon();

            match select(receive_fut, beacon_fut).await {
                Either::First(Ok(pdu)) => {
                    self.receive_pdu(&pdu).await?;
                }
                Either::First(Err(err)) => return Err(err.into()),
                Either::Second(_) => {
                    self.send_beacon().await?;
                }
            }
        }
    }

    fn next_beacon(&self) -> BeaconFuture<'_, N, R> {
        async move {
            if let Some(next_beacon_deadline) = self.stack.next_beacon_deadline() {
                Timer::at(next_beacon_deadline).await
            } else {
                pending().await
            }
        }
    }
}

type BeaconFuture<'f, N, R>
where
    N: NetworkInterfaces + 'f,
    R: CryptoRng + RngCore + 'f,
= impl Future<Output = ()> + 'f;

pub enum DeviceState {
    Unprovisioned { uuid: Uuid, in_progress: bool },
    Provisioned,
}
