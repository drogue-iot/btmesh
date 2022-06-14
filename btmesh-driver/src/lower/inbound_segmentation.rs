use heapless::FnvIndexMap;

use crate::{Driver, DriverError};
use btmesh_common::address::UnicastAddress;
use btmesh_common::mic::SzMic;
use btmesh_common::SeqZero;
use btmesh_pdu::lower::access::SegmentedLowerAccessPDU;
use btmesh_pdu::lower::control::SegmentedLowerControlPDU;
use btmesh_pdu::lower::{BlockAck, SegmentedLowerPDU};
use btmesh_pdu::upper::access::UpperAccessPDU;
use btmesh_pdu::upper::control::{UpperControlOpcode, UpperControlPDU};
use btmesh_pdu::upper::UpperPDU;

pub struct InboundSegmentation<const N: usize = 5> {
    current: FnvIndexMap<UnicastAddress, InFlight, N>,
}

impl<const N: usize> Default for InboundSegmentation<N> {
    fn default() -> Self {
        Self {
            current: Default::default(),
        }
    }
}

impl<const N: usize> InboundSegmentation<N> {
    /// Accept an inbound segmented `LowerPDU`, and attempt to reassemble
    /// into an `UpperPDU`. If processed without error, will return a tuple
    /// containing the current `BlockAck` set and optionally the completely
    /// reassembled `UpperPDU`, if all segments have been processed.
    pub fn process(
        &mut self,
        pdu: &SegmentedLowerPDU<Driver>,
    ) -> Result<(BlockAck, Option<UpperPDU<Driver>>), DriverError> {
        if let Some(src) = &pdu.meta().src {
            let in_flight = if let Some(current) = self.current.get_mut(src) {
                current
            } else {
                let in_flight = InFlight::new(pdu);
                self.current
                    .insert(*src, in_flight)
                    .map_err(|_| DriverError::InsufficientSpace)?;
                self.current.get_mut(src).unwrap()
            };

            if !in_flight.is_valid(pdu) {
                return Err(DriverError::InvalidPDU);
            }

            if in_flight.already_seen(pdu)? {
                Ok((in_flight.block_ack(), None))
            } else {
                in_flight.ingest(pdu)?;

                Ok((
                    in_flight.block_ack(),
                    if in_flight.is_complete()? {
                        let reassembled = Some(in_flight.reassemble()?);
                        self.current.remove(src);
                        reassembled
                    } else {
                        None
                    },
                ))
            }
        } else {
            Err(DriverError::InvalidPDU)
        }
    }
}

/// Tracking of processed blocks and `BlockAck`.
struct Blocks {
    seg_n: u8,
    block_ack: BlockAck,
}

impl Blocks {
    /// Construct a new block-tracker.
    ///
    /// * `seg_n` The number of segments to expect.
    fn new(seg_n: u8) -> Self {
        Self {
            seg_n,
            block_ack: Default::default(),
        }
    }

    /// Record that a given segment has been processed.
    ///
    /// * `seg_o` The `seg_o` (offset) of the block to denote as processed.
    fn ack(&mut self, seg_o: u8) -> Result<(), DriverError> {
        if seg_o > self.seg_n {
            return Err(DriverError::InvalidState);
        }
        Ok(self.block_ack.ack(seg_o)?)
    }

    /// Determine if a given segment has been seen.
    ///
    /// * `seg_o`: The `seg_o` (offset) of the block to check.
    ///
    /// Returns `true` if it has been processed, otherwise `false`,
    /// or a `DriverError` if attempting to check a block outside of
    /// the range of 0-31.
    fn already_seen(&self, seg_o: u8) -> Result<bool, DriverError> {
        Ok(self.block_ack.is_acked(seg_o)?)
    }

    /// Determine if all expected blocks have been processed.
    ///
    /// Returns `true` if all blocks have been processed, otherwise `false`.
    fn is_complete(&self) -> Result<bool, DriverError> {
        for seg_o in 0..=self.seg_n {
            if !self.block_ack.is_acked(seg_o)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// Track the in-flight reassembly of segmented `LowerPDUs`.
struct InFlight {
    seq_zero: SeqZero,
    blocks: Blocks,
    reassembly: Reassembly,
}

impl InFlight {
    /// Construct a new `InFlight` initialized with expected number of segments
    /// and other access- or control-specific details, such as `SzMic` or `UpperControlOpcode`.
    fn new(pdu: &SegmentedLowerPDU<Driver>) -> Self {
        match pdu {
            SegmentedLowerPDU::Access(pdu) => {
                Self::new_access(pdu.seq_zero(), pdu.seg_n(), pdu.szmic())
            }
            SegmentedLowerPDU::Control(pdu) => {
                Self::new_control(pdu.seq_zero(), pdu.seg_n(), pdu.opcode())
            }
        }
    }

    fn new_access(seq_zero: SeqZero, seg_n: u8, szmic: SzMic) -> Self {
        Self {
            seq_zero,
            blocks: Blocks::new(seg_n),
            reassembly: Reassembly::new_access(szmic),
        }
    }

    fn new_control(seq_zero: SeqZero, seg_n: u8, opcode: UpperControlOpcode) -> Self {
        Self {
            seq_zero,
            blocks: Blocks::new(seg_n),
            reassembly: Reassembly::new_control(opcode),
        }
    }

    fn block_ack(&self) -> BlockAck {
        self.blocks.block_ack
    }

    /// Determine if the proposed segment is valid for the current in-flight reassembly.
    ///
    /// Returns `true` if it is valid, otherwise `false`.
    fn is_valid(&self, pdu: &SegmentedLowerPDU<Driver>) -> bool {
        // TODO: check pdu-specific details such as SzMic or UpperControlOpcode.
        match (&self.reassembly, pdu) {
            (Reassembly::Access { szmic, .. }, SegmentedLowerPDU::Access(pdu)) => {
                if pdu.szmic() != *szmic {
                    return false;
                }
                // ignore, okay
            }
            (Reassembly::Control { opcode, .. }, SegmentedLowerPDU::Control(pdu)) => {
                if pdu.opcode() != *opcode {
                    return false;
                }
                // ignore, okay
            }
            _ => return false,
        }
        self.seq_zero == pdu.seq_zero()
    }

    /// Determine if the proposed segment has already been seen for the current in-flight reassembly.
    ///
    /// Returns `true` if it has been seen, otherwise `false`.
    fn already_seen(&self, pdu: &SegmentedLowerPDU<Driver>) -> Result<bool, DriverError> {
        self.blocks.already_seen(pdu.seg_n())
    }

    /// Ingest a segment.
    ///
    /// Returns a result of `()` or a `DriverError`.
    fn ingest(&mut self, pdu: &SegmentedLowerPDU<Driver>) -> Result<(), DriverError> {
        if !self.is_valid(pdu) {
            return Err(DriverError::InvalidPDU);
        }
        self.reassembly.ingest(pdu)?;
        self.blocks.ack(pdu.seg_o())?;
        Ok(())
    }

    /// Determine if all expected blocks have been processed.
    ///
    /// Returns `true` if all blocks have been processed, otherwise `false`.
    fn is_complete(&self) -> Result<bool, DriverError> {
        self.blocks.is_complete()
    }

    /// Reassemble a complete set of segments into a single `UpperPDU`.
    ///
    /// Returns a result of the reassembled `UpperPDU` or a `DriverError`, most likely `DriverError::InvalidState`.
    fn reassemble(&self) -> Result<UpperPDU<Driver>, DriverError> {
        if !self.is_complete()? {
            return Err(DriverError::InvalidState);
        }
        self.reassembly.reassemble()
    }
}

/// Structure allowing random-access assembly of access or control PDU segments.
enum Reassembly {
    Access {
        szmic: SzMic,
        data: [u8; 380],
        len: usize,
    },
    Control {
        opcode: UpperControlOpcode,
        data: [u8; 256],
        len: usize,
    },
}

impl Reassembly {
    fn new_access(szmic: SzMic) -> Self {
        Self::Access {
            szmic,
            data: [0; 380],
            len: 0,
        }
    }

    fn new_control(opcode: UpperControlOpcode) -> Self {
        Self::Control {
            opcode,
            data: [0; 256],
            len: 0,
        }
    }

    fn ingest(&mut self, pdu: &SegmentedLowerPDU<Driver>) -> Result<(), DriverError> {
        match (self, pdu) {
            (Reassembly::Access { data, len, .. }, SegmentedLowerPDU::Access(pdu)) => {
                const SEGMENT_SIZE: usize = SegmentedLowerAccessPDU::<Driver>::SEGMENT_SIZE;
                if pdu.seg_o() == pdu.seg_n() {
                    // the last segment, we now know the length.
                    *len = SEGMENT_SIZE * (pdu.seg_n() as usize) + pdu.segment_m().len();
                    data[SEGMENT_SIZE * pdu.seg_o() as usize
                        ..SEGMENT_SIZE * pdu.seg_o() as usize + pdu.segment_m().len()]
                        .clone_from_slice(pdu.segment_m());
                } else {
                    data[SEGMENT_SIZE * pdu.seg_o() as usize
                        ..=SEGMENT_SIZE * pdu.seg_o() as usize + (SEGMENT_SIZE - 1)]
                        .clone_from_slice(pdu.segment_m());
                }
            }
            (Reassembly::Control { data, len, .. }, SegmentedLowerPDU::Control(pdu)) => {
                const SEGMENT_SIZE: usize = SegmentedLowerControlPDU::<Driver>::SEGMENT_SIZE;
                if pdu.seg_o() == pdu.seg_n() {
                    // the last segment
                    *len = SEGMENT_SIZE * (pdu.seg_n() as usize) + pdu.segment_m().len();
                    data[SEGMENT_SIZE * pdu.seg_o() as usize
                        ..SEGMENT_SIZE * pdu.seg_o() as usize + pdu.segment_m().len()]
                        .clone_from_slice(pdu.segment_m());
                } else {
                    data[SEGMENT_SIZE * pdu.seg_o() as usize
                        ..=SEGMENT_SIZE * pdu.seg_o() as usize + (SEGMENT_SIZE - 1)]
                        .clone_from_slice(pdu.segment_m());
                }
            }
            _ => return Err(DriverError::InvalidPDU),
        }
        Ok(())
    }

    fn reassemble(&self) -> Result<UpperPDU<Driver>, DriverError> {
        match self {
            Reassembly::Control { data, opcode, len } => {
                Ok(UpperControlPDU::parse(*opcode, &data[0..*len])?.into())
            }
            Reassembly::Access { data, szmic, len } => {
                Ok(UpperAccessPDU::parse(&data[0..*len], *szmic)?.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lower::inbound_segmentation::{Blocks, InFlight, Reassembly};
    use crate::{Driver, DriverError};
    use btmesh_common::mic::{Bit32TransMic, SzMic, TransMic};
    use btmesh_common::SeqZero;
    use btmesh_pdu::lower::access::SegmentedLowerAccessPDU;
    use btmesh_pdu::lower::control::SegmentedLowerControlPDU;
    use btmesh_pdu::lower::SegmentedLowerPDU;
    use btmesh_pdu::upper::control::UpperControlOpcode;
    use btmesh_pdu::upper::UpperPDU;

    #[test]
    fn in_flight_is_valid_seq_zero() {
        let seq_zero = SeqZero::new(42);
        let seg_n = 4;
        let szmic = SzMic::Bit64;
        let in_flight = InFlight::new_access(seq_zero, seg_n, szmic);

        let pdu =
            SegmentedLowerAccessPDU::new(None, szmic, SeqZero::new(42), 0, seg_n, &[]).unwrap();

        let pdu = SegmentedLowerPDU::Access(pdu);

        assert_eq!(true, in_flight.is_valid(&pdu));

        let pdu =
            SegmentedLowerAccessPDU::new(None, szmic, SeqZero::new(88), 0, seg_n, &[]).unwrap();

        let pdu = SegmentedLowerPDU::Access(pdu);

        assert_eq!(false, in_flight.is_valid(&pdu));
    }

    #[test]
    fn in_flight_is_valid_pdu_type() {
        let seq_zero = SeqZero::new(42);
        let seg_n = 4;
        let szmic = SzMic::Bit64;
        let in_flight = InFlight::new_control(seq_zero, seg_n, UpperControlOpcode::FriendPoll);

        let pdu =
            SegmentedLowerAccessPDU::new(None, szmic, SeqZero::new(42), 0, seg_n, &[]).unwrap();

        let pdu = SegmentedLowerPDU::Access(pdu);

        assert_eq!(false, in_flight.is_valid(&pdu));

        let in_flight = InFlight::new_access(seq_zero, seg_n, SzMic::Bit32);

        let pdu = SegmentedLowerControlPDU::new(
            UpperControlOpcode::FriendPoll,
            SeqZero::new(42),
            0,
            seg_n,
            &[],
        )
        .unwrap();

        let pdu = SegmentedLowerPDU::Control(pdu);

        assert_eq!(false, in_flight.is_valid(&pdu));
    }

    #[test]
    fn blocks() {
        let mut blocks = Blocks::new(3);

        assert_eq!(Ok(false), blocks.is_complete());
        assert_eq!(Ok(()), blocks.ack(0));
        assert_eq!(Ok(false), blocks.is_complete());
        assert_eq!(Ok(()), blocks.ack(1));
        assert_eq!(Ok(false), blocks.is_complete());
        assert_eq!(Ok(()), blocks.ack(2));
        assert_eq!(Ok(()), blocks.ack(2));
        assert_eq!(Ok(()), blocks.ack(2));
        assert_eq!(Ok(()), blocks.ack(2));
        assert_eq!(Ok(false), blocks.is_complete());
        assert_eq!(Ok(()), blocks.ack(3));
        assert_eq!(Ok(true), blocks.is_complete());

        assert_eq!(Err(DriverError::InvalidState), blocks.ack(4));
    }

    #[test]
    fn reassembly() {
        let mut reassembly = Reassembly::new_access(SzMic::Bit32);

        let pdu = SegmentedLowerAccessPDU::<Driver>::new(
            None,
            SzMic::Bit32,
            SeqZero::new(42),
            0,
            1,
            b"ABCDEFGHIJKL",
        )
        .unwrap();

        let pdu = SegmentedLowerPDU::Access(pdu);

        reassembly.ingest(&pdu);

        let pdu = SegmentedLowerAccessPDU::<Driver>::new(
            None,
            SzMic::Bit32,
            SeqZero::new(42),
            1,
            1,
            b"ZYX",
        )
        .unwrap();

        let pdu = SegmentedLowerPDU::Access(pdu);

        reassembly.ingest(&pdu);

        if let UpperPDU::Access(result) = reassembly.reassemble().unwrap() {
            let payload = result.payload();
            // last 4 go to the transmic
            assert_eq!(b"ABCDEFGHIJK", payload);

            let transmic = result.transmic();
            assert_eq!(b"LZYX", transmic.as_slice());
        } else {
            assert!(false)
        }
    }
}
