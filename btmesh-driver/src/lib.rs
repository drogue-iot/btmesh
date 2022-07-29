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

pub mod fmt;
mod error;
pub mod stack;

pub mod storage;
mod util;

use crate::stack::interface::NetworkInterfaces;
use crate::stack::provisioned::network::DeviceInfo;
use crate::stack::provisioned::secrets::Secrets;
use crate::stack::provisioned::sequence::Sequence;
use crate::stack::provisioned::system::UpperMetadata;
use crate::stack::provisioned::{NetworkState, ProvisionedStack};
use crate::stack::unprovisioned::{ProvisioningState, UnprovisionedStack};
use crate::stack::Stack;
use crate::storage::unprovisioned::UnprovisionedConfiguration;
use crate::storage::{BackingStore, Configuration, Storage};
pub use error::DriverError;

pub trait BluetoothMeshDriver {
    type RunFuture<'f>: Future<Output = Result<(), DriverError>> + 'f
    where
        Self: 'f;

    fn run(&mut self) -> Self::RunFuture<'_>;
}

pub struct Driver<N: NetworkInterfaces, R: RngCore + CryptoRng, B: BackingStore> {
    stack: Stack,
    network: N,
    rng: R,
    storage: Storage<B>,
}

impl<N: NetworkInterfaces, R: RngCore + CryptoRng, B: BackingStore> Driver<N, R, B> {
    pub fn new(network: N, rng: R, backing_store: B, capabilities: Capabilities) -> Self {
        Self {
            stack: Stack::None,
            network,
            rng,
            storage: Storage::new(backing_store, capabilities),
        }
    }

    async fn receive_pdu(&mut self, pdu: &PDU) -> Result<(), DriverError> {
        match (&pdu, &mut self.stack) {
            (PDU::Provisioning(pdu), Stack::Unprovisioned { stack, uuid }) => {
                if let Some(provisioning_state) = stack.process(pdu, &mut self.rng)? {
                    match provisioning_state {
                        ProvisioningState::Failed => {
                            self.stack = Stack::Unprovisioned {
                                stack: UnprovisionedStack::new(self.storage.capabilities()),
                                uuid: *uuid,
                            };
                        }
                        ProvisioningState::Response(pdu) => {
                            self.network.transmit(&PDU::Provisioning(pdu)).await?;
                        }
                        ProvisioningState::Data(device_key, provisioning_data) => {
                            let primary_unicast_addr = provisioning_data.unicast_address;
                            let device_info = DeviceInfo::new(
                                primary_unicast_addr,
                                self.storage.capabilities().number_of_elements,
                            );
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
                // PDU incompatible with stack state or stack not initialized; ignore.
            }
        }
        Ok(())
    }

    async fn send_beacon(&self) -> Result<(), DriverError> {
        match &self.stack {
            Stack::None => {
                // nothing
            }
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

    fn next_beacon(&self) -> BeaconFuture<'_, N, R, B> {
        async move {
            if let Some(next_beacon_deadline) = self.stack.next_beacon_deadline() {
                Timer::at(next_beacon_deadline).await
            } else {
                pending().await
            }
        }
    }
}

impl<N: NetworkInterfaces, R: RngCore + CryptoRng, B: BackingStore> BluetoothMeshDriver
    for Driver<N, R, B>
{
    type RunFuture<'f> = impl Future<Output=Result<(), DriverError>> + 'f
        where
            Self: 'f;

    fn run(&mut self) -> Self::RunFuture<'_> {
        info!("staring up");
        async move {
            loop {
                let config = match self.storage.get().await {
                    Ok(config) => config,
                    Err(_) => {
                        info!("failed to load config");
                        let config = Configuration::Unprovisioned(UnprovisionedConfiguration {
                            uuid: Uuid::new_random(&mut self.rng),
                        });
                        info!("storing provisioning config");
                        self.storage.put(&config).await?;
                        info!("stored provisioning config");
                        config
                    }
                };

                match (&self.stack, config) {
                    (Stack::None, Configuration::Unprovisioned(content))
                    | (Stack::Provisioned { .. }, Configuration::Unprovisioned(content)) => {
                        self.stack = Stack::Unprovisioned {
                            stack: UnprovisionedStack::new(self.storage.capabilities()),
                            uuid: content.uuid(),
                        }
                    }
                    (Stack::None, Configuration::Provisioned(content))
                    | (Stack::Unprovisioned { .. }, Configuration::Provisioned(content)) => {
                        self.stack = Stack::Provisioned {
                            sequence: Sequence::new(Seq::new(content.sequence())),
                            stack: content.into(),
                        }
                    }
                    _ => {
                        // unchanged, don't reconfigure the stack.
                    }
                }

                let device_state = self.stack.device_state();

                if let Some(device_state) = device_state {
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

                    let config: Option<Configuration> = (&self.stack).try_into().ok();
                    if let Some(config) = config {
                        // will conditionally put depending on hash/dirty/sequence changes.
                        self.storage.put(&config).await?;
                    }
                }
            }
        }
    }
}

type BeaconFuture<'f, N, R, B>
where
    N: NetworkInterfaces + 'f,
    R: CryptoRng + RngCore + 'f,
    B: BackingStore + 'f,
= impl Future<Output = ()> + 'f;

pub enum DeviceState {
    Unprovisioned { uuid: Uuid, in_progress: bool },
    Provisioned,
}
