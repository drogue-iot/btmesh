use crate::provisioned::upper::UpperPDU;
use crate::provisioned::System;
use btmesh_common::{InsufficientBuffer, ParseError};
use heapless::Vec;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ControlOpcode {
    SegmentAcknowledgement = 0x00,
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

impl ControlOpcode {
    pub fn parse(data: u8) -> Result<Self, ParseError> {
        match data {
            0x00 => Ok(Self::SegmentAcknowledgement),
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
pub struct UpperControlPDU<S: System> {
    opcode: ControlOpcode,
    parameters: Vec<u8, 256>,
    meta: S::UpperMetadata,
}

impl<S: System> Clone for UpperControlPDU<S> {
    fn clone(&self) -> Self {
        Self {
            opcode: self.opcode,
            parameters: self.parameters.clone(),
            meta: self.meta.clone(),
        }
    }
}

impl<S: System> UpperControlPDU<S> {
    pub fn new(
        opcode: ControlOpcode,
        parameters: &[u8],
        meta: S::UpperMetadata,
    ) -> Result<Self, InsufficientBuffer> {
        Ok(Self {
            opcode,
            parameters: Vec::from_slice(parameters)?,
            meta,
        })
    }

    pub fn parse(
        opcode: ControlOpcode,
        data: &[u8],
        meta: S::UpperMetadata,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            opcode,
            parameters: Vec::from_slice(data)?,
            meta,
        })
    }

    pub fn opcode(&self) -> ControlOpcode {
        self.opcode
    }

    pub fn parameters(&self) -> &[u8] {
        &self.parameters
    }

    pub fn meta(&self) -> &S::UpperMetadata {
        &self.meta
    }

    pub fn meta_mut(&mut self) -> &mut S::UpperMetadata {
        &mut self.meta
    }
}

impl<S: System> From<UpperControlPDU<S>> for UpperPDU<S> {
    fn from(pdu: UpperControlPDU<S>) -> Self {
        UpperPDU::Control(pdu)
    }
}
