use crate::System;
use crate::access::{AccessMessage, Opcode};
use btmesh_common::{address::{Address, UnicastAddress}, Aid, ParseError};
use core::convert::TryInto;
use heapless::Vec;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum UpperPDU<S:System> {
    Control(UpperControl<S>),
    Access(UpperAccess<S>),
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UpperControl<S:System> {
    pub(crate) ttl: u8,
    pub(crate) network_key: S::NetworkKeyHandle,
    pub(crate) ivi: u8,
    pub(crate) nid: u8,
    pub(crate) src: UnicastAddress,
    pub(crate) dst: Address,
    pub(crate) opcode: Opcode,
    pub(crate) data: Vec<u8, 256>,
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UpperAccess<S:System> {
    pub(crate) ttl: Option<u8>,
    pub(crate) network_key: S::NetworkKeyHandle,
    pub(crate) ivi: u8,
    pub(crate) nid: u8,
    pub(crate) akf: bool,
    pub(crate) aid: Aid,
    pub(crate) src: UnicastAddress,
    pub(crate) dst: Address,
    pub(crate) payload: Vec<u8, 380>,
}

impl<S:System> TryInto<AccessMessage<S>> for UpperPDU<S> {
    type Error = ParseError;

    fn try_into(self) -> Result<AccessMessage<S>, Self::Error> {
        match self {
            UpperPDU::Control(_) => Err(ParseError::InvalidPDUFormat),
            UpperPDU::Access(inner) => AccessMessage::parse(&inner),
        }
    }
}
