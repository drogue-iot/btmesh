use crate::provisioned::lower::BlockAck;
use crate::provisioned::upper::control::ControlOpcode;
use crate::provisioned::{Message, System};
use btmesh_common::{InsufficientBuffer, ParseError};
use heapless::Vec;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ControlMessage<S: System> {
    opcode: ControlOpcode,
    parameters: Vec<u8, 256>,
    meta: S::ControlMetadata,
}

impl<S: System> ControlMessage<S> {
    pub fn new(
        opcode: ControlOpcode,
        parameters: &[u8],
        meta: S::ControlMetadata,
    ) -> Result<Self, InsufficientBuffer> {
        Ok(Self {
            opcode,
            parameters: Vec::from_slice(parameters)?,
            meta,
        })
    }

    pub fn opcode(&self) -> ControlOpcode {
        self.opcode
    }

    pub fn parameters(&self) -> &[u8] {
        &self.parameters
    }
}

impl<S: System> From<ControlMessage<S>> for Message<S> {
    fn from(inner: ControlMessage<S>) -> Self {
        Self::Control(inner)
    }
}

impl<S: System> TryFrom<&ControlMessage<S>> for BlockAck {
    type Error = ParseError;

    fn try_from(value: &ControlMessage<S>) -> Result<Self, Self::Error> {
        if let ControlOpcode::SegmentAcknowledgement = value.opcode {
            Ok(BlockAck::parse(&value.parameters)?)
        } else {
            Err(ParseError::InvalidPDUFormat)
        }
    }
}
