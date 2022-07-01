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
