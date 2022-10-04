#![cfg_attr(not(test), no_std)]
#![feature(type_alias_impl_trait)]
#![feature(associated_type_defaults)]
#![allow(dead_code)]
#![allow(clippy::await_holding_refcell_ref)]
#![feature(async_closure)]

use btmesh_bearer::beacon::Beacon;
use btmesh_common::address::{Address, UnicastAddress};
use btmesh_common::{Composition, Seq, Ttl, Uuid};
use btmesh_device::{
    BluetoothMeshDevice, CompletionToken, CompositionExtra, InboundChannel, InboundChannelReceiver,
    KeyHandle, OutboundChannel, OutboundExtra, OutboundPayload, PublicationCadence, SendExtra,
};
use btmesh_models::foundation::configuration::model_publication::PublishAddress;
use btmesh_models::foundation::configuration::CONFIGURATION_SERVER;
use btmesh_pdu::provisioned::access::AccessMessage;
use btmesh_pdu::provisioned::network::NetworkPDU;
use btmesh_pdu::provisioned::Message;
use btmesh_pdu::provisioning::generic::Reason;
use btmesh_pdu::provisioning::{Capabilities, ProvisioningPDU};
use btmesh_pdu::PDU;
use core::cell::RefCell;
use core::future::{pending, Future};
use embassy_futures::select::{select, select3, select4, Either, Either3, Either4};
use embassy_time::{Duration, Instant, Timer};
use heapless::Vec;
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
use crate::stack::provisioned::system::{AccessMetadata, UpperMetadata};
use crate::stack::provisioned::{NetworkState, ProvisionedStack};
use crate::stack::unprovisioned::{ProvisioningState, UnprovisionedStack};
use crate::stack::Stack;
use crate::storage::provisioned::ProvisionedConfiguration;
use crate::storage::unprovisioned::UnprovisionedConfiguration;
use crate::storage::{BackingStore, Configuration, Storage};
use crate::util::hash::hash_of;
use crate::watchdog::{Watchdog, WatchdogEvent};
pub use error::DriverError;

#[derive(Default)]
pub struct BluetoothMeshDriverConfig {
    pub persist_interval: Option<Duration>,
    pub uuid: Option<Uuid>,
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
    persist_interval: Option<Duration>,
}

impl<N: NetworkInterfaces, R: RngCore + CryptoRng, B: BackingStore> Driver<N, R, B> {
    pub fn new(
        network: N,
        mut rng: R,
        backing_store: B,
        config: BluetoothMeshDriverConfig,
    ) -> Self {
        let upc = UnprovisionedConfiguration::new(
            config.uuid.unwrap_or_else(|| Uuid::new_random(&mut rng)),
        );
        Self {
            network: Some(network),
            rng: Some(rng),
            storage: Storage::new(backing_store, upc),
            persist_interval: config.persist_interval,
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
    persist_interval: Option<Duration>,
}

impl<'s, N: NetworkInterfaces, R: RngCore + CryptoRng, B: BackingStore> InnerDriver<'s, N, R, B> {
    pub fn new(
        network: N,
        rng: R,
        storage: &'s Storage<B>,
        persist_interval: Option<Duration>,
    ) -> Self {
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
            persist_interval,
        }
    }

    async fn receive_provisioning_pdu(
        &self,
        pdu: &ProvisioningPDU,
        stack: &mut UnprovisionedStack,
    ) -> Result<(), DriverError> {
        if let Some(provisioning_state) = stack.process(pdu, &mut *self.rng.borrow_mut())? {
            match provisioning_state {
                ProvisioningState::Failed => {
                    warn!("provisioning failed");
                    *stack = UnprovisionedStack::new(self.storage.capabilities());
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
                    self.storage.provision(provisioned_config).await?;
                }
            }
        }

        Ok(())
    }

    async fn receive_network_pdu(
        &self,
        pdu: &NetworkPDU,
        stack: &mut ProvisionedStack,
        sequence: &Sequence,
        is_loopback: bool,
    ) -> Result<(), DriverError> {
        let (relay_pdu, block_ack_pdu, result) = self
            .storage
            .read_provisioned(|config| {
                let result = match stack.process_inbound_network_pdu(
                    config.secrets(),
                    pdu,
                    &self.watchdog,
                    is_loopback,
                    config.subscriptions(),
                ) {
                    Ok((relay_pdu, Some(result))) => {
                        if let Some((block_ack, meta)) = &result.block_ack {
                            // send outbound block-ack
                            if let Address::Unicast(addr) = result.dst {
                                let block_ack_pdu = stack.process_outbound_block_ack(
                                    config.secrets(),
                                    sequence,
                                    *block_ack,
                                    meta,
                                    &addr,
                                )?;
                                (relay_pdu, block_ack_pdu, Some(result))
                            } else {
                                (relay_pdu, None, Some(result))
                            }
                        } else {
                            (relay_pdu, None, Some(result))
                        }
                    }
                    Ok((relay_pdu, None)) => (relay_pdu, None, None),
                    Err(DriverError::InvalidPDU) => {
                        debug!("invalid PDU (ignored)");
                        (None, None, None)
                    }
                    Err(err) => {
                        warn!("error (ignored) processing inbound pdu: {}", err);
                        (None, None, None)
                    }
                };
                Ok(result)
            })
            .await?;

        if let Some(network_pdu) = block_ack_pdu {
            self.network
                .transmit(&(network_pdu.into()), false)
                .await
                .ok();
        }

        #[cfg(feature = "relay")]
        if let Some(relay_pdu) = relay_pdu {
            let relay_pdu = self
                .storage
                .read_provisioned(|config| {
                    stack.process_outbound_relay_network_pdu(config.secrets(), &relay_pdu)
                })
                .await;

            if let Ok(Some(relay_pdu)) = relay_pdu {
                self.network.transmit(&(relay_pdu.into()), false).await.ok();
            }
        }

        if let Some(result) = result {
            if let Some(message) = &result.message {
                // dispatch to element(s)
                let subscriptions = self
                    .storage
                    .read_provisioned(|config| Ok(config.subscriptions().clone()))
                    .await?;
                match message {
                    Message::Access(message) => {
                        self.dispatcher
                            .borrow_mut()
                            .dispatch(message, &subscriptions)
                            .await?;
                    }
                    Message::Control(message) => {
                        stack.process_inbound_control(message, &self.watchdog)?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn receive_pdu(&self, pdu: &PDU) -> Result<(), DriverError> {
        let mut current_stack = &mut *self.stack.borrow_mut();

        match (&pdu, &mut current_stack) {
            (PDU::Provisioning(pdu), Stack::Unprovisioned { stack, .. }) => {
                debug!("inbound provisioning pdu: {}", pdu);
                self.receive_provisioning_pdu(pdu, stack).await?;
            }
            (PDU::Network(pdu), Stack::Provisioned { stack, sequence }) => {
                self.receive_network_pdu(pdu, stack, sequence, false)
                    .await?;
            }
            _ => {
                // PDU incompatible with stack state or stack not initialized; ignore.
            }
        }
        Ok(())
    }

    fn process_outbound_send(
        &self,
        element_address: UnicastAddress,
        default_ttl: Ttl,
        outbound_payload: &OutboundPayload,
        extra: &SendExtra,
    ) -> Result<
        (
            Option<AccessMessage<ProvisionedStack>>,
            Option<CompletionToken>,
        ),
        DriverError,
    > {
        Ok((
            Some(AccessMessage::new(
                outbound_payload.opcode,
                Vec::from_slice(&outbound_payload.parameters)?,
                (element_address, extra.meta, default_ttl),
            )),
            extra.completion_token.clone(),
        ))
    }

    fn process_outbound_publish(
        &self,
        config: &ProvisionedConfiguration,
        element_address: UnicastAddress,
        default_ttl: Ttl,
        outbound_payload: &OutboundPayload,
    ) -> Result<
        (
            Option<AccessMessage<ProvisionedStack>>,
            Option<CompletionToken>,
        ),
        DriverError,
    > {
        if let Some(publication) = config.publications().get(
            outbound_payload.element_index as u8,
            outbound_payload.model_identifer,
        ) {
            let (dst, label_uuid): (Address, _) = match publication.details.publish_address {
                PublishAddress::Unicast(addr) => (addr.into(), None),
                PublishAddress::Group(addr) => (addr.into(), None),
                PublishAddress::Label(addr) => (addr.virtual_address().into(), Some(addr)),
                PublishAddress::Virtual(addr) => (addr.into(), None),
                PublishAddress::Unassigned => unreachable!(),
            };

            if let Some((network_key_handle, app_key_handle)) = config
                .secrets()
                .get_key_pair(publication.details.app_key_index)
            {
                let meta = AccessMetadata {
                    network_key_handle,
                    iv_index: config.iv_index(),
                    local_element_index: Some(outbound_payload.element_index as u8),
                    key_handle: KeyHandle::Application(app_key_handle),
                    src: element_address,
                    dst,
                    ttl: publication.details.publish_ttl.unwrap_or(default_ttl),
                    label_uuid,
                    replay_seq: None,
                };
                Ok((
                    Some(AccessMessage::<ProvisionedStack>::new(
                        outbound_payload.opcode,
                        Vec::from_slice(&outbound_payload.parameters.clone())?,
                        meta,
                    )),
                    None,
                ))
            } else {
                Ok((None, None))
            }
        } else {
            Ok((None, None))
        }
    }

    async fn process_outbound_payload(
        &self,
        outbound_payload: &OutboundPayload,
    ) -> Result<(), DriverError> {
        let locked_config = self.storage.lock().await;

        if let Some(Configuration::Provisioned(config)) = &*locked_config {
            let element_address = config
                .device_info()
                .local_element_address(outbound_payload.element_index as u8)
                .ok_or(DriverError::InvalidState)?;
            let default_ttl = config.foundation().configuration().default_ttl();
            let (message, completion_token, retransmits) = match &outbound_payload.extra {
                OutboundExtra::Send(extra) => {
                    let (message, completion_token) = self.process_outbound_send(
                        element_address,
                        default_ttl,
                        outbound_payload,
                        extra,
                    )?;
                    (message, completion_token, 3)
                }
                OutboundExtra::Publish => {
                    let retransmits = config
                        .publications()
                        .get(
                            outbound_payload.element_index as u8,
                            outbound_payload.model_identifer,
                        )
                        .map(|p| p.details.publish_retransmit.count())
                        .unwrap_or(3);
                    let (message, completion_token) = self.process_outbound_publish(
                        config,
                        element_address,
                        default_ttl,
                        outbound_payload,
                    )?;
                    (message, completion_token, retransmits)
                }
            };

            if let (Some(message), Stack::Provisioned { stack, sequence }) =
                (message, &mut *self.stack.borrow_mut())
            {
                let message = message.into();
                let pdus = stack.process_outbound(
                    config.secrets(),
                    sequence,
                    &message,
                    completion_token,
                    &self.watchdog,
                    retransmits,
                )?;

                drop(locked_config);
                for pdu in pdus {
                    self.receive_network_pdu(&pdu, stack, sequence, true)
                        .await?;
                    let pdu = pdu.into();
                    self.network.transmit(&pdu, false).await?;
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
                let pdus = self
                    .storage
                    .read_provisioned(|config| stack.retransmit(config.secrets(), sequence))
                    .await?;

                for pdu in pdus {
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

            Stack::Provisioned { .. } => {
                let network_id = self
                    .storage
                    .read_provisioned(|config| {
                        Ok(config.secrets().network_key_by_index(0)?.network_id())
                    })
                    .await?;
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
        receiver: InboundChannelReceiver,
    ) -> impl Future<Output = Result<(), ()>> + '_ {
        device.run(DeviceContext::new(receiver, OUTBOUND.sender()))
    }

    fn run_network(network: &N) -> impl Future<Output = Result<(), NetworkError>> + '_ {
        network.run()
    }

    async fn notify_publications(
        &self,
        config: &Option<Configuration>,
        composition: &mut Composition<CompositionExtra>,
    ) {
        if let Some(Configuration::Provisioned(config)) = config {
            for (element_index, element) in composition.elements_iter_mut().enumerate() {
                for model_descriptor in element.models_iter_mut() {
                    if let Some(publication) = config
                        .publications()
                        .get(element_index as u8, model_descriptor.model_identifier)
                    {
                        let pub_cadence =
                            PublicationCadence::from(publication.details.publish_period);

                        if model_descriptor.extra.publication_cadence != pub_cadence {
                            self.dispatcher
                                .borrow()
                                .dispatch_publish(
                                    element_index as u8,
                                    model_descriptor.model_identifier,
                                    pub_cadence,
                                )
                                .await;
                            model_descriptor.extra.publication_cadence = pub_cadence;
                        }
                    } else if model_descriptor.extra.publication_cadence != PublicationCadence::None
                    {
                        self.dispatcher
                            .borrow()
                            .dispatch_publish(
                                element_index as u8,
                                model_descriptor.model_identifier,
                                PublicationCadence::None,
                            )
                            .await;
                        model_descriptor.extra.publication_cadence = PublicationCadence::None;
                    }
                }
            }
        } else {
            // Stop all publications
            for (element_index, element) in composition.elements_iter_mut().enumerate() {
                for model_descriptor in element.models_iter_mut() {
                    if model_descriptor.extra.publication_cadence != PublicationCadence::None {
                        self.dispatcher
                            .borrow()
                            .dispatch_publish(
                                element_index as u8,
                                model_descriptor.model_identifier,
                                PublicationCadence::None,
                            )
                            .await;
                        model_descriptor.extra.publication_cadence = PublicationCadence::None;
                    }
                }
            }
        }
    }

    fn display_configuration(
        composition: &Composition,
        config: &Configuration,
        last_displayed_hash: Option<u64>,
    ) -> u64 {
        let current_hash = hash_of(config);

        let changed = match last_displayed_hash {
            Some(previous_hash) => current_hash != previous_hash,
            None => true,
        };

        if changed {
            config.display(composition);
        }
        current_hash
    }

    fn reconfigure_stack(&self, config: &Configuration) {
        let mut stack = self.stack.borrow_mut();

        match (&*stack, config) {
            (Stack::None, Configuration::Unprovisioned(config))
            | (Stack::Provisioned { .. }, Configuration::Unprovisioned(config)) => {
                *stack = Stack::Unprovisioned {
                    stack: UnprovisionedStack::new(self.storage.capabilities()),
                    uuid: config.uuid,
                };
                self.network.reset();
            }
            (Stack::None, Configuration::Provisioned(config))
            | (Stack::Unprovisioned { .. }, Configuration::Provisioned(config)) => {
                *stack = Stack::Provisioned {
                    sequence: Sequence::new(Seq::new(config.sequence())),
                    stack: config.into(),
                };
            }
            _ => {
                // unchanged, don't reconfigure the stack.
            }
        }
    }

    async fn update_config(&self) -> Result<(), DriverError> {
        let stack = self.stack.borrow_mut();
        match &*stack {
            Stack::Provisioned { sequence, .. } => {
                self.storage
                    .modify_provisioned(|config| {
                        *config.sequence_mut() = sequence.current();
                        debug!("Updating config sequence counter to {}", sequence.current());
                        Ok(())
                    })
                    .await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn run_driver(
        &self,
        composition: &mut Composition<CompositionExtra>,
    ) -> Result<(), DriverError> {
        info!("btmesh: starting up");

        let capabilities = Capabilities {
            number_of_elements: composition.number_of_elements(),
            ..Default::default()
        };

        enhance_composition(composition)?;

        let simplified_composition = composition.simplify();

        self.storage.set_composition(simplified_composition.clone());
        self.storage.set_capabilities(capabilities);

        self.storage.init().await?;

        let mut last_displayed_hash = None;
        let mut last_update = Instant::now();

        loop {
            if let Some(persist_interval) = self.persist_interval {
                let now = Instant::now();
                if last_update + persist_interval < now {
                    self.update_config().await?;
                    last_update = now;
                }
            }

            let config = self.storage.lock().await;
            if config.is_none() {
                return Err(DriverError::InvalidState);
            }

            if let Some(config) = &*config {
                self.reconfigure_stack(config);
                last_displayed_hash.replace(Self::display_configuration(
                    &simplified_composition,
                    config,
                    last_displayed_hash,
                ));
            }

            let device_state = self.stack.borrow().device_state();
            self.notify_publications(&config, composition).await;
            drop(config);

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
                            if !self.stack.borrow().has_ongoing_completion() {
                                if let Err(result) = self.receive_pdu(&pdu).await {
                                    match result {
                                        DriverError::InvalidPDU | DriverError::Parse(_) => continue,
                                        _ => return Err(result),
                                    }
                                }
                            }
                        }
                        Either::First(Err(err)) => {
                            return Err(err.into());
                        }
                        Either::Second(outbound_payload) => {
                            if let DeviceState::Provisioned = device_state {
                                self.process_outbound_payload(&outbound_payload).await?;
                            }
                        }
                    },
                    Either4::Second(_) => {
                        self.send_beacon().await.ok();
                    }
                    Either4::Third(_) => {
                        self.retransmit().await.ok();
                    }
                    Either4::Fourth(Some(expiration)) => {
                        self.handle_watchdog_event(&expiration.take()).await.ok();
                    }
                    Either4::Fourth(None) => {
                        // nothing?
                    }
                }
            }
        }
    }

    async fn handle_watchdog_event(&self, event: &WatchdogEvent) -> Result<(), DriverError> {
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
                    let network_pdu = self
                        .storage
                        .read_provisioned(|config| {
                            if let Some(src) = config.device_info().local_element_address(0) {
                                Ok(stack.inbound_expiration(
                                    config.secrets(),
                                    sequence,
                                    seq_zero,
                                    &src,
                                    &self.watchdog,
                                )?)
                            } else {
                                Ok(None)
                            }
                        })
                        .await?;

                    if let Some(network_pdu) = network_pdu {
                        self.network.transmit(&network_pdu.into(), false).await.ok();
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
        loop {
            let mut composition = device.composition();

            let mut foundation_device = FoundationDevice::new(self.storage);

            let network_fut = Self::run_network(&self.network);
            let device_fut = select(
                Self::run_device(&mut foundation_device, FOUNDATION_INBOUND.receiver()),
                Self::run_device(device, DEVICE_INBOUND.receiver()),
            );
            let driver_fut = self.run_driver(&mut composition);

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
        }
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
                self.persist_interval,
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

static FOUNDATION_INBOUND: InboundChannel = InboundChannel::new();
static DEVICE_INBOUND: InboundChannel = InboundChannel::new();

static OUTBOUND: OutboundChannel = OutboundChannel::new();

fn enhance_composition<X: Default>(composition: &mut Composition<X>) -> Result<(), DriverError> {
    if composition.number_of_elements() > 0 {
        composition[0].add_model(CONFIGURATION_SERVER);
    }

    Ok(())
}
