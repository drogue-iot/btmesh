pub mod access;
pub mod control;

use crate::lower::access::{SegmentedLowerAccessPDU, UnsegmentedLowerAccessPDU};
use crate::lower::control::{SegmentedLowerControlPDU, UnsegmentedLowerControlPDU};
use crate::network::CleartextNetworkPDU;
use crate::System;
use btmesh_common::address::UnicastAddress;
use btmesh_common::mic::SzMic;
use btmesh_common::{Aid, Ctl, InsufficientBuffer, ParseError, SeqZero};
use heapless::Vec;
use std::marker::PhantomData;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LowerPDU<S: System = ()> {
    Unsegmented(UnsegmentedLowerPDU<S>),
    Segmented(SegmentedLowerPDU<S>),
}

impl<S: System> LowerPDU<S> {
    pub fn meta(&self) -> &S::LowerMetadata {
        match self {
            LowerPDU::Unsegmented(pdu) => pdu.meta(),
            LowerPDU::Segmented(pdu) => pdu.meta(),
        }
    }

    pub fn meta_mut(&mut self) -> &mut S::LowerMetadata {
        match self {
            LowerPDU::Unsegmented(pdu) => pdu.meta_mut(),
            LowerPDU::Segmented(pdu) => pdu.meta_mut(),
        }
    }
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum UnsegmentedLowerPDU<S: System> {
    Access(UnsegmentedLowerAccessPDU<S>),
    Control(UnsegmentedLowerControlPDU<S>),
}

impl<S: System> UnsegmentedLowerPDU<S> {
    pub fn meta(&self) -> &S::LowerMetadata {
        match self {
            UnsegmentedLowerPDU::Access(pdu) => pdu.meta(),
            UnsegmentedLowerPDU::Control(pdu) => pdu.meta(),
        }
    }

    pub fn meta_mut(&mut self) -> &mut S::LowerMetadata {
        match self {
            UnsegmentedLowerPDU::Access(pdu) => pdu.meta_mut(),
            UnsegmentedLowerPDU::Control(pdu) => pdu.meta_mut(),
        }
    }
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SegmentedLowerPDU<S: System> {
    Access(SegmentedLowerAccessPDU<S>),
    Control(SegmentedLowerControlPDU<S>),
}

impl<S: System> SegmentedLowerPDU<S> {
    pub fn meta(&self) -> &S::LowerMetadata {
        match self {
            SegmentedLowerPDU::Access(pdu) => pdu.meta(),
            SegmentedLowerPDU::Control(pdu) => pdu.meta(),
        }
    }

    pub fn meta_mut(&mut self) -> &mut S::LowerMetadata {
        match self {
            SegmentedLowerPDU::Access(pdu) => pdu.meta_mut(),
            SegmentedLowerPDU::Control(pdu) => pdu.meta_mut(),
        }
    }

    pub fn seq_zero(&self) -> SeqZero {
        match self {
            SegmentedLowerPDU::Access(pdu) => pdu.seq_zero(),
            SegmentedLowerPDU::Control(pdu) => pdu.seq_zero(),
        }
    }
    pub fn seg_o(&self) -> u8 {
        match self {
            SegmentedLowerPDU::Access(pdu) => pdu.seg_o(),
            SegmentedLowerPDU::Control(pdu) => pdu.seg_o(),
        }
    }
    pub fn seg_n(&self) -> u8 {
        match self {
            SegmentedLowerPDU::Access(pdu) => pdu.seg_n(),
            SegmentedLowerPDU::Control(pdu) => pdu.seg_n(),
        }
    }
}

impl<S: System> LowerPDU<S> {
    pub fn parse(network_pdu: &CleartextNetworkPDU<S>) -> Result<Self, ParseError> {
        let data = network_pdu.transport_pdu();

        if data.len() >= 2 {
            let seg = data[0] & 0b10000000 != 0;

            match (network_pdu.ctl(), seg) {
                (Ctl::Access, false) => Ok(LowerPDU::Unsegmented(UnsegmentedLowerPDU::Access(
                    UnsegmentedLowerAccessPDU::parse(&data[1..])?,
                ))),
                (Ctl::Access, true) => Ok(LowerPDU::Segmented(SegmentedLowerPDU::Access(
                    SegmentedLowerAccessPDU::parse(&data[1..])?,
                ))),
                (Ctl::Control, false) => Ok(LowerPDU::Unsegmented(UnsegmentedLowerPDU::Control(
                    UnsegmentedLowerControlPDU::parse(&data[1..])?,
                ))),
                (Ctl::Control, true) => Ok(LowerPDU::Segmented(SegmentedLowerPDU::Control(
                    SegmentedLowerControlPDU::parse(&data[1..])?,
                ))),
            }
        } else {
            Err(ParseError::InvalidLength)
        }
    }
}

#[derive(Copy, Clone)]
pub struct BlockAck(u32);

impl Default for BlockAck {
    fn default() -> Self {
        Self(0)
    }
}

impl BlockAck {
    pub fn is_acked(&self, seg_o: u8) -> bool {
        (self.0 & (1 << seg_o)) != 0
    }

    pub fn ack(&mut self, seg_o: u8) {
        self.0 = self.0 | (1 << seg_o)
    }
}
