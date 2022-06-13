use heapless::FnvIndexMap;
use heapless::IndexMap;
use heapless::Vec;

use crate::{Driver, DriverError};
use btmesh_common::address::UnicastAddress;
use btmesh_common::mic::SzMic;
use btmesh_common::{Seq, SeqZero};
use btmesh_pdu::lower::access::SegmentedLowerAccessPDU;
use btmesh_pdu::lower::control::SegmentedLowerControlPDU;
use btmesh_pdu::lower::{BlockAck, SegmentedLowerPDU};
use btmesh_pdu::upper::access::UpperAccessPDU;
use btmesh_pdu::upper::control::{UpperControlOpcode, UpperControlPDU};
use btmesh_pdu::upper::UpperPDU;

pub struct InboundSegmentation<const N: usize> {
    current: FnvIndexMap<UnicastAddress, InFlight, N>,
}

impl<const N: usize> InboundSegmentation<N> {
    fn process(
        &mut self,
        pdu: &SegmentedLowerPDU<Driver>,
    ) -> Result<(BlockAck, Option<UpperPDU<Driver>>), DriverError> {
        if let Some(src) = &pdu.meta().src {
            let in_flight = if let Some(current) = self.current.get_mut(src) {
                current
            } else {
                let in_flight = InFlight::new(pdu);
                self.current.insert(*src, in_flight);
                self.current.get_mut(src).unwrap()
            };

            if !in_flight.is_valid(pdu) {
                Err(DriverError::InvalidPDU)?;
            }

            if in_flight.already_seen(pdu) {
                Ok((
                    in_flight.block_ack(),
                    None,
                ))

            } else {
                in_flight.ingest(pdu)?;

                Ok((
                    in_flight.block_ack(),
                    if in_flight.is_complete() {
                        let reassembled = Some(in_flight.reassemble()?);
                        self.current.remove(src);
                        reassembled
                    } else {
                        None
                    }
                ))
            }
        } else {
            Err(DriverError::InvalidPDU)?
        }
    }
}

struct Blocks {
    seg_n: u8,
    block_ack: BlockAck,
}

impl Blocks {
    fn new(seg_n: u8) -> Self {
        Self {
            seg_n,
            block_ack: Default::default(),
        }
    }

    fn ack(&mut self, seg_o: u8) {
        self.block_ack.ack(seg_o)
    }

    fn already_seen(&self, seg_o: u8) -> bool {
        self.block_ack.is_acked(seg_o)
    }

    fn is_complete(&self) -> bool {
        for seg_o in 0..self.seg_n {
            if !self.block_ack.is_acked(seg_o) {
                return false;
            }
        }
        true
    }
}

struct InFlight {
    seq_zero: SeqZero,
    blocks: Blocks,
    reassembly: Reassembly,
}

impl InFlight {
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

    fn is_valid(&self, pdu: &SegmentedLowerPDU<Driver>) -> bool {
        self.seq_zero == pdu.seq_zero()
    }

    fn already_seen(&self, pdu: &SegmentedLowerPDU<Driver>) -> bool {
        self.blocks.already_seen(pdu.seg_n())
    }

    fn ingest(&mut self, pdu: &SegmentedLowerPDU<Driver>) -> Result<(), DriverError> {
        self.reassembly.ingest(pdu)?;
        self.blocks.ack(pdu.seg_o());
        Ok(())
    }

    fn is_complete(&self) -> bool {
        self.blocks.is_complete()
    }

    fn reassemble(&self) -> Result<UpperPDU<Driver>, DriverError> {
        self.reassembly.reassemble()
    }
}

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
            (
                Reassembly::Access {
                    data, len, ..
                },
                SegmentedLowerPDU::Access(pdu),
            ) => {
                const SEGMENT_SIZE: usize = SegmentedLowerAccessPDU::<Driver>::SEGMENT_SIZE;
                if pdu.seg_o() == pdu.seg_n() {
                    // the last segment, we now know the length.
                    *len = SEGMENT_SIZE * (pdu.seg_n() as usize - 1) + pdu.segment_m().len();
                    data[SEGMENT_SIZE * pdu.seg_o() as usize
                        ..SEGMENT_SIZE * pdu.seg_o() as usize + pdu.segment_m().len()]
                        .clone_from_slice(pdu.segment_m());
                } else {
                    data[SEGMENT_SIZE * pdu.seg_o() as usize
                        ..SEGMENT_SIZE * pdu.seg_o() as usize + (SEGMENT_SIZE - 1)]
                        .clone_from_slice(pdu.segment_m());
                }
            }
            (
                Reassembly::Control {
                    data, len, ..
                },
                SegmentedLowerPDU::Control(pdu),
            ) => {
                const SEGMENT_SIZE: usize = SegmentedLowerControlPDU::<Driver>::SEGMENT_SIZE;
                if pdu.seg_o() == pdu.seg_n() {
                    // the last segment
                    *len = SEGMENT_SIZE * (pdu.seg_n() as usize - 1) + pdu.segment_m().len();
                    data[SEGMENT_SIZE * pdu.seg_o() as usize
                        ..SEGMENT_SIZE * pdu.seg_o() as usize + pdu.segment_m().len()]
                        .clone_from_slice(pdu.segment_m());
                } else {
                    data[SEGMENT_SIZE * pdu.seg_o() as usize
                        ..SEGMENT_SIZE * pdu.seg_o() as usize + (SEGMENT_SIZE - 1)]
                        .clone_from_slice(pdu.segment_m());
                }
            }
            _ => Err(DriverError::InvalidPDU)?,
        }
        Ok(())
    }

    fn reassemble(&self) -> Result<UpperPDU<Driver>, DriverError> {
        match self {
            Reassembly::Control { data, opcode, .. } => {
                Ok(UpperControlPDU::parse(*opcode, data)?.into())
            }
            Reassembly::Access { data, szmic, .. } => {
                Ok(UpperAccessPDU::parse(data, *szmic)?.into())
            }
        }
    }
}
