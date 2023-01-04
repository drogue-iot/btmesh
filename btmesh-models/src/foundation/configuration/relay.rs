use crate::foundation::configuration::ConfigurationMessage;
use crate::Message;
use btmesh_common::opcode::Opcode;
use btmesh_common::{opcode, InsufficientBuffer, ParseError};
use heapless::Vec;

opcode!( CONFIG_RELAY_GET 0x80, 0x26);
opcode!( CONFIG_RELAY_SET 0x80, 0x27);
opcode!( CONFIG_RELAY_STATUS 0x80, 0x28);

/// The Relay state indicates support for the Relay feature.
#[derive(Copy, Clone, Hash, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Relay {
    /// The node support Relay feature that is disabled.
    SupportedDisabled = 0x00,
    /// The node supports Relay feature that is enabled.
    SupportedEnabled = 0x01,
    /// Relay feature is not supported.
    NotSupported = 0x02,
}

impl Relay {
    /// Parses parameters into Relay state.
    pub fn parse(data: u8) -> Result<Self, ParseError> {
        match data {
            0x00 => Ok(Self::SupportedDisabled),
            0x01 => Ok(Self::SupportedEnabled),
            0x02 => Ok(Self::NotSupported),
            _ => Err(ParseError::InvalidValue),
        }
    }

    /// Emits Relay state into array of bytes.
    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        xmit.push(*self as u8).map_err(|_| InsufficientBuffer)?;
        Ok(())
    }
}

/// Configuration of Relay message.
#[derive(Copy, Clone, Debug, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RelayConfig {
    /// Relay state.
    relay: Relay,
    /// Number of retransmissions on advertising bearer for each Network PDU relayed by the node.
    relay_retransmit_count: u8,
    /// Number of 10-millisecond steps between retransmissions.
    relay_retransmit_interval_steps: u8,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            relay: Relay::SupportedEnabled,
            relay_retransmit_count: 1,
            relay_retransmit_interval_steps: 20,
        }
    }
}

impl RelayConfig {
    /// Creates "not supported" relay state.
    pub fn not_supported() -> Self {
        Self {
            relay: Relay::NotSupported,
            relay_retransmit_count: 0,
            relay_retransmit_interval_steps: 0,
        }
    }

    /// Parses parameters into Relay configuration.
    pub fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() < 2 {
            Err(ParseError::InvalidLength)
        } else {
            let relay = Relay::parse(parameters[0])?;
            let relay_retransmit_count = parameters[0] & 0b11100000 >> 5;
            let relay_retransmit_interval_steps = parameters[0] & 0b00011111;

            Ok(Self {
                relay,
                relay_retransmit_count,
                relay_retransmit_interval_steps,
            })
        }
    }

    /// Emits Relay configuration into array of bytes.
    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        self.relay.emit(xmit)?;

        xmit.push(
            self.relay_retransmit_count & 0b111 << 5
                | self.relay_retransmit_interval_steps & 0b11111,
        )
        .map_err(|_| InsufficientBuffer)?;

        Ok(())
    }

    /// Returns Relay state.
    pub fn relay(&self) -> Relay {
        self.relay
    }

    /// Returns Relay retransmit count.
    pub fn retransmit_count(&self) -> u8 {
        self.relay_retransmit_count
    }

    /// Returns Relay retransmit interval.
    pub fn retransmit_interval_steps(&self) -> u8 {
        self.relay_retransmit_interval_steps
    }
}

/// Relay Message.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub enum RelayMessage {
    /// Relay Get is an acknowledged message used to get the current Relay and Relay Retransmit states of a node.
    Get,
    /// The Config Relay Set is an acknowledged message used to set the Relay and Relay Retransmit states of a node.
    Set(RelayConfig),
    /// The Config Relay Status is an unacknowledged message used to report the current Relay and Relay Retransmit states of a node.
    Status(RelayConfig),
}

impl From<RelayMessage> for ConfigurationMessage {
    fn from(inner: RelayMessage) -> Self {
        ConfigurationMessage::Relay(inner)
    }
}

impl Message for RelayMessage {
    fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_RELAY_GET,
            Self::Set(_) => CONFIG_RELAY_SET,
            Self::Status(_) => CONFIG_RELAY_STATUS,
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        match self {
            Self::Get => {}
            Self::Set(inner) => inner.emit(xmit)?,
            Self::Status(inner) => inner.emit(xmit)?,
        }
        Ok(())
    }
}

#[allow(unused)]
impl RelayMessage {
    /// Parses parameters into Relay Get message.
    pub fn parse_get(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.is_empty() {
            Ok(Self::Get)
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    /// Parses parameters into Relay Set message.
    pub fn parse_set(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Set(RelayConfig::parse(parameters)?))
    }

    /// Parses parameters into Relay Status message.
    pub fn parse_status(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Set(RelayConfig::parse(parameters)?))
    }
}
