#![cfg_attr(not(test), no_std)]

#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(associated_type_defaults)]

pub mod beacon;
mod error;

use core::future::Future;
use heapless::Vec;
pub use error::BearerError;

pub const PB_ADV_MTU: usize = 64;

pub trait AdvertisingBearer {

    type ReceiveFuture<'m>: Future<Output = Result<Vec<u8, PB_ADV_MTU>, BearerError>> + 'm
        where
            Self: 'm;

    /// Receive data from the bearer.
    fn receive<'m>(&'m self) -> Self::ReceiveFuture<'m>;

    type TransmitFuture<'m>: Future<Output = Result<(), BearerError>> + 'm
        where
            Self: 'm;

    /// Transmit data on the bearer.
    fn transmit<'m>(&'m self, pdu: &'m Vec<u8, PB_ADV_MTU>) -> Self::TransmitFuture<'m>;
}

pub trait GattBearer<const MTU: usize> {

    type RunFuture<'m>: Future<Output = Result<(), BearerError>> + 'm
        where
            Self: 'm;

    fn run<'m>(&'m self) -> Self::RunFuture<'m>;

    type ReceiveFuture<'m>: Future<Output = Result<Vec<u8, MTU>, BearerError>> + 'm
        where
            Self: 'm;

    /// Receive data from the bearer.
    fn receive<'m>(&'m self) -> Self::ReceiveFuture<'m>;

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

