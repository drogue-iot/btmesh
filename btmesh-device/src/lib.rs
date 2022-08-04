#![cfg_attr(not(test), no_std)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(associated_type_defaults)]
#![allow(dead_code)]

pub use btmesh_common::location;
pub use btmesh_common::ElementDescriptor;
use btmesh_common::ParseError;
pub use btmesh_common::{
    CompanyIdentifier, Composition, Features, InsufficientBuffer, ModelIdentifier,
    ProductIdentifier, VersionIdentifier,
};
pub use btmesh_models::{Model, Opcode};
use core::future::Future;
use embassy::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
pub use embassy::channel::{Channel, Receiver, Sender};
pub use futures::future::join;
use heapless::Vec;

pub type ChannelImpl = Channel<CriticalSectionRawMutex, ReceivePayload, 1>;
pub type SenderImpl = Sender<'static, CriticalSectionRawMutex, ReceivePayload, 1>;
pub type ReceiverImpl = Receiver<'static, CriticalSectionRawMutex, ReceivePayload, 1>;
pub type ReceivePayload = (Option<usize>, Opcode, Vec<u8, 380>);

pub trait BluetoothMeshDeviceContext {
    type ElementContext: BluetoothMeshElementContext;

    fn element_context(&self, index: usize, channel: ChannelImpl) -> Self::ElementContext;

    type ReceiveFuture<'f>: Future<Output = ReceivePayload> + 'f
    where
        Self: 'f;

    fn receive(&self) -> Self::ReceiveFuture<'_>;
}

pub trait BluetoothMeshDevice {
    fn composition(&self) -> Composition;

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
    fn populate(&self, composition: &mut Composition);

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
    type ModelContext<M: Model>: BluetoothMeshModelContext<M>;
    fn model_context<M: Model>(&self, index: usize, channel: ChannelImpl) -> Self::ModelContext<M>;

    type ReceiveFuture<'f>: Future<Output = ReceivePayload> + 'f
    where
        Self: 'f;

    fn receive(&self) -> Self::ReceiveFuture<'_>;
}

pub trait BluetoothMeshModel<M: Model> {
    type RunFuture<'f, C>: Future<Output = Result<(), ()>> + 'f
    where
        Self: 'f,
        C: BluetoothMeshModelContext<M> + 'f;

    fn run<'run, C: BluetoothMeshModelContext<M> + 'run>(
        &'run self,
        ctx: C,
    ) -> Self::RunFuture<'_, C>;

    fn model_identifier(&self) -> ModelIdentifier {
        M::IDENTIFIER
    }
}

pub trait BluetoothMeshModelContext<M: Model> {
    type ReceiveFuture<'f>: Future<Output = M::Message> + 'f
    where
        Self: 'f,
        M: 'f;

    fn receive(&self) -> Self::ReceiveFuture<'_>;
}
