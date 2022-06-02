use crate::access::{AccessMessage, Opcode};
use btmesh_common::{
    address::{Address, UnicastAddress},
    ParseError,
};
use core::convert::TryInto;
use heapless::Vec;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum UpperPDU {
    Control(UpperControl),
    Access(UpperAccess),
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UpperControl {
    pub(crate) ttl: u8,
    // TODO: pub(crate) network_key: NetworkKeyHandle,
    pub(crate) ivi: u8,
    pub(crate) nid: u8,
    pub(crate) src: UnicastAddress,
    pub(crate) dst: Address,
    pub(crate) opcode: Opcode,
    pub(crate) data: Vec<u8, 256>,
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UpperAccess {
    pub(crate) ttl: Option<u8>,
    // TODO: pub(crate) network_key: NetworkKeyHandle,
    pub(crate) ivi: u8,
    pub(crate) nid: u8,
    pub(crate) akf: bool,
    pub(crate) aid: crate::ApplicationKeyIdentifier,
    pub(crate) src: UnicastAddress,
    pub(crate) dst: Address,
    pub(crate) payload: Vec<u8, 380>,
}

impl TryInto<AccessMessage> for UpperPDU {
    type Error = ParseError;

    fn try_into(self) -> Result<AccessMessage, Self::Error> {
        match self {
            UpperPDU::Control(_) => Err(ParseError::InvalidPDUFormat),
            UpperPDU::Access(inner) => AccessMessage::parse(&inner),
        }
    }
}
