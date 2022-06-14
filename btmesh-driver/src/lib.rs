#![allow(dead_code)]
use crate::network::replay_protection::ReplayProtection;
use btmesh_common::address::{InvalidAddress, UnicastAddress};
use btmesh_common::{InsufficientBuffer, IvIndex, ParseError};
use btmesh_pdu::network::CleartextNetworkPDU;
use btmesh_pdu::System;
use hash32_derive::Hash32;
use btmesh_pdu::lower::InvalidBlock;
use secrets::Secrets;

mod lower;
mod network;
mod secrets;

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
    type UpperMetadata = UpperMetadata;
    type AccessMetadata = AccessMetadata;
}

#[derive(Copy, Clone, Default)]
pub struct NetworkMetadata {
    iv_index: IvIndex,
    replay_protected: Option<bool>,
    should_relay: Option<bool>,
}

impl NetworkMetadata {
    pub fn iv_index(&self) -> IvIndex {
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
pub struct LowerMetadata {
    src: Option<UnicastAddress>,
    iv_index: Option<IvIndex>,
}

impl LowerMetadata {
    pub(crate) fn apply(&mut self, pdu: &CleartextNetworkPDU<Driver>) {
        self.src.replace(pdu.src());
    }
}

#[derive(Copy, Clone, Default)]
pub struct UpperMetadata {
    iv_index: IvIndex,
}

#[derive(Copy, Clone, Default)]
pub struct AccessMetadata {
    iv_index: IvIndex,
}
