#![cfg_attr(not(test), no_std)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(associated_type_defaults)]
#![allow(dead_code)]

use btmesh_common::Uuid;

mod error;
pub mod stack;
pub mod unprovisioned;

use crate::stack::provisioned::network::DeviceInfo;
use crate::stack::provisioned::secrets::Secrets;
use crate::stack::provisioned::{NetworkState, ProvisionedStack};
use crate::stack::Stack;
pub use error::DriverError;

pub struct Driver {
    stack: Stack,
}

impl Driver {
    pub fn new_provisioned(
        device_info: DeviceInfo,
        secrets: Secrets,
        network_state: NetworkState,
    ) -> Self {
        Self {
            stack: Stack::Provisioned(ProvisionedStack::new(device_info, secrets, network_state)),
        }
    }

    /// Perform a single end-to-end loop through the driver's processing logic.
    pub async fn process(&mut self) -> Result<(), DriverError> {
        Ok(())
    }
}

pub enum DeviceState {
    Unprovisioned { uuid: Uuid },
    Provisioned,
}
