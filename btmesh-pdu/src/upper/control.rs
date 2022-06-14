use crate::upper::UpperPDU;
use crate::System;
use btmesh_common::{InsufficientBuffer, ParseError};
use heapless::Vec;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum UpperControlOpcode {
    FriendPoll = 0x01,
    FriendUpdate = 0x02,
    FriendRequest = 0x03,
    FriendOffer = 0x04,
    FriendClear = 0x05,
    FriendClearConfirm = 0x06,
    FriendSubscriptionListAdd = 0x07,
    FriendSubscriptionListRemove = 0x08,
    FriendSubscriptionListConfirm = 0x09,
    Heartbeat = 0x0A,
}

impl UpperControlOpcode {
    pub fn parse(data: u8) -> Result<Self, ParseError> {
        match data {
            0x01 => Ok(Self::FriendPoll),
            0x02 => Ok(Self::FriendUpdate),
            0x03 => Ok(Self::FriendRequest),
            0x04 => Ok(Self::FriendOffer),
            0x05 => Ok(Self::FriendClear),
            0x06 => Ok(Self::FriendClearConfirm),
            0x07 => Ok(Self::FriendSubscriptionListAdd),
            0x08 => Ok(Self::FriendSubscriptionListRemove),
            0x09 => Ok(Self::FriendSubscriptionListConfirm),
            0x0A => Ok(Self::Heartbeat),
            _ => Err(ParseError::InvalidValue),
        }
    }
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[allow(dead_code)]
pub struct UpperControlPDU<S: System> {
    opcode: UpperControlOpcode,
    parameters: Vec<u8, 256>,
    meta: S::UpperMetadata,
}

impl<S: System> UpperControlPDU<S> {
    pub fn new(opcode: UpperControlOpcode, parameters: &[u8]) -> Result<Self, InsufficientBuffer> {
        Ok(Self {
            opcode,
            parameters: Vec::from_slice(parameters)?,
            meta: Default::default(),
        })
    }

    pub fn parse(opcode: UpperControlOpcode, data: &[u8]) -> Result<Self, ParseError> {
        Ok(Self {
            opcode,
            parameters: Vec::from_slice(data)?,
            meta: Default::default(),
        })
    }
}

impl<S: System> From<UpperControlPDU<S>> for UpperPDU<S> {
    fn from(pdu: UpperControlPDU<S>) -> Self {
        UpperPDU::Control(pdu)
    }
}
