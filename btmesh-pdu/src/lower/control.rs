use crate::upper::control::UpperControlOpcode;
use crate::System;
use btmesh_common::{InsufficientBuffer, ParseError, SeqZero};
use heapless::Vec;

pub struct UnsegmentedLowerControlPDU<S: System> {
    opcode: UpperControlOpcode,
    parameters: Vec<u8, 88>,
    meta: S::LowerMetadata,
}

impl<S: System> UnsegmentedLowerControlPDU<S> {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        let opcode = UpperControlOpcode::parse(data[0] & 0b01111111)?;
        let parameters = &data[1..];
        Ok(Self {
            opcode,
            parameters: Vec::from_slice(parameters)?,
            meta: Default::default(),
        })
    }

    pub fn opcode(&self) -> UpperControlOpcode {
        self.opcode
    }

    pub fn parameters(&self) -> &[u8] {
        &*self.parameters
    }

    pub fn meta(&self) -> &S::LowerMetadata {
        &self.meta
    }

    pub fn meta_mut(&mut self) -> &mut S::LowerMetadata {
        &mut self.meta
    }
}

pub struct SegmentedLowerControlPDU<S: System> {
    opcode: UpperControlOpcode,
    seq_zero: SeqZero,
    seg_o: u8,
    seg_n: u8,
    segment_m: Vec<u8, 8>,
    meta: S::LowerMetadata,
}

impl<S: System> SegmentedLowerControlPDU<S> {
    pub const SEGMENT_SIZE: usize = 8;

    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        let opcode = UpperControlOpcode::parse(data[0] & 0b01111111)?;
        let seq_zero =
            SeqZero::parse(u16::from_be_bytes([data[1] & 0b01111111, data[2] & 0b11111100]) >> 2)?;
        let seg_o = (u16::from_be_bytes([data[2] & 0b00000011, data[3] & 0b11100000]) >> 5) as u8;
        let seg_n = data[3] & 0b00011111;
        let segment_m = &data[4..];
        Ok(Self {
            opcode,
            seq_zero,
            seg_o,
            seg_n,
            segment_m: Vec::from_slice(segment_m)?,
            meta: Default::default(),
        })
    }

    pub fn new(opcode: UpperControlOpcode, seq_zero: SeqZero, seg_o: u8, seg_n: u8, segment_m: &[u8]) -> Result<Self, InsufficientBuffer> {
        Ok( Self {
            opcode,
            seq_zero,
            seg_o,
            seg_n,
            segment_m: Vec::from_slice(segment_m)?,
            meta: Default::default()
        } )

    }

    pub fn opcode(&self) -> UpperControlOpcode {
        self.opcode
    }

    pub fn seq_zero(&self) -> SeqZero {
        self.seq_zero
    }

    pub fn seg_o(&self) -> u8 {
        self.seg_o
    }

    pub fn seg_n(&self) -> u8 {
        self.seg_n
    }

    pub fn segment_m(&self) -> &[u8] {
        &*self.segment_m
    }

    pub fn meta(&self) -> &S::LowerMetadata {
        &self.meta
    }

    pub fn meta_mut(&mut self) -> &mut S::LowerMetadata {
        &mut self.meta
    }
}
