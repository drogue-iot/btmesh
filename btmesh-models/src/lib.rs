#![cfg_attr(not(test), no_std)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(associated_type_defaults)]
#![allow(dead_code)]

#[allow(unused_imports)]
use crate::foundation::configuration::{CONFIGURATION_CLIENT, CONFIGURATION_SERVER};
use crate::opcode::Opcode;
#[allow(unused_imports)]
use crate::{
    generic::{
        battery::{GENERIC_BATTERY_CLIENT, GENERIC_BATTERY_SERVER},
        onoff::{GENERIC_ONOFF_CLIENT, GENERIC_ONOFF_SERVER},
    },
    sensor::{SENSOR_CLIENT, SENSOR_SERVER, SENSOR_SETUP_SERVER},
};
use btmesh_common::{InsufficientBuffer, ModelIdentifier, ParseError};
use heapless::Vec;

pub mod opcode;
//pub mod firmware;
pub mod foundation;
pub mod generic;
pub mod sensor;

#[cfg(feature = "defmt")]
impl defmt::Format for ModelIdentifier {
    fn format(&self, fmt: defmt::Formatter) {
        match *self {
            CONFIGURATION_SERVER => {
                defmt::write!(fmt, "Configuration Server (0x0000)");
            }
            CONFIGURATION_CLIENT => {
                defmt::write!(fmt, "Configuration Client (0x0001)");
            }
            GENERIC_ONOFF_SERVER => {
                defmt::write!(fmt, "Generic OnOff Server (0x1000)");
            }
            GENERIC_ONOFF_CLIENT => {
                defmt::write!(fmt, "Generic OnOff Client (0x1001)");
            }
            GENERIC_BATTERY_SERVER => {
                defmt::write!(fmt, "Generic Battery Server (0x100C)");
            }
            SENSOR_SERVER => {
                defmt::write!(fmt, "Sensor Server (0x1100)");
            }
            SENSOR_SETUP_SERVER => {
                defmt::write!(fmt, "Sensor Setup Server (0x1101)");
            }
            SENSOR_CLIENT => {
                defmt::write!(fmt, "Sensor Client (0x1102)");
            }
            GENERIC_BATTERY_CLIENT => {
                defmt::write!(fmt, "Generic Battery Client (0x100D)");
            }
            ModelIdentifier::SIG(id) => match id {
                _ => {
                    defmt::write!(fmt, "SIG(0x{=u16:04x})", id);
                }
            },
            ModelIdentifier::Vendor(company_id, model_id) => {
                defmt::write!(fmt, "Vendor({}, 0x{=u16:04x})", company_id, model_id);
            }
        }
    }
}

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

pub enum HandlerError {
    Unhandled,
    NotConnected,
}

pub trait Model {
    const IDENTIFIER: ModelIdentifier;
    const SUPPORTS_SUBSCRIPTION: bool = true;
    const SUPPORTS_PUBLICATION: bool = true;
    type Message<'m>: Message;

    fn parse(opcode: Opcode, parameters: &[u8]) -> Result<Option<Self::Message<'_>>, ParseError>;
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
