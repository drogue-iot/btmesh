use crate::foundation::configuration::ConfigurationMessage;
use crate::Message;
use btmesh_common::opcode::Opcode;
use btmesh_common::{opcode, InsufficientBuffer, ParseError, Ttl};
use heapless::Vec;

opcode!( CONFIG_DEFAULT_TTL_GET 0x80, 0x0C );
opcode!( CONFIG_DEFAULT_TTL_SET 0x80, 0x0D );
opcode!( CONFIG_DEFAULT_TTL_STATUS 0x80, 0x0E );

/// Default TTL message.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub enum DefaultTTLMessage {
    /// Default TTL Get is an acknowledged message used to get the current Default TTL state of a node.
    Get,
    /// Default TTL Set is an acknowledged message used to set the Default TTL state of a node.
    Set(Ttl),
    /// Default TTL Status is an unacknowledged message used to report the current Default TTL state of a node.
    Status(Ttl),
}

impl From<DefaultTTLMessage> for ConfigurationMessage {
    fn from(inner: DefaultTTLMessage) -> Self {
        Self::DefaultTTL(inner)
    }
}

#[allow(unused)]
impl Message for DefaultTTLMessage {
    fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_DEFAULT_TTL_GET,
            Self::Set(_) => CONFIG_DEFAULT_TTL_SET,
            Self::Status(_) => CONFIG_DEFAULT_TTL_STATUS,
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        match self {
            Self::Get => {}
            Self::Set(ttl) => xmit.push(ttl.value()).map_err(|_| InsufficientBuffer)?,
            Self::Status(ttl) => xmit.push(ttl.value()).map_err(|_| InsufficientBuffer)?,
        }
        Ok(())
    }
}

impl DefaultTTLMessage {
    /// Parses byte array into Default TTL Set message.
    pub fn parse_get(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.is_empty() {
            Ok(Self::Get)
        } else {
            Err(ParseError::InvalidLength)
        }
    }
    /// Parses byte array into Default TTL Get message.
    pub fn parse_set(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() == 1 {
            Ok(Self::Set(Ttl::parse(parameters[0])?))
        } else {
            Err(ParseError::InvalidLength)
        }
    }
}
