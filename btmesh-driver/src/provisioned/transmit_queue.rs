use core::ops::Add;
use heapless::Vec;
use btmesh_common::address::Address;
use btmesh_common::{InsufficientBuffer, Seq};
use btmesh_pdu::lower::BlockAck;
use btmesh_pdu::network::NetworkPDU;
use btmesh_pdu::upper::UpperPDU;
use crate::provisioned::ProvisionedDriver;
use crate::provisioned::sequence::Sequence;


#[derive(Default)]
pub struct TransmitQueue<const N: usize=5> {
    queue: Vec<Option<QueueEntry>, N>,
}

struct QueueEntry {
    upper_pdu: UpperPDU<ProvisionedDriver>,
    acked: Acked,
}

impl<const N: usize> TransmitQueue<N> {

    pub fn add(&mut self, upper_pdu: UpperPDU<ProvisionedDriver>, num_segments: u8) -> Result<(), InsufficientBuffer> {
        let slot = self.queue.iter_mut().find(|e| matches!(e, None) );

        if let Some(slot) = slot {
            slot.replace(
                QueueEntry {
                    upper_pdu,
                    acked: Acked::new(num_segments ),
                }
            );
        }

        Ok(())
    }
}

struct Acked {
    num_segments: u8,
    block_ack: BlockAck,
}

impl Acked {
    fn new(num_segments: u8) -> Self {
        Self {
            num_segments,
            block_ack: Default::default()
        }

    }

}
