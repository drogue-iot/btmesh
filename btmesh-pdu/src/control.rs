use crate::upper::control::ControlOpcode;
use crate::{Message, System};
use btmesh_common::InsufficientBuffer;
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
}

impl<S: System> From<ControlMessage<S>> for Message<S> {
    fn from(inner: ControlMessage<S>) -> Self {
        Self::Control(inner)
    }
}
