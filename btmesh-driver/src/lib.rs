#![cfg_attr(not(test), no_std)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(associated_type_defaults)]
#![allow(dead_code)]

use btmesh_bearer::beacon::Beacon;
use btmesh_common::{Composition, Seq, Uuid};
use btmesh_device::{BluetoothMeshDevice, ChannelImpl};
use btmesh_pdu::provisioning::Capabilities;
use btmesh_pdu::PDU;
use core::cell::RefCell;
use core::future::{pending, Future};
use embassy::channel::Channel;
use embassy::util::{select, select3, Either, Either3};
use rand_core::{CryptoRng, RngCore};

mod error;
pub mod fmt;
pub mod stack;

mod device;
pub mod storage;
mod util;

use crate::device::DeviceContext;
use crate::stack::interface::{NetworkError, NetworkInterfaces};
use crate::stack::provisioned::network::DeviceInfo;
use crate::stack::provisioned::secrets::Secrets;
use crate::stack::provisioned::sequence::Sequence;
use crate::stack::provisioned::system::UpperMetadata;
use crate::stack::provisioned::{NetworkState, ProvisionedStack};
use crate::stack::unprovisioned::{ProvisioningState, UnprovisionedStack};
use crate::stack::Stack;
use crate::storage::provisioned::ProvisionedConfiguration;
use crate::storage::unprovisioned::UnprovisionedConfiguration;
use crate::storage::{BackingStore, Configuration, Storage};
pub use error::DriverError;

enum DesiredStack {
    Unchanged,
    Unprovisioned(UnprovisionedConfiguration),
    Provisioned(ProvisionedConfiguration),
}

enum CurrentStack<'s> {
    None,
    Unprovisioned(&'s Uuid),
    Provisioned(&'s Sequence),
}

pub trait BluetoothMeshDriver {
    type RunFuture<'f, D>: Future<Output = Result<(), DriverError>> + 'f
    where
        Self: 'f,
        D: BluetoothMeshDevice + 'f;

    fn run<'r, D: BluetoothMeshDevice>(&'r mut self, device: &'r mut D) -> Self::RunFuture<'_, D>;
}

pub struct Driver<N: NetworkInterfaces, R: RngCore + CryptoRng, B: BackingStore> {
    stack: RefCell<Stack>,
    network: N,
    rng: RefCell<R>,
    storage: RefCell<Storage<B>>,
}

impl<N: NetworkInterfaces, R: RngCore + CryptoRng, B: BackingStore> Driver<N, R, B> {
    pub fn new(network: N, rng: R, backing_store: B) -> Self {
        Self {
            stack: RefCell::new(Stack::None),
            network,
            rng: RefCell::new(rng),
            storage: RefCell::new(Storage::new(backing_store)),
        }
    }

    async fn receive_pdu(&self, pdu: &PDU) -> Result<(), DriverError> {
        let mut current_stack = &mut *self.stack.borrow_mut();

        match (&pdu, &mut current_stack) {
            (PDU::Provisioning(pdu), Stack::Unprovisioned { stack, uuid }) => {
                if let Some(provisioning_state) = stack.process(pdu, &mut *self.rng.borrow_mut())? {
                    match provisioning_state {
                        ProvisioningState::Failed => {
                            info!("provisioning: state failed");
                            *current_stack = Stack::Unprovisioned {
                                stack: UnprovisionedStack::new(
                                    self.storage.borrow().capabilities(),
                                ),
                                uuid: *uuid,
                            };
                        }
                        ProvisioningState::Response(pdu) => {
                            info!("provisioning: state response");
                            self.network.transmit(&PDU::Provisioning(pdu)).await?;
                        }
                        ProvisioningState::Data(device_key, provisioning_data, pdu) => {
                            info!("provisioning: state data");
                            let primary_unicast_addr = provisioning_data.unicast_address;
                            let device_info = DeviceInfo::new(
                                primary_unicast_addr,
                                self.storage.borrow().capabilities().number_of_elements,
                            );
                            let secrets = (device_key, provisioning_data).into();
                            let network_state = provisioning_data.into();

                            info!("provisioning: provisioned");
                            self.network.transmit(&PDU::Provisioning(pdu)).await?;
                            *current_stack = Stack::Provisioned {
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
                info!("weird ass combination");
                // PDU incompatible with stack state or stack not initialized; ignore.
            }
        }
        Ok(())
    }

    async fn send_beacon(&self) -> Result<(), DriverError> {
        match &*self.stack.borrow() {
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
            if let Some(next_beacon_deadline) = self.stack.borrow().next_beacon_deadline() {
                next_beacon_deadline.await
            } else {
                pending().await
            }
        }
    }

    fn run_device<'ch, D: BluetoothMeshDevice>(
        device: &'ch mut D,
        _channel: &'ch ChannelImpl,
    ) -> impl Future<Output = Result<(), ()>> + 'ch {
        let receiver = INBOUND.receiver();
        device.run(DeviceContext::new(receiver))
    }

    fn run_network(network: &N) -> impl Future<Output = Result<(), NetworkError>> + '_ {
        network.run()
    }

    async fn run_driver(&self, composition: Composition) -> Result<(), DriverError> {
        info!("btmesh: starting up");
        info!("composition {}", composition);

        let capabilities = Capabilities {
            number_of_elements: composition.number_of_elements(),
            algorithms: Default::default(),
            public_key_type: Default::default(),
            static_oob_type: Default::default(),
            output_oob_size: Default::default(),
            output_oob_action: Default::default(),
            input_oob_size: Default::default(),
            input_oob_action: Default::default(),
        };

        self.storage.borrow_mut().set_capabilities(capabilities);

        loop {
            info!("driver loop");
            let config = match self.storage.borrow().get().await {
                Ok(config) => config,
                Err(_) => {
                    info!("failed to load config");
                    let config = Configuration::Unprovisioned(UnprovisionedConfiguration {
                        uuid: Uuid::new_random(&mut *self.rng.borrow_mut()),
                    });
                    info!("storing provisioning config");
                    self.storage.borrow().put(&config).await?;
                    config
                }
            };

            let mut desired = DesiredStack::Unchanged;

            match (&*self.stack.borrow(), config) {
                (Stack::None, Configuration::Unprovisioned(config))
                | (Stack::Provisioned { .. }, Configuration::Unprovisioned(config)) => {
                    desired = DesiredStack::Unprovisioned(config);
                }
                (Stack::None, Configuration::Provisioned(config))
                | (Stack::Unprovisioned { .. }, Configuration::Provisioned(config)) => {
                    desired = DesiredStack::Provisioned(config);
                }
                _ => {
                    // unchanged, don't reconfigure the stack.
                }
            }

            match desired {
                DesiredStack::Unchanged => { /*nothing*/ }
                DesiredStack::Unprovisioned(config) => {
                    info!("setting up unprovisioned stack");
                    *self.stack.borrow_mut() = Stack::Unprovisioned {
                        stack: UnprovisionedStack::new(self.storage.borrow().capabilities()),
                        uuid: config.uuid(),
                    }
                }
                DesiredStack::Provisioned(config) => {
                    info!("setting up provisioned stack");
                    *self.stack.borrow_mut() = Stack::Provisioned {
                        sequence: Sequence::new(Seq::new(config.sequence())),
                        stack: config.into(),
                    }
                }
            }

            let device_state = self.stack.borrow().device_state();

            if let Some(device_state) = device_state {
                let receive_fut = self.network.receive(&device_state);
                let beacon_fut = self.next_beacon();

                match select(receive_fut, beacon_fut).await {
                    Either::First(Ok(pdu)) => {
                        info!("receive_pdu!");
                        self.receive_pdu(&pdu).await?;
                    }
                    Either::First(Err(err)) => {
                        info!("receive_pdu error!");
                        return Err(err.into());
                    }
                    Either::Second(_) => {
                        info!("send beacon!");
                        self.send_beacon().await?;
                    }
                }

                let config: Option<Configuration> = (&*self.stack.borrow()).try_into().ok();
                if let Some(config) = config {
                    // will conditionally put depending on hash/dirty/sequence changes.
                    self.storage.borrow().put(&config).await?;
                }
            }
        }
        info!("driver loop done");
    }
}

impl<N: NetworkInterfaces, R: RngCore + CryptoRng, B: BackingStore> BluetoothMeshDriver
    for Driver<N, R, B>
{
    type RunFuture<'f, D> = impl Future<Output=Result<(), DriverError>> + 'f
        where
            Self: 'f,
            D: BluetoothMeshDevice + 'f;

    fn run<'r, D: BluetoothMeshDevice>(&'r mut self, device: &'r mut D) -> Self::RunFuture<'r, D> {
        let composition = device.composition();

        info!("run!");

        async move {
            let channel = Channel::new();
            let device_fut = Self::run_device(device, &channel);
            let driver_fut = self.run_driver(composition);
            let network_fut = Self::run_network(&self.network);

            // if the device or the driver is `Ready` then stuff is just done, stop.
            match select3(driver_fut, device_fut, network_fut).await {
                Either3::First(_) => {
                    info!("driver done");
                }
                Either3::Second(_val) => {
                    info!("device done");
                }
                Either3::Third(_val) => {
                    info!("network done");
                }
            }

            info!("run ended!");
            Ok(())
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

static INBOUND: ChannelImpl = ChannelImpl::new();
