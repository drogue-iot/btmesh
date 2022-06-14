use crate::System;
use btmesh_common::mic::SzMic;
use btmesh_common::{Aid, InsufficientBuffer, ParseError, SeqZero};
use heapless::Vec;

pub struct UnsegmentedLowerAccessPDU<S: System> {
    akf_aid: Option<Aid>,
    upper_pdu: Vec<u8, 120>,
    meta: S::LowerMetadata,
}

impl<S: System> UnsegmentedLowerAccessPDU<S> {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        let akf_aid = Aid::parse(data[0])?;
        Ok(Self {
            akf_aid,
            upper_pdu: Vec::from_slice(&data[1..])?,
            meta: Default::default(),
        })
    }

    pub fn akf(&self) -> bool {
        self.akf_aid.is_some()
    }

    pub fn aid(&self) -> Option<Aid> {
        self.akf_aid
    }

    pub fn upper_pdu(&self) -> &[u8] {
        &*self.upper_pdu
    }

    pub fn meta(&self) -> &S::LowerMetadata {
        &self.meta
    }

    pub fn meta_mut(&mut self) -> &mut S::LowerMetadata {
        &mut self.meta
    }
}

pub struct SegmentedLowerAccessPDU<S: System> {
    akf_aid: Option<Aid>,
    szmic: SzMic,
    seq_zero: SeqZero,
    seg_o: u8,
    seg_n: u8,
    segment_m: Vec<u8, 12>,
    meta: S::LowerMetadata,
}

impl<S: System> SegmentedLowerAccessPDU<S> {
    pub const SEGMENT_SIZE: usize = 12;

    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        let akf_aid = Aid::parse(data[0])?;
        let szmic = SzMic::parse(data[1] & 0b10000000);
        let seq_zero =
            SeqZero::parse(u16::from_be_bytes([data[1] & 0b01111111, data[2] & 0b11111100]) >> 2)?;
        let seg_o = (u16::from_be_bytes([data[2] & 0b00000011, data[3] & 0b11100000]) >> 5) as u8;
        let seg_n = data[3] & 0b00011111;
        let segment_m = Vec::from_slice(&data[4..])?;
        Ok(Self {
            akf_aid,
            szmic,
            seq_zero,
            seg_o,
            seg_n,
            segment_m,
            meta: Default::default(),
        })
    }

    pub fn new(akf_aid: Option<Aid>, szmic: SzMic, seq_zero: SeqZero, seg_o: u8, seg_n: u8, segment_m: &[u8]) -> Result<Self, InsufficientBuffer> {
        Ok( Self {
            akf_aid,
            szmic,
            seq_zero,
            seg_o,
            seg_n,
            segment_m: Vec::from_slice(segment_m)?,
            meta: Default::default()
        } )
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

    pub fn szmic(&self) -> SzMic {
        self.szmic
    }

    pub fn meta(&self) -> &S::LowerMetadata {
        &self.meta
    }

    pub fn meta_mut(&mut self) -> &mut S::LowerMetadata {
        &mut self.meta
    }
}
