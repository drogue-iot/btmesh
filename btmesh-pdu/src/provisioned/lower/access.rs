use crate::provisioned::System;
use btmesh_common::crypto::application::Aid;
use btmesh_common::mic::SzMic;
use btmesh_common::{InsufficientBuffer, ParseError, SeqZero};
use heapless::Vec;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UnsegmentedLowerAccessPDU<S: System> {
    akf_aid: Option<Aid>,
    upper_pdu: Vec<u8, 120>,
    meta: S::LowerMetadata,
}

impl<S: System> UnsegmentedLowerAccessPDU<S> {
    pub fn new(
        akf_aid: Option<Aid>,
        upper_pdu: &[u8],
        meta: S::LowerMetadata,
    ) -> Result<Self, InsufficientBuffer> {
        Ok(Self {
            akf_aid,
            upper_pdu: Vec::from_slice(upper_pdu)?,
            meta,
        })
    }
    pub fn parse(data: &[u8], meta: S::LowerMetadata) -> Result<Self, ParseError> {
        let akf_aid = Aid::parse(data[0])?;
        Ok(Self {
            akf_aid,
            upper_pdu: Vec::from_slice(&data[1..])?,
            meta,
        })
    }

    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        match self.akf_aid {
            None => xmit.push(0)?,
            Some(aid) => aid.emit(xmit)?,
        }
        xmit.extend_from_slice(&self.upper_pdu)?;
        Ok(())
    }

    pub fn akf(&self) -> bool {
        self.akf_aid.is_some()
    }

    pub fn aid(&self) -> Option<Aid> {
        self.akf_aid
    }

    pub fn upper_pdu(&self) -> &[u8] {
        &self.upper_pdu
    }

    pub fn meta(&self) -> &S::LowerMetadata {
        &self.meta
    }

    pub fn meta_mut(&mut self) -> &mut S::LowerMetadata {
        &mut self.meta
    }
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SegmentedLowerAccessPDU<S: System = ()> {
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

    pub fn parse(data: &[u8], meta: S::LowerMetadata) -> Result<Self, ParseError> {
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
            meta,
        })
    }

    #[allow(clippy::identity_op)]
    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        let cur = xmit.len();
        match self.akf_aid {
            None => xmit.push(0)?,
            Some(aid) => aid.emit(xmit)?,
        }
        // set the SEGMENTED bit.
        xmit[cur] |= 0b10000000;

        let mut header = [0; 3];
        match self.szmic {
            // small szmic + first 7 bits of seq_zero
            SzMic::Bit32 => {
                header[0] = 0b00000000 | ((self.seq_zero & 0b1111111000000) >> 6) as u8;
            }
            // big szmic + first 7 bits of seq_zero
            SzMic::Bit64 => {
                header[0] = 0b10000000 | ((self.seq_zero & 0b1111111000000) >> 6) as u8;
            }
        }
        // last 6 bits of seq_zero + first 2 bits of seg_o
        header[1] =
            ((self.seq_zero & 0b111111) << 2) as u8 | ((self.seg_o & 0b00011000) >> 2) as u8;
        header[2] = ((self.seg_o & 0b00000111) << 5) | (self.seg_n & 0b00011111);
        xmit.extend_from_slice(&header)?;
        xmit.extend_from_slice(&self.segment_m)?;
        Ok(())
    }

    pub fn new(
        akf_aid: Option<Aid>,
        szmic: SzMic,
        seq_zero: SeqZero,
        seg_o: u8,
        seg_n: u8,
        segment_m: &[u8],
        meta: S::LowerMetadata,
    ) -> Result<Self, InsufficientBuffer> {
        Ok(Self {
            akf_aid,
            szmic,
            seq_zero,
            seg_o,
            seg_n,
            segment_m: Vec::from_slice(segment_m)?,
            meta,
        })
    }

    pub fn aid(&self) -> Option<Aid> {
        self.akf_aid
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
        &self.segment_m
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
