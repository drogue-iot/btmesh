#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]

use btmesh_common::address::InvalidAddress;
use btmesh_common::mic::InvalidLength;
use btmesh_common::{InsufficientBuffer, ParseError, SeqRolloverError};
use btmesh_pdu::lower::InvalidBlock;

mod error;
pub mod stack;
pub mod unprovisioned;

pub use error::DriverError;
use crate::stack::provisioned::network::DeviceInfo;
use crate::stack::provisioned::{NetworkState, ProvisionedStack};
use crate::stack::provisioned::secrets::Secrets;
use crate::stack::Stack;


pub struct Driver {
    stack: Stack,
}

impl Driver {
    pub fn new_provisioned(device_info: DeviceInfo, secrets: Secrets, network_state: NetworkState) -> Self {
        Self {
            stack: Stack::Provisioned(
                ProvisionedStack::new(
                    device_info,
                    secrets,
                    network_state,
                )
            )
        }
    }

    /// Perform a single end-to-end loop through the driver's processing logic.
    pub async fn process(&mut self) -> Result<(), DriverError> {
        Ok(())
    }

}