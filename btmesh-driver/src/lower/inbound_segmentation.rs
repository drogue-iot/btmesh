use heapless::FnvIndexMap;
use heapless::IndexMap;
use heapless::Vec;

use crate::{Driver, DriverError};
use btmesh_common::address::UnicastAddress;
use btmesh_common::mic::SzMic;
use btmesh_common::{Seq, SeqZero};
use btmesh_pdu::lower::access::SegmentedLowerAccessPDU;
use btmesh_pdu::lower::control::SegmentedLowerControlPDU;
use btmesh_pdu::lower::SegmentedLowerPDU;
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
    ) -> Result<Option<UpperPDU<Driver>>, DriverError> {
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

            if !in_flight.already_seen(pdu) {
                in_flight.ingest(pdu)?;

                if in_flight.is_complete() {
                    // remove, make room for the next
                    let reassembled = Ok(Some(in_flight.reassemble()?));
                    self.current.remove(src);
                    return reassembled;
                }
            }
        }

        Ok(None)
    }
}

struct InFlight {
    seq_zero: SeqZero,
    seg_n: u8,
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
            seg_n,
            reassembly: Reassembly::new_access(seg_n, szmic),
        }
    }

    fn new_control(seq_zero: SeqZero, seg_n: u8, opcode: UpperControlOpcode) -> Self {
        Self {
            seq_zero,
            seg_n,
            reassembly: Reassembly::new_control(seg_n, opcode),
        }
    }

    fn is_valid(&self, pdu: &SegmentedLowerPDU<Driver>) -> bool {
        self.seq_zero == pdu.seq_zero()
    }

    fn already_seen(&self, pdu: &SegmentedLowerPDU<Driver>) -> bool {
        self.reassembly.already_seen(pdu.seg_n())
    }

    fn ingest(&mut self, pdu: &SegmentedLowerPDU<Driver>) -> Result<(), DriverError> {
        self.reassembly.ingest(pdu)
    }

    fn is_complete(&self) -> bool {
        for seg_o in 0..self.seg_n {
            if !self.reassembly.already_seen(seg_o) {
                return false;
            }
        }
        true
    }

    fn reassemble(&self) -> Result<UpperPDU<Driver>, DriverError> {
        self.reassembly.reassemble()
    }
}

enum Reassembly {
    Access {
        szmic: SzMic,
        mask: u64,
        data: [u8; 380],
        len: usize,
    },
    Control {
        opcode: UpperControlOpcode,
        mask: u32,
        data: [u8; 256],
        len: usize,
    },
}

impl Reassembly {
    fn new_access(seg_n: u8, szmic: SzMic) -> Self {
        Self::Access {
            szmic,
            mask: 0,
            data: [0; 380],
            len: 0,
        }
    }

    fn new_control(seg_n: u8, opcode: UpperControlOpcode) -> Self {
        Self::Control {
            opcode,
            mask: 0,
            data: [0; 256],
            len: 0,
        }
    }

    fn already_seen(&self, seg_n: u8) -> bool {
        match self {
            Reassembly::Access { mask, .. } => (*mask & (1 << seg_n)) != 0,
            Reassembly::Control { mask, .. } => (*mask & (1 << seg_n)) != 0,
        }
    }

    fn ingest(&mut self, pdu: &SegmentedLowerPDU<Driver>) -> Result<(), DriverError> {
        match (self, pdu) {
            (
                Reassembly::Access {
                    mask, data, len, ..
                },
                SegmentedLowerPDU::Access(pdu),
            ) => {
                const SEGMENT_SIZE: usize = SegmentedLowerAccessPDU::<Driver>::SEGMENT_SIZE;
                *mask = *mask | (1 << pdu.seg_o());
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
            (
                Reassembly::Control {
                    mask, data, len, ..
                },
                SegmentedLowerPDU::Control(pdu),
            ) => {
                const SEGMENT_SIZE: usize = SegmentedLowerControlPDU::<Driver>::SEGMENT_SIZE;
                *mask = *mask | (1 << pdu.seg_o());
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
