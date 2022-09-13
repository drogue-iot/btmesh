#![cfg_attr(not(test), no_std)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(associated_type_defaults)]
#![allow(dead_code)]

#[allow(unused_imports)]
use crate::foundation::configuration::{CONFIGURATION_CLIENT, CONFIGURATION_SERVER};
#[allow(unused_imports)]
use crate::{
    generic::{
        battery::{GENERIC_BATTERY_CLIENT, GENERIC_BATTERY_SERVER},
        onoff::{GENERIC_ONOFF_CLIENT, GENERIC_ONOFF_SERVER},
    },
    sensor::{SENSOR_CLIENT, SENSOR_SERVER, SENSOR_SETUP_SERVER},
};
use btmesh_common::opcode::Opcode;
pub use btmesh_common::{InsufficientBuffer, ModelIdentifier, ParseError};
use heapless::Vec;

//pub mod firmware;
pub mod foundation;
pub mod generic;
pub mod sensor;

#[cfg(feature = "defmt")]
pub trait Message: defmt::Format {
    fn opcode(&self) -> Opcode;
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer>;
}

#[cfg(not(feature = "defmt"))]
pub trait Message {
    fn opcode(&self) -> Opcode;
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer>;
}

pub trait Model {
    const IDENTIFIER: ModelIdentifier;
    const SUPPORTS_SUBSCRIPTION: bool = true;
    const SUPPORTS_PUBLICATION: bool = true;
    type Message: Message;

    fn parse(opcode: &Opcode, parameters: &[u8]) -> Result<Option<Self::Message>, ParseError>;
}

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Status {
    Success = 0x00,
    InvalidAddress = 0x01,
    InvalidModel = 0x02,
    InvalidAppKeyIndex = 0x03,
    InvalidNetKeyIndex = 0x04,
    InsufficientResources = 0x05,
    KeyIndexAlreadyStored = 0x06,
    InvalidPublishParameters = 0x07,
    NotASubscribeModel = 0x08,
    StorageFailure = 0x09,
    FeatureNotSupported = 0x0A,
    CannotUpdate = 0x0B,
    CannotRemove = 0x0C,
    CannotBind = 0x0D,
    TemporarilyUnableToChangeState = 0x0E,
    CannotSet = 0x0F,
    UnspecifiedError = 0x10,
    InvalidBinding = 0x11,
}

impl TryFrom<u8> for Status {
    type Error = ParseError;
    fn try_from(status: u8) -> Result<Self, Self::Error> {
        match status {
            0x00 => Ok(Self::Success),
            0x01 => Ok(Self::InvalidAddress),
            0x02 => Ok(Self::InvalidModel),
            0x03 => Ok(Self::InvalidAppKeyIndex),
            0x04 => Ok(Self::InvalidNetKeyIndex),
            0x05 => Ok(Self::InsufficientResources),
            0x06 => Ok(Self::KeyIndexAlreadyStored),
            0x07 => Ok(Self::InvalidPublishParameters),
            0x08 => Ok(Self::NotASubscribeModel),
            0x09 => Ok(Self::StorageFailure),
            0x0A => Ok(Self::FeatureNotSupported),
            0x0B => Ok(Self::CannotUpdate),
            0x0C => Ok(Self::CannotRemove),
            0x0D => Ok(Self::CannotBind),
            0x0E => Ok(Self::TemporarilyUnableToChangeState),
            0x0F => Ok(Self::CannotSet),
            0x10 => Ok(Self::UnspecifiedError),
            0x11 => Ok(Self::InvalidBinding),
            _ => Err(ParseError::InvalidValue),
        }
    }
}
