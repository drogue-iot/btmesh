#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]

use crate::provisioned::network::NetworkPDU;
use crate::provisioning::ProvisioningPDU;

pub const PB_ADV: u8 = 0x29;
pub const MESH_MESSAGE: u8 = 0x2A;
pub const MESH_BEACON: u8 = 0x2B;

pub mod provisioned;
pub mod provisioning;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PDU {
    Provisioning(ProvisioningPDU),
    Network(NetworkPDU),
}
