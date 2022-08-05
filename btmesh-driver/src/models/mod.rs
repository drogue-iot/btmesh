use btmesh_device::{BluetoothMeshModel, BluetoothMeshModelContext};
use btmesh_macro::{device, element};
use btmesh_models::foundation::configuration::{ConfigurationMessage, ConfigurationServer};
use cmac::crypto_mac::Output;
use core::future::Future;
use crate::models::configuration::Configuration;

pub mod configuration;

#[device(cid = 0, pid = 0, vid = 0)]
pub struct FoundationDevice {
    zero: Zero,
}

impl FoundationDevice {
    pub fn new() -> Self {
        Self {
            zero: Zero::new(),
        }
    }
}

#[element(location = "internal")]
pub struct Zero {
    config: Configuration,
}

impl Zero {
    pub fn new() ->Self {
        Self {
            config: Configuration::new(),
        }
    }
}


