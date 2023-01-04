use crate::foundation::configuration::ConfigurationMessage;
use crate::Message;
use btmesh_common::opcode::Opcode;
use btmesh_common::{opcode, InsufficientBuffer, ParseError};
use heapless::Vec;

opcode!( CONFIG_NODE_RESET 0x80, 0x49 );
opcode!( CONFIG_NODE_RESET_STATUS 0x80, 0x4A );

/// Node Reset message.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub enum NodeResetMessage {
    /// Node Reset is an acknowledged message used to reset a node (other than a Provisioner) and remove it from the network.
    Reset,
    /// Node Reset Status is an unacknowledged message used to acknowledge that an element has received a Config Node Reset message.
    Status,
}

#[allow(unused)]
impl NodeResetMessage {
    /// Parses parameters into Node Reset message.
    pub fn parse_reset(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.is_empty() {
            Ok(Self::Reset)
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    /// Parses parameters into Node Reset Status message.
    pub fn parse_status(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.is_empty() {
            Ok(Self::Status)
        } else {
            Err(ParseError::InvalidLength)
        }
    }
}

impl From<NodeResetMessage> for ConfigurationMessage {
    fn from(inner: NodeResetMessage) -> Self {
        ConfigurationMessage::NodeReset(inner)
    }
}

impl Message for NodeResetMessage {
    fn opcode(&self) -> Opcode {
        match self {
            Self::Reset => CONFIG_NODE_RESET,
            Self::Status => CONFIG_NODE_RESET_STATUS,
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        _xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        Ok(())
    }
}
