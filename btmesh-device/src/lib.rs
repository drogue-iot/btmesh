#![cfg_attr(not(test), no_std)]
#![feature(type_alias_impl_trait)]
#![feature(associated_type_defaults)]
#![allow(dead_code)]

pub mod access_counted;

use crate::access_counted::AccessCountedHandle;
use btmesh_common::address::{Address, LabelUuid, UnicastAddress};
use btmesh_common::crypto::application::Aid;
use btmesh_common::crypto::network::Nid;
pub use btmesh_common::location;
use btmesh_common::opcode::Opcode;
pub use btmesh_common::ElementDescriptor;
pub use btmesh_common::{
    CompanyIdentifier, Composition, Features, InsufficientBuffer, ModelIdentifier,
    ProductIdentifier, VersionIdentifier,
};
use btmesh_common::{IvIndex, ParseError, Ttl};
use btmesh_models::foundation::configuration::model_publication::{PublishPeriod, Resolution};
use btmesh_models::foundation::configuration::{AppKeyIndex, NetKeyIndex};
pub use btmesh_models::Model;
use core::future::Future;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
pub use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::Duration;
pub use futures::future::join;
pub use futures::future::select;
pub use futures::future::Either;
pub use futures::pin_mut;
use heapless::Vec;

pub type Signal<T> =
    embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, T>;

pub type InboundChannel =
    Channel<CriticalSectionRawMutex, AccessCountedHandle<'static, InboundPayload>, 1>;
pub type InboundChannelSender =
    Sender<'static, CriticalSectionRawMutex, AccessCountedHandle<'static, InboundPayload>, 1>;
pub type InboundChannelReceiver =
    Receiver<'static, CriticalSectionRawMutex, AccessCountedHandle<'static, InboundPayload>, 1>;

pub struct InboundPayload {
    pub element_index: usize,
    pub model_identifier: Option<ModelIdentifier>,
    pub body: InboundBody,
}

#[allow(clippy::large_enum_variant)]
pub enum InboundBody {
    Control(Control),
    Message(InboundMessage),
}

#[derive(Copy, Clone)]
pub enum Control {
    Shutdown,
    PublicationCadence(PublicationCadence),
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum PublicationCadence {
    None,
    OnChange,
    Periodic(Duration),
}

impl From<PublishPeriod> for PublicationCadence {
    fn from(val: PublishPeriod) -> Self {
        let steps = val.steps();
        if steps == 0 {
            PublicationCadence::OnChange
        } else {
            PublicationCadence::Periodic(match val.resolution() {
                Resolution::Milliseconds100 => Duration::from_millis(steps as u64 * 100),
                Resolution::Seconds1 => Duration::from_secs(steps as u64),
                Resolution::Seconds10 => Duration::from_secs(steps as u64 * 10),
                Resolution::Minutes10 => Duration::from_secs(steps as u64 * 60 * 10),
            })
        }
    }
}

pub struct InboundMessage {
    pub opcode: Opcode,
    pub parameters: Vec<u8, 380>,
    pub meta: InboundMetadata,
}

pub type InboundModelChannel<M> = Channel<CriticalSectionRawMutex, InboundModelPayload<M>, 1>;
pub type InboundModelChannelSender<'m, M> =
    Sender<'m, CriticalSectionRawMutex, InboundModelPayload<M>, 1>;
pub type InboundModelChannelReceiver<'m, M> =
    Receiver<'m, CriticalSectionRawMutex, InboundModelPayload<M>, 1>;

pub enum InboundModelPayload<M> {
    Message(M, InboundMetadata),
    Control(Control),
}

pub type OutboundChannel = Channel<CriticalSectionRawMutex, OutboundPayload, 1>;
pub type OutboundChannelSender = Sender<'static, CriticalSectionRawMutex, OutboundPayload, 1>;
pub type OutboundChannelReceiver = Receiver<'static, CriticalSectionRawMutex, OutboundPayload, 1>;

pub struct OutboundPayload {
    pub element_index: usize,
    pub model_identifer: ModelIdentifier,
    pub opcode: Opcode,
    pub parameters: Vec<u8, 380>,
    pub extra: OutboundExtra,
}

pub enum OutboundExtra {
    Send(SendExtra),
    Publish,
}

pub struct SendExtra {
    pub meta: OutboundMetadata,
    pub completion_token: Option<CompletionToken>,
}

impl From<SendExtra> for OutboundExtra {
    fn from(inner: SendExtra) -> Self {
        OutboundExtra::Send(inner)
    }
}

pub trait BluetoothMeshDeviceContext {
    type ElementContext: BluetoothMeshElementContext;

    fn element_context(
        &self,
        index: usize,
        inbound: InboundChannelReceiver,
    ) -> Self::ElementContext;

    type ReceiveFuture<'f>: Future<Output = AccessCountedHandle<'static, InboundPayload>> + 'f
    where
        Self: 'f;

    fn receive(&self) -> Self::ReceiveFuture<'_>;
}

pub trait BluetoothMeshDevice {
    fn composition(&self) -> Composition<CompositionExtra>;

    type RunFuture<'f, C>: Future<Output = Result<(), ()>> + 'f
    where
        Self: 'f,
        C: BluetoothMeshDeviceContext + 'f;

    fn run<'run, C: BluetoothMeshDeviceContext + 'run>(
        &'run mut self,
        ctx: C,
    ) -> Self::RunFuture<'run, C>;
}

pub trait BluetoothMeshElement {
    fn populate(&self, composition: &mut Composition<CompositionExtra>);

    type RunFuture<'f, C>: Future<Output = Result<(), ()>> + 'f
    where
        Self: 'f,
        C: BluetoothMeshElementContext + 'f;

    fn run<'run, C: BluetoothMeshElementContext + 'run>(
        &'run mut self,
        ctx: C,
    ) -> Self::RunFuture<'run, C>;
}

pub trait BluetoothMeshElementContext {
    type ModelContext<'m, M: Model>: BluetoothMeshModelContext<M>
    where
        M: 'm,
        Self: 'm;

    fn model_context<'m, M: Model + 'm>(
        &'m self,
        inbound: InboundModelChannelReceiver<'m, M::Message>,
    ) -> Self::ModelContext<'m, M>;

    type ReceiveFuture<'f>: Future<Output = AccessCountedHandle<'static, InboundPayload>> + 'f
    where
        Self: 'f;

    fn receive(&self) -> Self::ReceiveFuture<'_>;
}

pub trait BluetoothMeshModel<M: Model> {
    type Model: Model = M;

    type RunFuture<'f, C>: Future<Output = Result<(), ()>> + 'f
    where
        Self: 'f,
        C: BluetoothMeshModelContext<M> + 'f;

    fn run<'run, C: BluetoothMeshModelContext<M> + 'run>(
        &'run mut self,
        ctx: C,
    ) -> Self::RunFuture<'_, C>;

    fn model_identifier(&self) -> ModelIdentifier {
        M::IDENTIFIER
    }

    fn parser(&self) -> ParseFunction<M> {
        M::parse as fn(&Opcode, &[u8]) -> Result<Option<M::Message>, ParseError>
    }
}

pub type ParseFunction<M> =
    for<'r> fn(&Opcode, &'r [u8]) -> Result<Option<<M as Model>::Message>, ParseError>;

pub trait BluetoothMeshModelContext<M: Model> {
    type ReceiveFuture<'f>: Future<Output = InboundModelPayload<M::Message>> + 'f
    where
        Self: 'f,
        M: 'f;

    fn receive(&self) -> Self::ReceiveFuture<'_>;

    type SendFuture<'f>: Future<Output = Result<(), ()>> + 'f
    where
        Self: 'f,
        M: 'f;

    fn send(&self, message: M::Message, meta: OutboundMetadata) -> Self::SendFuture<'_>;

    type SendWithCompletionFuture<'f>: Future<Output = CompletionStatus> + 'f
    where
        Self: 'f,
        M: 'f;

    fn send_with_completion(
        &self,
        message: M::Message,
        meta: OutboundMetadata,
        signal: &'static Signal<CompletionStatus>,
    ) -> Self::SendWithCompletionFuture<'_>;

    type PublishFuture<'f>: Future<Output = Result<(), ()>> + 'f
    where
        Self: 'f,
        M: 'f;

    fn publish(&self, message: M::Message) -> Self::PublishFuture<'_>;
}

pub enum CompletionStatus {
    Complete,
    Incomplete,
}

#[derive(Clone)]
pub struct CompletionToken {
    signal: &'static Signal<CompletionStatus>,
}

impl CompletionToken {
    pub fn new(signal: &'static Signal<CompletionStatus>) -> Self {
        Self { signal }
    }

    pub fn complete(&self) {
        self.signal.signal(CompletionStatus::Complete)
    }

    pub fn incomplete(&self) {
        self.signal.signal(CompletionStatus::Incomplete)
    }
}

impl Drop for CompletionToken {
    fn drop(&mut self) {
        if !self.signal.signaled() {
            self.incomplete()
        }
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct InboundMetadata {
    src: UnicastAddress,
    dst: Address,
    ttl: Ttl,
    //
    network_key_handle: NetworkKeyHandle,
    iv_index: IvIndex,
    key_handle: KeyHandle,
    label_uuid: Option<LabelUuid>,
}

impl InboundMetadata {
    pub fn new(
        src: UnicastAddress,
        dst: Address,
        ttl: Ttl,
        network_key_handle: NetworkKeyHandle,
        iv_index: IvIndex,
        key_handle: KeyHandle,
        label_uuid: Option<LabelUuid>,
    ) -> Self {
        Self {
            src,
            dst,
            ttl,
            network_key_handle,
            iv_index,
            key_handle,
            label_uuid,
        }
    }
    pub fn src(&self) -> UnicastAddress {
        self.src
    }

    pub fn dst(&self) -> Address {
        self.dst
    }

    pub fn ttl(&self) -> Ttl {
        self.ttl
    }

    pub fn reply(&self) -> OutboundMetadata {
        OutboundMetadata {
            dst: self.src.into(),
            network_key_handle: self.network_key_handle,
            iv_index: self.iv_index,
            key_handle: self.key_handle,
            label_uuid: self.label_uuid,
            ttl: None,
        }
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OutboundMetadata {
    dst: Address,
    //
    network_key_handle: NetworkKeyHandle,
    iv_index: IvIndex,
    key_handle: KeyHandle,
    label_uuid: Option<LabelUuid>,
    ttl: Option<Ttl>,
}

impl OutboundMetadata {
    pub fn with_ttl(mut self, ttl: Ttl) -> Self {
        self.ttl.replace(ttl);
        self
    }

    pub fn dst(&self) -> Address {
        self.dst
    }

    pub fn network_key_handle(&self) -> NetworkKeyHandle {
        self.network_key_handle
    }

    pub fn iv_index(&self) -> IvIndex {
        self.iv_index
    }

    pub fn key_handle(&self) -> KeyHandle {
        self.key_handle
    }

    pub fn label_uuid(&self) -> Option<LabelUuid> {
        self.label_uuid
    }

    pub fn ttl(&self) -> Option<Ttl> {
        self.ttl
    }
}

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum KeyHandle {
    Device,
    Network(NetworkKeyHandle),
    Application(ApplicationKeyHandle),
}

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NetworkKeyHandle {
    index: NetKeyIndex,
    nid: Nid,
}

impl NetworkKeyHandle {
    pub fn new(index: NetKeyIndex, nid: Nid) -> Self {
        Self { index, nid }
    }

    pub fn nid(&self) -> Nid {
        self.nid
    }

    pub fn index(&self) -> NetKeyIndex {
        self.index
    }
}

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ApplicationKeyHandle {
    index: AppKeyIndex,
    aid: Aid,
}

impl ApplicationKeyHandle {
    pub fn new(index: AppKeyIndex, aid: Aid) -> Self {
        Self { index, aid }
    }

    pub fn aid(&self) -> Aid {
        self.aid
    }
}

impl From<ApplicationKeyHandle> for AppKeyIndex {
    fn from(handle: ApplicationKeyHandle) -> Self {
        handle.index
    }
}

pub struct CompositionExtra {
    pub publication_cadence: PublicationCadence,
}

impl Default for CompositionExtra {
    fn default() -> Self {
        Self {
            publication_cadence: PublicationCadence::None,
        }
    }
}
