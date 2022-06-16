#![allow(dead_code)]

use crate::network::replay_protection::ReplayProtection;
use btmesh_common::address::{Address, InvalidAddress, LabelUuid, UnicastAddress};
use btmesh_common::{Aid, InsufficientBuffer, IvIndex, ParseError, Seq};
use btmesh_pdu::lower::{InvalidBlock, LowerPDU, SegmentedLowerPDU, UnsegmentedLowerPDU};
use btmesh_pdu::network::CleartextNetworkPDU;
use btmesh_pdu::System;
use heapless::Vec;
use hash32_derive::Hash32;
use btmesh_pdu::upper::access::UpperAccessPDU;
use secrets::Secrets;

mod lower;
mod network;
mod secrets;
mod upper;

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
    upper: upper::UpperDriver,
    lower: lower::LowerDriver,
    network: network::NetworkDriver,
}

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd)]
pub enum KeyHandle {
    Device,
    Network(NetworkKeyHandle),
    Application(ApplicationKeyHandle)
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

#[derive(Copy, Clone)]
pub struct NetworkMetadata {
    iv_index: IvIndex,
    replay_protected: bool,
    should_relay: bool,
    local_element_index: Option<u8>,
}

impl NetworkMetadata {
    pub fn iv_index(&self) -> IvIndex {
        self.iv_index
    }

    pub fn replay_protected(&mut self, protected: bool) {
        self.replay_protected = protected;
    }

    pub fn should_relay(&mut self, relay: bool) {
        self.should_relay = relay;
    }

    pub fn local_element_index(&self) -> Option<u8> {
        self.local_element_index
    }
}

#[derive(Copy, Clone)]
pub struct LowerMetadata {
    iv_index: IvIndex,
    src: UnicastAddress,
    dst: Address,
    seq: Seq,
}

impl LowerMetadata {

    pub fn new(iv_index: IvIndex, src: UnicastAddress, dst: Address, seq: Seq) -> Self {
        Self {
            iv_index,
            src,
            dst,
            seq
        }
    }

    pub fn from_network_pdu(pdu: &CleartextNetworkPDU<Driver>) -> Self {
        Self {
            iv_index: pdu.meta().iv_index(),
            src: pdu.src(),
            dst: pdu.dst(),
            seq: pdu.seq()
        }
    }

    pub fn src(&self) -> UnicastAddress {
        self.src
    }

    pub fn dst(&self) -> Address {
        self.dst
    }

    pub fn seq(&self) -> Seq {
        self.seq
    }

    pub fn iv_index(&self) -> IvIndex {
        self.iv_index
    }
}

#[derive(Clone)]
pub struct UpperMetadata {
    iv_index: IvIndex,
    akf_aid: Option<Aid>,
    seq: Seq,
    src: UnicastAddress,
    dst: Address,
    label_uuids: Vec<LabelUuid, 3>,
}

impl UpperMetadata {

    pub fn from_segmented_lower_pdu(pdu: &SegmentedLowerPDU<Driver>) -> Self {
        Self {
            iv_index: pdu.meta().iv_index(),
            akf_aid: if let SegmentedLowerPDU::Access(inner) = pdu { inner.aid() } else {None},
            seq: pdu.meta().seq(),
            src: pdu.meta().src(),
            dst: pdu.meta().dst(),
            label_uuids: Default::default()
        }
    }

    pub fn from_unsegmented_lower_pdu(pdu: &UnsegmentedLowerPDU<Driver>) -> Self {
        Self {
            iv_index: pdu.meta().iv_index(),
            akf_aid: if let UnsegmentedLowerPDU::Access(inner) = pdu { inner.aid() } else {None},
            seq: pdu.meta().seq(),
            src: pdu.meta().src(),
            dst: pdu.meta().dst(),
            label_uuids: Default::default()
        }
    }

    pub fn from_lower_pdu(pdu: &LowerPDU<Driver>) -> Self {
        match pdu {
            LowerPDU::Unsegmented(inner) => Self::from_unsegmented_lower_pdu(inner),
            LowerPDU::Segmented(inner) => Self::from_segmented_lower_pdu(inner),
        }
    }

    pub fn iv_index(&self) -> IvIndex {
        self.iv_index
    }

    pub fn aid(&self) -> Option<Aid> {
        self.akf_aid
    }

    pub fn seq(&self) -> Seq {
        self.seq
    }

    pub fn src(&self) -> UnicastAddress {
        self.src
    }

    pub fn dst(&self) -> Address {
        self.dst
    }

    pub fn label_uuids(&self) -> &[LabelUuid] {
        &*self.label_uuids
    }

    pub fn add_label_uuid(&mut self, label_uuid: LabelUuid) -> Result<(), DriverError> {
        self.label_uuids.push(label_uuid).map_err(|_| DriverError::InsufficientSpace)?;
        Ok(())
    }


    /*
    pub(crate) fn apply(&mut self, pdu: &LowerPDU<Driver>) {
        match pdu {
            LowerPDU::Unsegmented(UnsegmentedLowerPDU::Access(access)) => {
                self.akf_aid = access.aid().clone();
            }
            LowerPDU::Segmented(SegmentedLowerPDU::Access(access)) => {
                self.akf_aid = access.aid().clone();
            }
            _ => { /* nothing */ }
        }

        self.iv_index = pdu.meta().iv_index().clone();
        self.seq = pdu.meta().seq().clone();
        self.src = pdu.meta().src().clone();
        self.dst = pdu.meta().dst().clone();
    }
     */
}

#[derive(Copy, Clone)]
pub struct AccessMetadata {
    iv_index: IvIndex,
    key_handle: KeyHandle,
    src: UnicastAddress,
    dst: Address,
    label_uuid: Option<LabelUuid>,
}

impl AccessMetadata {
    pub fn from_upper_access_pdu(key_handle: KeyHandle, label_uuid: Option<LabelUuid>, pdu: UpperAccessPDU<Driver>) -> Self {
        Self {
            iv_index: pdu.meta().iv_index,
            key_handle,
            src: pdu.meta().src,
            dst: pdu.meta().dst,
            label_uuid,
        }
    }

}
