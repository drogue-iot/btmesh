//! Bluetooth mesh [models]
//!
//! [models]: https://www.bluetooth.com/specifications/specs/mesh-model-1-0-1/

#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]
#![warn(missing_docs)]

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

/// Foundation models.
pub mod foundation;
/// Generic models.
pub mod generic;
/// Sensor models.
pub mod sensor;

#[cfg(feature = "defmt")]
pub trait Message: defmt::Format {
    fn opcode(&self) -> Opcode;
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer>;
}

/// Model message
#[cfg(not(feature = "defmt"))]
pub trait Message {
    /// Opcode of the message.
    fn opcode(&self) -> Opcode;
    /// Decodes the message and appends it to the provided array of bytes.
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer>;
}

/// Bluetooth mesh model.
pub trait Model {
    /// Model identifier.
    const IDENTIFIER: ModelIdentifier;
    /// Does model supports subscriptions.
    const SUPPORTS_SUBSCRIPTION: bool = true;
    /// Does model support publications.
    const SUPPORTS_PUBLICATION: bool = true;
    /// Message type of the model.
    type Message: Message;

    /// Parses bytes and returns the model message.
    fn parse(opcode: &Opcode, parameters: &[u8]) -> Result<Option<Self::Message>, ParseError>;
}

/// Status code of model messages.
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Status {
    /// Operation successful.
    Success = 0x00,
    /// Invalid address used.
    InvalidAddress = 0x01,
    /// Invalid model used.
    InvalidModel = 0x02,
    /// Invalid application key used.
    InvalidAppKeyIndex = 0x03,
    /// Invalid network key used.
    InvalidNetKeyIndex = 0x04,
    /// Insufficient Resources.
    InsufficientResources = 0x05,
    /// Key index already stored.
    KeyIndexAlreadyStored = 0x06,
    /// Invalid publish parameters.
    InvalidPublishParameters = 0x07,
    /// Model doesn't support subscriptions.
    NotASubscribeModel = 0x08,
    /// General storage failure.
    StorageFailure = 0x09,
    /// Requested feature is not supported.
    FeatureNotSupported = 0x0A,
    /// Unable to update.
    CannotUpdate = 0x0B,
    /// Unable to remove.
    CannotRemove = 0x0C,
    /// Unable to bind.
    CannotBind = 0x0D,
    /// Unable to change the state.
    TemporarilyUnableToChangeState = 0x0E,
    /// Unable to set.
    CannotSet = 0x0F,
    /// Generic unspecified error.
    UnspecifiedError = 0x10,
    /// Invalid binding used.
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
