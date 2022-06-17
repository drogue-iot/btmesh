#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]

use btmesh_common::address::InvalidAddress;
use btmesh_common::{InsufficientBuffer, ParseError};
use btmesh_pdu::lower::InvalidBlock;

pub mod provisioned;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DriverError {
    InvalidState,
    InvalidKeyLength,
    CryptoError,
    InvalidAddress,
    InsufficientSpace,
    InvalidKeyHandle,
    InvalidPDU,
    ParseError(ParseError),
}

impl From<InsufficientBuffer> for DriverError {
    fn from(_: InsufficientBuffer) -> Self {
        Self::InsufficientSpace
    }
}

impl From<ParseError> for DriverError {
    fn from(inner: ParseError) -> Self {
        Self::ParseError(inner)
    }
}

impl From<InvalidAddress> for DriverError {
    fn from(_: InvalidAddress) -> Self {
        Self::InvalidAddress
    }
}

impl From<InvalidBlock> for DriverError {
    fn from(_: InvalidBlock) -> Self {
        Self::InvalidState
    }
}
