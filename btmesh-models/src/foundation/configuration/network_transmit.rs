use crate::Message;
use btmesh_common::opcode::Opcode;
use btmesh_common::{opcode, InsufficientBuffer, ParseError};
use heapless::Vec;

opcode!( CONFIG_NETWORK_TRANSMIT_GET 0x80, 0x23);
opcode!( CONFIG_NETWORK_TRANSMIT_SET 0x80, 0x24);
opcode!( CONFIG_NETWORK_TRANSMIT_STATUS 0x80, 0x25);

/// The Network Transmit state is a composite state that controls the number and timing of the transmissions of Network PDU originating from a node.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NetworkTransmitConfig {
    /// The Network Transmit Count field is a 3-bit value that controls the number of message transmissions of the Network PDU originating from the node.
    pub network_retransmit_count: u8,
    /// The Network Transmit Interval Steps field is a 5-bit value representing the number of 10 millisecond steps
    /// that controls the interval between message transmissions of Network PDUs originating from the node.
    pub network_retransmit_interval_steps: u8,
}

impl Default for NetworkTransmitConfig {
    fn default() -> Self {
        Self {
            network_retransmit_count: 2,
            network_retransmit_interval_steps: 10,
        }
    }
}

impl NetworkTransmitConfig {
    /// Parses parameters into Network Transmit.
    pub fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() < 2 {
            Err(ParseError::InvalidLength)
        } else {
            let network_retransmit_count = parameters[0] & 0b11100000 >> 5;
            let network_retransmit_interval_steps = parameters[0] & 0b00011111;

            Ok(Self {
                network_retransmit_count,
                network_retransmit_interval_steps,
            })
        }
    }

    /// Emits Network Transmit into array of bytes.
    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        xmit.push(
            self.network_retransmit_count & 0b111 << 5
                | self.network_retransmit_interval_steps & 0b11111,
        )
        .map_err(|_| InsufficientBuffer)?;

        Ok(())
    }
}

/// Network Transmit message.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum NetworkTransmitMessage {
    /// Network Transmit Get is an acknowledged message used to get the current Network Transmit state of a node.
    Get,
    /// Network Transmit Set is an acknowledged message used to set the Network Transmit state of a node.
    Set(NetworkTransmitConfig),
    /// Network Transmit Status is an unacknowledged message used to report the current Network Transmit state of a node.
    Status(NetworkTransmitConfig),
}

impl Message for NetworkTransmitMessage {
    fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_NETWORK_TRANSMIT_GET,
            Self::Set(_) => CONFIG_NETWORK_TRANSMIT_SET,
            Self::Status(_) => CONFIG_NETWORK_TRANSMIT_STATUS,
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
impl NetworkTransmitMessage {
    /// Parses parameters into Network Transmit Get message.
    pub fn parse_get(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.is_empty() {
            Ok(Self::Get)
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    /// Parses parameters into Network Transmit Set message.
    pub fn parse_set(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Set(NetworkTransmitConfig::parse(parameters)?))
    }

    /// Parses parameters into Network Transmit Status message.
    pub fn parse_status(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Set(NetworkTransmitConfig::parse(parameters)?))
    }
}
