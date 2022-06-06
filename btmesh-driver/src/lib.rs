use btmesh_common::address::InvalidAddress;
use btmesh_common::{InsufficientBuffer, ParseError};
use btmesh_pdu::System;
use hash32::{Hash, Hasher};
use hash32_derive::Hash32;
use secrets::Secrets;

pub mod network;
mod secrets;

#[derive(Debug)]
pub enum DriverError {
    InvalidKeyLength,
    CryptoError,
    InvalidAddress,
    InsufficientSpace,
    InvalidKeyHandle,
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

pub struct Driver {
    secrets: Secrets,
}

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Hash32)]
pub struct NetworkKeyHandle(u8);

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Hash32)]
pub struct ApplicationKeyHandle(u8);

impl System for Driver {
    type NetworkKeyHandle = NetworkKeyHandle;
    type ApplicationKeyHandle = ApplicationKeyHandle;
}
