use crate::{Message, Model};
use btmesh_common::opcode::Opcode;
use btmesh_common::{opcode, InsufficientBuffer, ModelIdentifier, ParseError};
use heapless::Vec;

#[derive(Clone, Debug)]
pub struct GenericBatteryServer;

#[derive(Clone, Debug)]
pub struct GenericBatteryClient;

pub const GENERIC_BATTERY_SERVER: ModelIdentifier = ModelIdentifier::SIG(0x100C);
pub const GENERIC_BATTERY_CLIENT: ModelIdentifier = ModelIdentifier::SIG(0x100D);

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GenericBatteryMessage {
    Get,
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

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GenericBatteryFlags {
    pub presence: GenericBatteryFlagsPresence,
    pub indicator: GenericBatteryFlagsIndicator,
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

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GenericBatteryFlagsPresence {
    NotPresent,
    PresentRemovable,
    PresentNotRemovable,
    Unknown,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GenericBatteryFlagsIndicator {
    LowCritical,
    Low,
    Good,
    Unknown,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GenericBatteryFlagsCharging {
    NotChargeable,
    ChargeableNotCharging,
    ChargeableCharging,
    Unknown,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GenericBatteryStatus {
    pub battery_level: u8,
    pub time_to_discharge: u32,
    pub time_to_charge: u32,
    pub flags: GenericBatteryFlags,
}

impl GenericBatteryStatus {
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
