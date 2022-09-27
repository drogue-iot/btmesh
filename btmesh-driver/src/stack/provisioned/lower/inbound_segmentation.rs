use embassy_time::{Duration, Instant};
use heapless::FnvIndexMap;

use crate::stack::provisioned::system::UpperMetadata;
use crate::stack::provisioned::{DriverError, ProvisionedStack};
use crate::Watchdog;
use btmesh_common::address::UnicastAddress;
use btmesh_common::mic::SzMic;
use btmesh_common::SeqZero;
use btmesh_pdu::provisioned::lower::access::SegmentedLowerAccessPDU;
use btmesh_pdu::provisioned::lower::control::SegmentedLowerControlPDU;
use btmesh_pdu::provisioned::lower::{BlockAck, SegmentedLowerPDU};
use btmesh_pdu::provisioned::upper::access::UpperAccessPDU;
use btmesh_pdu::provisioned::upper::control::{ControlOpcode, UpperControlPDU};
use btmesh_pdu::provisioned::upper::UpperPDU;

pub struct InboundSegmentation<const N: usize = 4> {
    current: FnvIndexMap<UnicastAddress, InFlight, N>,
}

impl<const N: usize> Default for InboundSegmentation<N> {
    fn default() -> Self {
        Self {
            current: Default::default(),
        }
    }
}

pub struct SegmentationResult {
    pub block_ack: BlockAck,
    pub meta: UpperMetadata,
    pub upper_pdu: Option<UpperPDU<ProvisionedStack>>,
}

impl<const N: usize> InboundSegmentation<N> {
    pub fn expire_inbound(
        &mut self,
        seq_zero: &SeqZero,
        watchdog: &Watchdog,
    ) -> Option<(BlockAck, UpperMetadata)> {
        let result = self
            .current
            .values_mut()
            .find(|e| e.seq_zero == *seq_zero)
            .map(|in_flight| in_flight.expire());

        for e in self.current.values() {
            e.set_watchdog_expiration(watchdog);
        }

        if let Some(result) = result {
            match result {
                Err(src) => {
                    self.current.remove(&src);
                    None
                }
                Ok(inner @ Some(_)) => inner,
                _ => None,
            }
        } else {
            None
        }
    }

    /// Accept an inbound segmented `LowerPDU`, and attempt to reassemble
    /// into an `UpperPDU`. If processed without error, will return a tuple
    /// containing the current `BlockAck` set and optionally the completely
    /// reassembled `UpperPDU`, if all segments have been processed.
    pub fn process(
        &mut self,
        pdu: &SegmentedLowerPDU<ProvisionedStack>,
        watchdog: &Watchdog,
    ) -> Result<SegmentationResult, DriverError> {
        let src = pdu.meta().src();
        let in_flight = if let Some(current) = self.current.get_mut(&src) {
            current
        } else {
            let in_flight = InFlight::new(pdu);
            in_flight.set_watchdog_expiration(watchdog);
            self.current
                .insert(src, in_flight)
                .map_err(|_| DriverError::InsufficientSpace)?;
            self.current.get_mut(&src).unwrap()
        };

        if !in_flight.is_valid(pdu) {
            return Err(DriverError::InvalidPDU);
        }

        if in_flight.already_seen(pdu)? {
            Ok(SegmentationResult {
                block_ack: in_flight.block_ack(),
                meta: UpperMetadata::from_segmented_lower_pdu(pdu),
                upper_pdu: None,
            })
        } else {
            in_flight.ingest(pdu)?;
            Ok(SegmentationResult {
                block_ack: in_flight.block_ack(),
                meta: UpperMetadata::from_segmented_lower_pdu(pdu),
                upper_pdu: if in_flight.is_complete()? {
                    let reassembled =
                        Some(in_flight.reassemble(UpperMetadata::from_segmented_lower_pdu(pdu))?);
                    self.current.remove(&src);
                    reassembled
                } else {
                    None
                },
            })
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
    fn new(seq_zero: SeqZero, seg_n: u8) -> Self {
        Self {
            seg_n,
            block_ack: BlockAck::new(seq_zero),
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
    meta: UpperMetadata,
    last_ack: Instant,
    watchdogs: u8,
}

impl InFlight {
    /// Construct a new `InFlight` initialized with expected number of segments
    /// and other access- or control-specific details, such as `SzMic` or `UpperControlOpcode`.
    fn new(pdu: &SegmentedLowerPDU<ProvisionedStack>) -> Self {
        let meta = UpperMetadata::from_segmented_lower_pdu(pdu);
        match pdu {
            SegmentedLowerPDU::Access(inner) => {
                Self::new_access(inner.seq_zero(), inner.seg_n(), inner.szmic(), meta)
            }
            SegmentedLowerPDU::Control(inner) => {
                Self::new_control(inner.seq_zero(), inner.seg_n(), inner.opcode(), meta)
            }
        }
    }

    fn new_access(seq_zero: SeqZero, seg_n: u8, szmic: SzMic, meta: UpperMetadata) -> Self {
        Self {
            seq_zero,
            blocks: Blocks::new(seq_zero, seg_n),
            reassembly: Reassembly::new_access(szmic),
            meta,
            last_ack: Instant::now(),
            watchdogs: 0,
        }
    }

    fn new_control(
        seq_zero: SeqZero,
        seg_n: u8,
        opcode: ControlOpcode,
        meta: UpperMetadata,
    ) -> Self {
        Self {
            seq_zero,
            blocks: Blocks::new(seq_zero, seg_n),
            reassembly: Reassembly::new_control(opcode),
            meta,
            last_ack: Instant::now(),
            watchdogs: 0,
        }
    }

    fn expire(&mut self) -> Result<Option<(BlockAck, UpperMetadata)>, UnicastAddress> {
        self.watchdogs += 1;
        if self.watchdogs > 2 {
            Err(self.meta.src())
        } else {
            self.last_ack = Instant::now();
            Ok(Some((self.blocks.block_ack, self.meta.clone())))
        }
    }

    fn block_ack(&self) -> BlockAck {
        self.blocks.block_ack
    }

    /// Determine if the proposed segment is valid for the current in-flight reassembly.
    ///
    /// Returns `true` if it is valid, otherwise `false`.
    fn is_valid(&self, pdu: &SegmentedLowerPDU<ProvisionedStack>) -> bool {
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
    fn already_seen(&self, pdu: &SegmentedLowerPDU<ProvisionedStack>) -> Result<bool, DriverError> {
        self.blocks.already_seen(pdu.seg_o())
    }

    /// Ingest a segment.
    ///
    /// Returns a result of `()` or a `DriverError`.
    fn ingest(&mut self, pdu: &SegmentedLowerPDU<ProvisionedStack>) -> Result<(), DriverError> {
        if !self.is_valid(pdu) {
            return Err(DriverError::InvalidPDU);
        }
        self.last_ack = Instant::now();
        self.reassembly.ingest(pdu)?;
        self.blocks.ack(pdu.seg_o())?;
        Ok(())
    }

    fn set_watchdog_expiration(&self, watchdog: &Watchdog) {
        watchdog.inbound_expiration((
            self.last_ack + Duration::from_millis(150 + (50 * self.meta.ttl().value() as u64)),
            self.seq_zero,
        ))
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
    fn reassemble(&self, meta: UpperMetadata) -> Result<UpperPDU<ProvisionedStack>, DriverError> {
        if !self.is_complete()? {
            return Err(DriverError::InvalidState);
        }
        self.reassembly.reassemble(meta)
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
        opcode: ControlOpcode,
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

    fn new_control(opcode: ControlOpcode) -> Self {
        Self::Control {
            opcode,
            data: [0; 256],
            len: 0,
        }
    }

    fn ingest(&mut self, pdu: &SegmentedLowerPDU<ProvisionedStack>) -> Result<(), DriverError> {
        match (self, pdu) {
            (Reassembly::Access { data, len, .. }, SegmentedLowerPDU::Access(pdu)) => {
                const SEGMENT_SIZE: usize =
                    SegmentedLowerAccessPDU::<ProvisionedStack>::SEGMENT_SIZE;
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
                const SEGMENT_SIZE: usize =
                    SegmentedLowerControlPDU::<ProvisionedStack>::SEGMENT_SIZE;
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

    fn reassemble(&self, meta: UpperMetadata) -> Result<UpperPDU<ProvisionedStack>, DriverError> {
        match self {
            Reassembly::Control { data, opcode, len } => {
                Ok(UpperControlPDU::parse(*opcode, &data[0..*len], meta)?.into())
            }
            Reassembly::Access { data, szmic, len } => {
                Ok(UpperAccessPDU::parse(&data[0..*len], szmic, meta)?.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::stack::provisioned::lower::inbound_segmentation::{Blocks, InFlight, Reassembly};
    use crate::stack::provisioned::system::{LowerMetadata, UpperMetadata};
    use crate::stack::provisioned::{DriverError, ProvisionedStack};
    use btmesh_common::address::UnicastAddress;
    use btmesh_common::crypto::network::Nid;
    use btmesh_common::mic::SzMic;
    use btmesh_common::{IvIndex, Seq, SeqZero, Ttl};
    use btmesh_device::NetworkKeyHandle;
    use btmesh_models::foundation::configuration::NetKeyIndex;
    use btmesh_pdu::provisioned::lower::access::SegmentedLowerAccessPDU;
    use btmesh_pdu::provisioned::lower::control::SegmentedLowerControlPDU;
    use btmesh_pdu::provisioned::lower::SegmentedLowerPDU;
    use btmesh_pdu::provisioned::upper::control::ControlOpcode;
    use btmesh_pdu::provisioned::upper::UpperPDU;

    #[test]
    fn in_flight_is_valid_seq_zero() {
        let network_key_handle = NetworkKeyHandle::new(NetKeyIndex::new(0), Nid::new(42));
        let seq_zero = SeqZero::new(42);
        let seg_n = 4;
        let szmic = SzMic::Bit64;

        let pdu = SegmentedLowerAccessPDU::new(
            None,
            szmic,
            SeqZero::new(42),
            0,
            seg_n,
            &[],
            LowerMetadata::new(
                network_key_handle,
                IvIndex::parse(&[1, 2, 3, 4]).unwrap(),
                UnicastAddress::parse([0x00, 0x0A]).unwrap(),
                UnicastAddress::parse([0x00, 0x0B]).unwrap().into(),
                Seq::parse(1001).unwrap(),
                Ttl::new(128),
            ),
        )
        .unwrap();

        let pdu = SegmentedLowerPDU::Access(pdu);

        let meta = UpperMetadata::from_segmented_lower_pdu(&pdu);

        let in_flight = InFlight::new_access(seq_zero, seg_n, szmic, meta);

        assert!(in_flight.is_valid(&pdu));

        let pdu = SegmentedLowerAccessPDU::new(
            None,
            szmic,
            SeqZero::new(88),
            0,
            seg_n,
            &[],
            LowerMetadata::new(
                network_key_handle,
                IvIndex::parse(&[1, 2, 3, 4]).unwrap(),
                UnicastAddress::parse([0x00, 0x0A]).unwrap(),
                UnicastAddress::parse([0x00, 0x0B]).unwrap().into(),
                Seq::parse(1001).unwrap(),
                Ttl::new(128),
            ),
        )
        .unwrap();

        let pdu = SegmentedLowerPDU::Access(pdu);

        assert!(!in_flight.is_valid(&pdu));
    }

    #[test]
    fn in_flight_is_valid_pdu_type() {
        let network_key_handle = NetworkKeyHandle::new(NetKeyIndex::new(0), Nid::new(42));

        let seq_zero = SeqZero::new(42);
        let seg_n = 4;
        let szmic = SzMic::Bit64;

        let pdu = SegmentedLowerAccessPDU::new(
            None,
            szmic,
            SeqZero::new(42),
            0,
            seg_n,
            &[],
            LowerMetadata::new(
                network_key_handle,
                IvIndex::parse(&[1, 2, 3, 4]).unwrap(),
                UnicastAddress::parse([0x00, 0x0A]).unwrap(),
                UnicastAddress::parse([0x00, 0x0B]).unwrap().into(),
                Seq::parse(1001).unwrap(),
                Ttl::new(128),
            ),
        )
        .unwrap();

        let pdu = SegmentedLowerPDU::Access(pdu);

        let meta = UpperMetadata::from_segmented_lower_pdu(&pdu);

        let in_flight = InFlight::new_control(seq_zero, seg_n, ControlOpcode::FriendPoll, meta);

        assert!(!in_flight.is_valid(&pdu));

        let meta = UpperMetadata::from_segmented_lower_pdu(&pdu);

        let in_flight = InFlight::new_access(seq_zero, seg_n, SzMic::Bit32, meta);

        let pdu = SegmentedLowerControlPDU::new(
            ControlOpcode::FriendPoll,
            SeqZero::new(42),
            0,
            seg_n,
            &[],
            LowerMetadata::new(
                network_key_handle,
                IvIndex::parse(&[1, 2, 3, 4]).unwrap(),
                UnicastAddress::parse([0x00, 0x0A]).unwrap(),
                UnicastAddress::parse([0x00, 0x0B]).unwrap().into(),
                Seq::parse(1001).unwrap(),
                Ttl::new(128),
            ),
        )
        .unwrap();

        let pdu = SegmentedLowerPDU::Control(pdu);

        assert!(!in_flight.is_valid(&pdu));
    }

    #[test]
    fn blocks() {
        let mut blocks = Blocks::new(SeqZero::new(42), 3);

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
        let network_key_handle = NetworkKeyHandle::new(NetKeyIndex::new(0), Nid::new(42));
        let mut reassembly = Reassembly::new_access(SzMic::Bit32);

        let pdu = SegmentedLowerAccessPDU::<ProvisionedStack>::new(
            None,
            SzMic::Bit32,
            SeqZero::new(42),
            0,
            1,
            b"ABCDEFGHIJKL",
            LowerMetadata::new(
                network_key_handle,
                IvIndex::parse(&[1, 2, 3, 4]).unwrap(),
                UnicastAddress::parse([0x00, 0x0A]).unwrap(),
                UnicastAddress::parse([0x00, 0x0B]).unwrap().into(),
                Seq::parse(1001).unwrap(),
                Ttl::new(128),
            ),
        )
        .unwrap();

        let pdu = SegmentedLowerPDU::Access(pdu);

        reassembly.ingest(&pdu).unwrap();

        let pdu = SegmentedLowerAccessPDU::<ProvisionedStack>::new(
            None,
            SzMic::Bit32,
            SeqZero::new(42),
            1,
            1,
            b"ZYX",
            LowerMetadata::new(
                network_key_handle,
                IvIndex::parse(&[1, 2, 3, 4]).unwrap(),
                UnicastAddress::parse([0x00, 0x0A]).unwrap(),
                UnicastAddress::parse([0x00, 0x0B]).unwrap().into(),
                Seq::parse(1001).unwrap(),
                Ttl::new(128),
            ),
        )
        .unwrap();

        let pdu = SegmentedLowerPDU::Access(pdu);

        let upper_meta = UpperMetadata::from_segmented_lower_pdu(&pdu);

        reassembly.ingest(&pdu).unwrap();

        if let UpperPDU::Access(result) = reassembly.reassemble(upper_meta).unwrap() {
            let payload = result.payload();
            // last 4 go to the transmic
            assert_eq!(b"ABCDEFGHIJK", payload);

            let transmic = result.transmic();
            assert_eq!(b"LZYX", transmic.as_ref());
        } else {
            panic!("shouldn't happen")
        }
    }
}
