use crate::network::replay_protection::ReplayProtection;
use btmesh_common::address::InvalidAddress;
use btmesh_common::{InsufficientBuffer, ParseError};
use btmesh_pdu::System;
use hash32_derive::Hash32;
use secrets::Secrets;

mod lower;
mod network;
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
    lower: lower::LowerDriver,
    network: network::NetworkDriver,
}

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Hash32)]
pub struct NetworkKeyHandle(u8);

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Hash32)]
pub struct ApplicationKeyHandle(u8);

impl System for Driver {
    type NetworkKeyHandle = NetworkKeyHandle;
    type ApplicationKeyHandle = ApplicationKeyHandle;
    type NetworkMetadata = NetworkMetadata;
    type LowerMetadata = LowerMetadata;
}

#[derive(Copy, Clone, Default)]
pub struct NetworkMetadata {
    iv_index: u32,
    replay_protected: Option<bool>,
    should_relay: Option<bool>,
}

impl NetworkMetadata {
    pub fn iv_index(&self) -> u32 {
        self.iv_index
    }

    pub fn replay_protected(&mut self, protected: bool) {
        self.replay_protected.replace(protected);
    }

    pub fn should_relay(&mut self, relay: bool) {
        self.should_relay.replace(relay);
    }
}

#[derive(Copy, Clone, Default)]
pub struct LowerMetadata {}

#[derive(Copy, Clone, Default)]
pub struct UpperMetadata {}



