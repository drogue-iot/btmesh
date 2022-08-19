pub mod access;
pub mod control;

use crate::provisioned::lower::access::{SegmentedLowerAccessPDU, UnsegmentedLowerAccessPDU};
use crate::provisioned::lower::control::{SegmentedLowerControlPDU, UnsegmentedLowerControlPDU};
use crate::provisioned::network::CleartextNetworkPDU;
use crate::provisioned::System;
use btmesh_common::{Ctl, ParseError, SeqZero};

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

impl<S: System> From<SegmentedLowerPDU<S>> for LowerPDU<S> {
    fn from(inner: SegmentedLowerPDU<S>) -> Self {
        Self::Segmented(inner)
    }
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
    pub fn parse(
        network_pdu: &CleartextNetworkPDU<S>,
        meta: S::LowerMetadata,
    ) -> Result<Self, ParseError> {
        let data = network_pdu.transport_pdu();

        if data.len() >= 2 {
            let seg = data[0] & 0b10000000 != 0;

            match (network_pdu.ctl(), seg) {
                (Ctl::Access, false) => Ok(LowerPDU::Unsegmented(UnsegmentedLowerPDU::Access(
                    UnsegmentedLowerAccessPDU::parse(data, meta)?,
                ))),
                (Ctl::Access, true) => Ok(LowerPDU::Segmented(SegmentedLowerPDU::Access(
                    SegmentedLowerAccessPDU::parse(data, meta)?,
                ))),
                (Ctl::Control, false) => Ok(LowerPDU::Unsegmented(UnsegmentedLowerPDU::Control(
                    UnsegmentedLowerControlPDU::parse(data, meta)?,
                ))),
                (Ctl::Control, true) => Ok(LowerPDU::Segmented(SegmentedLowerPDU::Control(
                    SegmentedLowerControlPDU::parse(data, meta)?,
                ))),
            }
        } else {
            Err(ParseError::InvalidLength)
        }
    }
}

/// Error indicating an attempt to ack or check a block outside the range of 0-31.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct InvalidBlock;

/// Structure for tracking and communicating "block acks",
/// indicating which segment(s) have been received and should
/// be ACK'd for a given segmented lower PDU.
#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct BlockAck(u32, SeqZero);

impl BlockAck {
    pub fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() != 6 {
            Err(ParseError::InvalidLength)
        } else {
            let seq_zero = SeqZero::parse(
                ((parameters[0] as u16 & 0b01111111) << 6)
                    | ((parameters[1] as u16 & 0b11111100) >> 2),
            )?;

            let block_ack =
                u32::from_be_bytes([parameters[2], parameters[3], parameters[4], parameters[5]]);

            Ok(Self(block_ack, seq_zero))
        }
    }

    pub fn new(seq_zero: SeqZero) -> Self {
        Self(0, seq_zero)
    }

    pub fn is_fully_acked(&self, seg_n: u8) -> bool {
        if seg_n >= 32 {
            return false;
        }

        for i in 0..seg_n {
            if (1 << i) & self.0 == 0 {
                return false;
            }
        }
        true
    }

    pub fn is_acked(&self, seg_o: u8) -> Result<bool, InvalidBlock> {
        if seg_o >= 32 {
            return Err(InvalidBlock);
        }
        Ok((self.0 & (1 << seg_o)) != 0)
    }

    pub fn acked_iter(&self) -> impl Iterator<Item = u8> {
        AckIter::new(self.0)
    }

    pub fn ack(&mut self, seg_o: u8) -> Result<(), InvalidBlock> {
        if seg_o >= 32 {
            return Err(InvalidBlock);
        }
        self.0 |= 1 << seg_o;
        Ok(())
    }

    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn seq_zero(&self) -> SeqZero {
        self.1
    }
}

pub struct AckIter {
    block_ack: u32,
    cur: u8,
}

impl AckIter {
    pub fn new(block_ack: u32) -> Self {
        Self { block_ack, cur: 0 }
    }
}

impl Iterator for AckIter {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.cur >= 32 {
                return None;
            } else {
                let next = ((1 << self.cur) & self.block_ack) != 0;
                self.cur += 1;
                if next {
                    return Some(self.cur - 1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::provisioned::lower::{BlockAck, InvalidBlock};
    use btmesh_common::SeqZero;

    #[test]
    pub fn block_ack_valid_blocks() {
        let mut block_ack = BlockAck::new(SeqZero::new(42));

        assert_eq!(0, block_ack.value());

        assert_eq!(Ok(false), block_ack.is_acked(1));

        block_ack.ack(1).unwrap();

        assert_eq!(Ok(true), block_ack.is_acked(1));

        block_ack.ack(1).unwrap();

        assert_eq!(Ok(true), block_ack.is_acked(1));

        assert_eq!(Ok(false), block_ack.is_acked(4));

        block_ack.ack(4).unwrap();

        assert_eq!(Ok(true), block_ack.is_acked(4));

        assert_eq!(18, block_ack.value());

        assert_eq!(Ok(false), block_ack.is_acked(0));
        assert_eq!(Ok(()), block_ack.ack(0));
        assert_eq!(Ok(true), block_ack.is_acked(0));

        assert_eq!(Ok(false), block_ack.is_acked(31));
        assert_eq!(Ok(()), block_ack.ack(31));
        assert_eq!(Ok(true), block_ack.is_acked(31));
    }

    #[test]
    pub fn block_ack_invalid_blocks() {
        let mut block_ack = BlockAck::new(SeqZero::new(42));

        assert_eq!(Err(InvalidBlock), block_ack.ack(32));
        assert_eq!(Err(InvalidBlock), block_ack.is_acked(32));

        assert_eq!(Err(InvalidBlock), block_ack.ack(99));
        assert_eq!(Err(InvalidBlock), block_ack.is_acked(99));
    }
}
