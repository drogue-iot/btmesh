use crate::models::configuration::Configuration;
use crate::{BackingStore, Storage};
use btmesh_device::BluetoothMeshModel;
use btmesh_macro::{device, element};
use core::future::Future;

pub mod configuration;

#[device(cid = 0, pid = 0, vid = 0)]
pub struct FoundationDevice<'s, B: BackingStore + 's> {
    zero: Zero<'s, B>,
}

impl<'s, B: BackingStore> FoundationDevice<'s, B> {
    pub fn new(storage: &'s Storage<B>) -> Self {
        Self {
            zero: Zero::new(storage),
        }
    }
}

#[element(location = "internal")]
pub struct Zero<'s, B: BackingStore + 's> {
    config: Configuration<'s, B>,
}

impl<'s, B: BackingStore> Zero<'s, B> {
    pub fn new(storage: &'s Storage<B>) -> Self {
        Self {
            config: Configuration::new(storage),
        }
    }
}
