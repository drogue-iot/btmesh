#![cfg_attr(not(test), no_std)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(associated_type_defaults)]
#![allow(dead_code)]

pub use btmesh_common::location;
pub use btmesh_common::ElementDescriptor;
pub use btmesh_common::{
    CompanyIdentifier, Composition, Features, InsufficientBuffer, ModelIdentifier,
    ProductIdentifier, VersionIdentifier,
};
pub use btmesh_models::Opcode;
use core::future::Future;

pub trait BluetoothMeshDevice {
    fn composition(&self) -> Composition;

    type DispatchFuture<'f>: Future<Output = Result<(), ()>> + 'f
    where
        Self: 'f;

    fn dispatch<'f>(
        &'f mut self,
        index: usize,
        opcode: Opcode,
        parameters: &'f [u8],
    ) -> Self::DispatchFuture<'f>;
}

pub trait BluetoothMeshElement {
    fn populate(&self, composition: &mut Composition);

    type DispatchFuture<'f>: Future<Output = Result<(), ()>> + 'f
    where
        Self: 'f;

    fn dispatch<'f>(&'f mut self, opcode: Opcode, parameters: &'f [u8])
        -> Self::DispatchFuture<'f>;
}

pub fn features() -> Features {
    Features {
        relay: cfg!(relay),
        proxy: cfg!(proxy),
        friend: cfg!(friend),
        low_power: cfg!(low_power),
    }
}
