//! Implementation of the Generic Battery model.

use crate::{Message, Model};
use btmesh_common::opcode::Opcode;
use btmesh_common::{opcode, InsufficientBuffer, ModelIdentifier, ParseError};
use heapless::Vec;

/// Generic Battery Server message.
#[derive(Clone, Debug)]
pub struct GenericBatteryServer;

/// Generic Battery Client message.
#[derive(Clone, Debug)]
pub struct GenericBatteryClient;

/// Generic Battery Server model Identifier
pub const GENERIC_BATTERY_SERVER: ModelIdentifier = ModelIdentifier::SIG(0x100C);
/// Generic Battery Client model Identifier
pub const GENERIC_BATTERY_CLIENT: ModelIdentifier = ModelIdentifier::SIG(0x100D);

/// Generic battery message.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GenericBatteryMessage {
    /// Get message.
    Get,
    /// Status message.
    Status(GenericBatteryStatus),
}

impl Message for GenericBatteryMessage {
    fn opcode(&self) -> Opcode {
        match self {
            Self::Get => GENERIC_BATTERY_GET,
            Self::Status(_) => GENERIC_BATTERY_STATUS,
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        match self {
            Self::Get => Ok(()),
            Self::Status(inner) => inner.emit_parameters(xmit),
        }
    }
}

impl Model for GenericBatteryClient {
    const IDENTIFIER: ModelIdentifier = GENERIC_BATTERY_CLIENT;
    type Message = GenericBatteryMessage;

    fn parse(opcode: &Opcode, parameters: &[u8]) -> Result<Option<Self::Message>, ParseError> {
        match *opcode {
            GENERIC_BATTERY_STATUS => Ok(Some(GenericBatteryMessage::Status(
                GenericBatteryStatus::parse(parameters)?,
            ))),
            _ => Ok(None),
        }
    }
}

impl Model for GenericBatteryServer {
    const IDENTIFIER: ModelIdentifier = GENERIC_BATTERY_SERVER;
    type Message = GenericBatteryMessage;

    fn parse(opcode: &Opcode, _parameters: &[u8]) -> Result<Option<Self::Message>, ParseError> {
        match *opcode {
            GENERIC_BATTERY_GET => Ok(Some(GenericBatteryMessage::Get)),
            _ => Ok(None),
        }
    }
}

opcode!( GENERIC_BATTERY_GET 0x82, 0x23 );
opcode!( GENERIC_BATTERY_STATUS 0x82, 0x24 );

/// The Generic Battery Flags state is a concatenation of four 2-bit bit fields: Presence, Indicator, Charging, and Serviceability.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GenericBatteryFlags {
    /// Generic Battery Flags Presence.
    pub presence: GenericBatteryFlagsPresence,
    /// Generic Battery Flags Indicator.
    pub indicator: GenericBatteryFlagsIndicator,
    /// Generic Battery Flags Charging.
    pub charging: GenericBatteryFlagsCharging,
}

impl GenericBatteryFlags {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        let mut value: u8 = 0;
        value |= match self.presence {
            GenericBatteryFlagsPresence::NotPresent => 0b00,
            GenericBatteryFlagsPresence::PresentRemovable => 0b01,
            GenericBatteryFlagsPresence::PresentNotRemovable => 0b10,
            GenericBatteryFlagsPresence::Unknown => 0b11,
        } << 4;

        value |= match self.indicator {
            GenericBatteryFlagsIndicator::LowCritical => 0b00,
            GenericBatteryFlagsIndicator::Low => 0b01,
            GenericBatteryFlagsIndicator::Good => 0b10,
            GenericBatteryFlagsIndicator::Unknown => 0b11,
        } << 2;

        value |= match self.charging {
            GenericBatteryFlagsCharging::NotChargeable => 0b00,
            GenericBatteryFlagsCharging::ChargeableNotCharging => 0b01,
            GenericBatteryFlagsCharging::ChargeableCharging => 0b10,
            GenericBatteryFlagsCharging::Unknown => 0b11,
        };

        xmit.push(value).map_err(|_| InsufficientBuffer)?;
        Ok(())
    }

    fn parse(v: u8) -> Self {
        let charging = match v & 0b11 {
            0b00 => GenericBatteryFlagsCharging::NotChargeable,
            0b01 => GenericBatteryFlagsCharging::ChargeableNotCharging,
            0b10 => GenericBatteryFlagsCharging::ChargeableCharging,
            0b11 => GenericBatteryFlagsCharging::Unknown,
            _ => panic!("impossible!"),
        };

        let indicator = match (v >> 2) & 0b11 {
            0b00 => GenericBatteryFlagsIndicator::LowCritical,
            0b01 => GenericBatteryFlagsIndicator::Low,
            0b10 => GenericBatteryFlagsIndicator::Good,
            0b11 => GenericBatteryFlagsIndicator::Unknown,
            _ => panic!("impossible!"),
        };

        let presence = match (v >> 4) & 0b11 {
            0b00 => GenericBatteryFlagsPresence::NotPresent,
            0b01 => GenericBatteryFlagsPresence::PresentRemovable,
            0b10 => GenericBatteryFlagsPresence::PresentNotRemovable,
            0b11 => GenericBatteryFlagsPresence::Unknown,
            _ => panic!("impossible!"),
        };

        Self {
            charging,
            indicator,
            presence,
        }
    }
}

/// The Generic Battery Flags Presence state bit field indicates presence of a battery.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GenericBatteryFlagsPresence {
    /// The battery is not present.
    NotPresent,
    /// The battery is present and is removable.
    PresentRemovable,
    /// The battery is present and is non-removable.
    PresentNotRemovable,
    /// The battery presence is unknown.
    Unknown,
}

/// The Generic Battery Flags Indicator state bit field indicates the charge level of a battery.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GenericBatteryFlagsIndicator {
    /// The battery charge is Critically Low Level.
    LowCritical,
    /// The battery charge is Low Level.
    Low,
    /// The battery charge is Good Level.
    Good,
    /// The battery charge is unknown.
    Unknown,
}

/// The Generic Battery Flags Charging state bit field indicates whether a battery is charging.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GenericBatteryFlagsCharging {
    /// The battery is not chargeable.
    NotChargeable,
    /// The battery is chargeable and is not charging.
    ChargeableNotCharging,
    /// The battery is chargeable and is charging.
    ChargeableCharging,
    /// The battery charging state is unknown.
    Unknown,
}

/// Generic Battery Status is an unacknowledged message used to report the Generic Battery state of an element.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GenericBatteryStatus {
    /// The value of the Generic Battery Level state.
    pub battery_level: u8,
    /// The value of the Generic Battery Time to Discharge state.
    pub time_to_discharge: u32,
    /// The value of the Generic Battery Time to Charge state.
    pub time_to_charge: u32,
    /// The value of the Generic Battery Flags state.
    pub flags: GenericBatteryFlags,
}

impl GenericBatteryStatus {
    /// Creates new battery status.
    pub fn new(
        battery_level: u8,
        time_to_discharge: u32,
        time_to_charge: u32,
        flags: GenericBatteryFlags,
    ) -> Self {
        Self {
            battery_level,
            time_to_discharge,
            time_to_charge,
            flags,
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        xmit.push(self.battery_level)
            .map_err(|_| InsufficientBuffer)?;
        xmit.extend_from_slice(&self.time_to_discharge.to_be_bytes()[1..])
            .map_err(|_| InsufficientBuffer)?;
        xmit.extend_from_slice(&self.time_to_charge.to_be_bytes()[1..])
            .map_err(|_| InsufficientBuffer)?;
        self.flags.emit_parameters(xmit)?;
        Ok(())
    }

    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 8 {
            let battery_level = parameters[0];

            let time_to_discharge =
                u32::from_be_bytes([0, parameters[1], parameters[2], parameters[3]]);

            let time_to_charge =
                u32::from_be_bytes([0, parameters[4], parameters[5], parameters[6]]);

            let flags = GenericBatteryFlags::parse(parameters[7]);

            Ok(Self {
                battery_level,
                time_to_discharge,
                time_to_charge,
                flags,
            })
        } else {
            Err(ParseError::InvalidLength)
        }
    }
}
