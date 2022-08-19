#![cfg_attr(not(test), no_std)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(associated_type_defaults)]
#![allow(dead_code)]
#![allow(clippy::await_holding_refcell_ref)]

use btmesh_bearer::beacon::Beacon;
use btmesh_common::{Composition, Seq, Uuid};
use btmesh_device::{
    BluetoothMeshDevice, InboundChannelImpl, InboundReceiverImpl, OutboundChannelImpl,
    OutboundPayload,
};
use btmesh_models::foundation::configuration::CONFIGURATION_SERVER;
use btmesh_pdu::provisioned::access::AccessMessage;
use btmesh_pdu::provisioned::Message;
use btmesh_pdu::provisioning::generic::Reason;
use btmesh_pdu::provisioning::Capabilities;
use btmesh_pdu::PDU;
use core::cell::RefCell;
use core::future::{pending, Future};
use embassy_time::{Duration, Timer};
use embassy_util::{select, select3, select4, Either, Either3, Either4};
use rand_core::{CryptoRng, RngCore};

mod error;
pub mod fmt;
pub mod interface;
pub mod stack;

mod device;
pub(crate) mod dispatch;
mod models;
pub mod storage;
mod util;
mod watchdog;

use crate::device::DeviceContext;
use crate::dispatch::Dispatcher;
use crate::interface::{NetworkError, NetworkInterfaces};
use crate::models::FoundationDevice;
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
use crate::watchdog::{Watchdog, WatchdogEvent};
pub use error::DriverError;

#[allow(clippy::large_enum_variant)]
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
    network: Option<N>,
    rng: Option<R>,
    storage: Storage<B>,
}

impl<N: NetworkInterfaces, R: RngCore + CryptoRng, B: BackingStore> Driver<N, R, B> {
    pub fn new(network: N, mut rng: R, backing_store: B) -> Self {
        let upc = UnprovisionedConfiguration::new(&mut rng);
        Self {
            network: Some(network),
            rng: Some(rng),
            storage: Storage::new(backing_store, upc),
        }
    }
}

pub struct InnerDriver<'s, N: NetworkInterfaces, R: RngCore + CryptoRng, B: BackingStore + 's> {
    stack: RefCell<Stack>,
    network: N,
    rng: RefCell<R>,
    storage: &'s Storage<B>,
    dispatcher: RefCell<Dispatcher>,
    watchdog: Watchdog,
}

impl<'s, N: NetworkInterfaces, R: RngCore + CryptoRng, B: BackingStore> InnerDriver<'s, N, R, B> {
    pub fn new(network: N, rng: R, storage: &'s Storage<B>) -> Self {
        Self {
            stack: RefCell::new(Stack::None),
            network,
            rng: RefCell::new(rng),
            storage,
            dispatcher: RefCell::new(Dispatcher::new(
                FOUNDATION_INBOUND.sender(),
                DEVICE_INBOUND.sender(),
            )),
            watchdog: Default::default(),
        }
    }

    async fn receive_pdu(&self, pdu: &PDU) -> Result<(), DriverError> {
        let mut current_stack = &mut *self.stack.borrow_mut();

        match (&pdu, &mut current_stack) {
            (PDU::Provisioning(pdu), Stack::Unprovisioned { stack, uuid }) => {
                debug!("inbound provisioning pdu: {}", pdu);
                if let Some(provisioning_state) = stack.process(pdu, &mut *self.rng.borrow_mut())? {
                    match provisioning_state {
                        ProvisioningState::Failed => {
                            warn!("provisioning failed");
                            *current_stack = Stack::Unprovisioned {
                                stack: UnprovisionedStack::new(self.storage.capabilities()),
                                uuid: *uuid,
                            };
                        }
                        ProvisioningState::Response(pdu) => {
                            debug!("outbound provisioning pdu: {}", pdu);
                            self.network.transmit(&(pdu.into()), false).await?;
                        }
                        ProvisioningState::Data(device_key, provisioning_data, pdu) => {
                            debug!("received provisioning data: {}", provisioning_data);
                            let primary_unicast_addr = provisioning_data.unicast_address;
                            let device_info = DeviceInfo::new(
                                primary_unicast_addr,
                                self.storage.capabilities().number_of_elements,
                            );
                            let secrets = (device_key, provisioning_data).into();
                            let network_state = provisioning_data.into();

                            let pdu = pdu.into();
                            debug!("sending provisioning complete response");
                            for retransmit in 0..5 {
                                self.network.transmit(&pdu, retransmit != 0).await?;
                                Timer::after(Duration::from_millis(100)).await;
                            }
                            debug!("adjusting into fully provisioned state");

                            let provisioned_config: ProvisionedConfiguration =
                                (device_info, secrets, network_state).into();
                            self.storage
                                .put(&Configuration::Provisioned(provisioned_config))
                                .await?;
                        }
                    }
                }
            }
            (PDU::Network(pdu), Stack::Provisioned { stack, sequence }) => {
                debug!("inbound network pdu: {}", pdu);
                if let Some(result) = stack.process_inbound_network_pdu(pdu, &self.watchdog)? {
                    if let Some((block_ack, meta)) = result.block_ack {
                        debug!("we have outbound block_ack");
                        // send outbound block-ack
                        if let Configuration::Provisioned(config) = self.storage.get().await? {
                            if let Some(src) = config.device_info().local_element_address(0) {
                                for network_pdu in stack
                                    .process_outbound_block_ack(sequence, block_ack, meta, src)?
                                {
                                    debug!("outbound network block-ack pdu: {}", pdu);
                                    // don't error if we can't send.
                                    self.network
                                        .transmit(&PDU::Network(network_pdu), false)
                                        .await
                                        .ok();
                                }
                            }
                        }
                    }

                    if let Some(message) = result.message {
                        // dispatch to element(s)
                        match message {
                            Message::Access(message) => {
                                self.dispatcher.borrow_mut().dispatch(message).await?;
                            }
                            Message::Control(message) => {
                                stack.process_inbound_control(&message, &self.watchdog)?;
                            }
                        }
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

    async fn process_outbound_payload(
        &self,
        outbound_payload: OutboundPayload,
    ) -> Result<(), DriverError> {
        let config = self.storage.get().await?;
        if let Configuration::Provisioned(config) = config {
            let element_address = config
                .device_info()
                .local_element_address(outbound_payload.0 .0 as u8)
                .ok_or(DriverError::InvalidState)?;
            let default_ttl = config.foundation().configuration().default_ttl();
            let message: AccessMessage<ProvisionedStack> = AccessMessage::new(
                outbound_payload.1,
                outbound_payload.2,
                (element_address, outbound_payload.3, default_ttl),
            );

            if let Stack::Provisioned { stack, sequence } = &mut *self.stack.borrow_mut() {
                let network_pdus = stack.process_outbound(
                    sequence,
                    &(message.into()),
                    outbound_payload.4,
                    &self.watchdog,
                );
                for pdu in network_pdus? {
                    //debug!("outbound network pdu: {}", pdu);
                    self.network.transmit(&(pdu.into()), false).await?;
                }
            }
        }

        Ok(())
    }

    async fn retransmit(&self) -> Result<(), DriverError> {
        match &mut *self.stack.borrow_mut() {
            Stack::None => {}
            Stack::Unprovisioned { stack, .. } => {
                if let Some(pdu) = stack.retransmit() {
                    self.network.transmit(&(pdu.into()), true).await?;
                }
            }
            Stack::Provisioned { stack, sequence } => {
                for pdu in stack.retransmit(sequence)? {
                    self.network.transmit(&(pdu.into()), true).await?;
                }
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

    fn next_retransmit(&self) -> RetransmitFuture<'_, N, R, B> {
        async move {
            if let Some(next_retransmit) = self.stack.borrow().next_retransmit() {
                next_retransmit.await
            } else {
                pending().await
            }
        }
    }

    fn run_device<D: BluetoothMeshDevice>(
        device: &mut D,
        receiver: InboundReceiverImpl,
    ) -> impl Future<Output = Result<(), ()>> + '_ {
        device.run(DeviceContext::new(receiver, OUTBOUND.sender()))
    }

    fn run_network(network: &N) -> impl Future<Output = Result<(), NetworkError>> + '_ {
        network.run()
    }

    async fn run_driver(&self, composition: Composition) -> Result<(), DriverError> {
        info!("btmesh: starting up");

        let capabilities = Capabilities {
            number_of_elements: composition.number_of_elements(),
            ..Default::default()
        };

        let composition = enhance_composition(composition)?;

        self.storage.set_composition(composition.clone());
        self.storage.set_capabilities(capabilities);

        self.storage.init().await?;

        loop {
            let config = self.storage.get().await?;

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
                    *self.stack.borrow_mut() = Stack::Unprovisioned {
                        stack: UnprovisionedStack::new(self.storage.capabilities()),
                        uuid: config.uuid,
                    };
                    self.network.reset();
                    config.display(&composition);
                }
                DesiredStack::Provisioned(config) => {
                    *self.stack.borrow_mut() = Stack::Provisioned {
                        sequence: Sequence::new(Seq::new(config.sequence())),
                        stack: (&config).into(),
                    };
                    config.display(&composition);
                }
            }

            let device_state = self.stack.borrow().device_state();

            if let Some(device_state) = device_state {
                let receive_fut = self.network.receive(&device_state, &self.watchdog);
                let transmit_fut = OUTBOUND.recv();
                let io_fut = select(receive_fut, transmit_fut);

                let beacon_fut = self.next_beacon();
                let retransmit_fut = self.next_retransmit();

                let watchdog_fut = self.watchdog.next();

                match select4(io_fut, beacon_fut, retransmit_fut, watchdog_fut).await {
                    Either4::First(inner) => match inner {
                        Either::First(Ok(pdu)) => {
                            if let Err(result) = self.receive_pdu(&pdu).await {
                                match result {
                                    DriverError::InvalidPDU | DriverError::Parse(_) => continue,
                                    _ => return Err(result),
                                }
                            }
                        }
                        Either::First(Err(err)) => {
                            return Err(err.into());
                        }
                        Either::Second(outbound_payload) => {
                            self.process_outbound_payload(outbound_payload).await?;
                        }
                    },
                    Either4::Second(_) => {
                        self.send_beacon().await?;
                    }
                    Either4::Third(_) => {
                        self.retransmit().await?;
                    }
                    Either4::Fourth(Some(expiration)) => {
                        self.handle_watchdog_event(expiration.take()).await?;
                    }
                    Either4::Fourth(None) => {
                        // nothing?
                    }
                }
            }
        }
    }

    async fn handle_watchdog_event(&self, event: WatchdogEvent) -> Result<(), DriverError> {
        match event {
            WatchdogEvent::LinkOpenTimeout => {
                self.network.close_link(Reason::Timeout).await?;
                *self.stack.borrow_mut() = Stack::None;
            }
            WatchdogEvent::OutboundExpiration(seq_zero) => {
                if let Stack::Provisioned { stack, .. } = &mut *self.stack.borrow_mut() {
                    stack.outbound_expiration(seq_zero);
                }
            }
            WatchdogEvent::InboundExpiration(seq_zero) => {
                if let Stack::Provisioned {
                    stack, sequence, ..
                } = &mut *self.stack.borrow_mut()
                {
                    if let Configuration::Provisioned(config) = self.storage.get().await? {
                        if let Some(src) = config.device_info().local_element_address(0) {
                            for network_pdu in
                                stack.inbound_expiration(sequence, seq_zero, src, &self.watchdog)?
                            {
                                self.network.transmit(&network_pdu.into(), false).await.ok();
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn run<'r, D: BluetoothMeshDevice>(
        &'r mut self,
        device: &'r mut D,
    ) -> Result<(), DriverError> {
        let composition = device.composition();

        let mut foundation_device = FoundationDevice::new(self.storage);

        let network_fut = Self::run_network(&self.network);
        let device_fut = select(
            Self::run_device(&mut foundation_device, FOUNDATION_INBOUND.receiver()),
            Self::run_device(device, DEVICE_INBOUND.receiver()),
        );
        let driver_fut = self.run_driver(composition);

        // if the device or the driver is `Ready` then stuff is just done, stop.
        match select3(network_fut, driver_fut, device_fut).await {
            Either3::First(_val) => {
                info!("************** network exited");
            }
            Either3::Second(Ok(_)) => {
                info!("************** driver exited");
            }
            Either3::Second(Err(err)) => {
                info!("************** driver exited with error {}", err);
            }
            Either3::Third(_val) => {
                info!("************** device exited");
            }
        }

        info!("run ended!");
        Ok(())
    }
}

impl<N: NetworkInterfaces, R: RngCore + CryptoRng, B: BackingStore> BluetoothMeshDriver
    for Driver<N, R, B>
{
    type RunFuture<'f, D> = impl Future<Output=Result<(), DriverError>> + 'f
    where
    Self: 'f,
    D: BluetoothMeshDevice + 'f;

    fn run<'r, D: BluetoothMeshDevice>(&'r mut self, device: &'r mut D) -> Self::RunFuture<'_, D> {
        async move {
            InnerDriver::new(
                unwrap!(self.network.take()),
                unwrap!(self.rng.take()),
                &self.storage,
            )
            .run(device)
            .await
        }
    }
}

type BeaconFuture<'f, N, R, B>
where
    N: NetworkInterfaces + 'f,
    R: CryptoRng + RngCore + 'f,
    B: BackingStore + 'f,
= impl Future<Output = ()> + 'f;

type RetransmitFuture<'f, N, R, B>
where
    N: NetworkInterfaces + 'f,
    R: CryptoRng + RngCore + 'f,
    B: BackingStore + 'f,
= impl Future<Output = ()> + 'f;

pub enum DeviceState {
    Unprovisioned { uuid: Uuid, in_progress: bool },
    Provisioned,
}

static FOUNDATION_INBOUND: InboundChannelImpl = InboundChannelImpl::new();
static DEVICE_INBOUND: InboundChannelImpl = InboundChannelImpl::new();

static OUTBOUND: OutboundChannelImpl = OutboundChannelImpl::new();

fn enhance_composition(composition: Composition) -> Result<Composition, DriverError> {
    let mut enhanced = Composition::new(composition.cid(), composition.pid(), composition.vid());

    for (i, element) in composition.elements_iter().enumerate() {
        let mut element = element.clone();
        if i == 0 {
            element.add_model(CONFIGURATION_SERVER);
        }
        enhanced
            .add_element(element)
            .map_err(|_| DriverError::InsufficientSpace)?;
    }
    Ok(enhanced)
}
