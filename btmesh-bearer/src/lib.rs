#![cfg_attr(not(test), no_std)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(associated_type_defaults)]

pub mod beacon;
mod error;

use core::future::Future;
pub use error::BearerError;
use heapless::Vec;

pub const PB_ADV_MTU: usize = 64;

pub trait AdvertisingBearer {
    type ReceiveFuture<'m>: Future<Output = Result<Vec<u8, PB_ADV_MTU>, BearerError>> + 'm
    where
        Self: 'm;

    /// Receive data from the bearer.
    fn receive(&self) -> Self::ReceiveFuture<'_>;

    type TransmitFuture<'m>: Future<Output = Result<(), BearerError>> + 'm
    where
        Self: 'm;

    /// Transmit data on the bearer.
    fn transmit<'m>(&'m self, pdu: &'m Vec<u8, PB_ADV_MTU>) -> Self::TransmitFuture<'m>;
}

pub trait GattBearer<const MTU: usize> {
    fn reset(&self);

    type RunFuture<'m>: Future<Output = Result<(), BearerError>> + 'm
    where
        Self: 'm;

    fn run(&self) -> Self::RunFuture<'_>;

    type ReceiveFuture<'m>: Future<Output = Result<Vec<u8, MTU>, BearerError>> + 'm
    where
        Self: 'm;

    /// Receive data from the bearer.
    fn receive(&self) -> Self::ReceiveFuture<'_>;

    type TransmitFuture<'m>: Future<Output = Result<(), BearerError>> + 'm
    where
        Self: 'm;

    /// Transmit data on the bearer.
    fn transmit<'m>(&'m self, pdu: &'m Vec<u8, MTU>) -> Self::TransmitFuture<'m>;

    type AdvertiseFuture<'m>: Future<Output = Result<(), BearerError>> + 'm
    where
        Self: 'm;

    /// Transmit data on the bearer.
    fn advertise<'m>(&'m self, adv_data: &'m Vec<u8, 64>) -> Self::AdvertiseFuture<'m>;
}
