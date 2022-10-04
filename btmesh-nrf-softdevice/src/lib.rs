#![cfg_attr(not(test), no_std)]
#![feature(type_alias_impl_trait)]
#![feature(associated_type_defaults)]
#![feature(future_join)]
#![allow(dead_code)]

#[cfg(not(any(feature = "nrf52833", feature = "nrf52840",)))]
compile_error!("No chip feature activated. You must activate exactly one of the following features: nrf52833, nrf52840");

mod advertising;
mod gatt;

mod rng;

mod driver;

pub use btmesh_driver::{BluetoothMeshDriver, BluetoothMeshDriverConfig};

#[cfg(feature = "gatt")]
pub use driver::NrfSoftdeviceAdvertisingAndGattDriver as Driver;

#[cfg(not(feature = "gatt"))]
pub use driver::NrfSoftdeviceAdvertisingOnlyDriver as Driver;
