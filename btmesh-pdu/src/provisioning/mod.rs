use btmesh_common::{InsufficientBuffer, ParseError};
use heapless::Vec;

pub mod advertising;
pub mod generic;

#[derive(Clone)]
pub enum ProvisioningPDU {}

impl ProvisioningPDU {
    pub fn parse(_data: &[u8]) -> Result<Self, ParseError> {
        todo!()
    }

    pub fn emit<const N: usize>(&self, _xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        todo!()
    }
}
