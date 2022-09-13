use crate::foundation::configuration::ConfigurationMessage;
use crate::Message;
use btmesh_common::opcode::Opcode;
use btmesh_common::{opcode, InsufficientBuffer, ParseError, Ttl};
use heapless::Vec;

opcode!( CONFIG_DEFAULT_TTL_GET 0x80, 0x0C );
opcode!( CONFIG_DEFAULT_TTL_SET 0x80, 0x0D );
opcode!( CONFIG_DEFAULT_TTL_STATUS 0x80, 0x0E );

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub enum DefaultTTLMessage {
    Get,
    Set(Ttl),
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
    pub fn parse_get(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.is_empty() {
            Ok(Self::Get)
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    pub fn parse_set(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() == 1 {
            Ok(Self::Set(Ttl::parse(parameters[0])?))
        } else {
            Err(ParseError::InvalidLength)
        }
    }
}
