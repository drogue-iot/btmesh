use crate::provisioned::upper::UpperPDU;
use crate::provisioned::System;
use btmesh_common::mic::{SzMic, TransMic};
use btmesh_common::{InsufficientBuffer, ParseError};
use heapless::Vec;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[allow(dead_code)]
pub struct UpperAccessPDU<S: System> {
    payload: Vec<u8, 380>,
    transmic: TransMic,
    meta: S::UpperMetadata,
}

impl<S: System> Clone for UpperAccessPDU<S> {
    fn clone(&self) -> Self {
        Self {
            payload: self.payload.clone(),
            transmic: self.transmic,
            meta: self.meta.clone(),
        }
    }
}

impl<S: System> UpperAccessPDU<S> {
    pub fn new(
        payload: &[u8],
        transmic: TransMic,
        meta: S::UpperMetadata,
    ) -> Result<Self, InsufficientBuffer> {
        Ok(Self {
            payload: Vec::from_slice(payload)?,
            transmic,
            meta,
        })
    }

    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        xmit.extend_from_slice(&self.payload)?;
        xmit.extend_from_slice(self.transmic.as_ref())?;
        Ok(())
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn transmic(&self) -> TransMic {
        self.transmic
    }

    pub fn meta(&self) -> &S::UpperMetadata {
        &self.meta
    }

    pub fn meta_mut(&mut self) -> &mut S::UpperMetadata {
        &mut self.meta
    }
}

impl<S: System> From<UpperAccessPDU<S>> for UpperPDU<S> {
    fn from(pdu: UpperAccessPDU<S>) -> Self {
        UpperPDU::Access(pdu)
    }
}

impl<S: System> UpperAccessPDU<S> {
    pub fn parse(data: &[u8], szmic: &SzMic, meta: S::UpperMetadata) -> Result<Self, ParseError> {
        let (payload, transmic) = data.split_at(data.len() - szmic.size());

        Ok(Self {
            payload: Vec::from_slice(payload)?,
            transmic: TransMic::parse(transmic)?,
            meta,
        })
    }
}
