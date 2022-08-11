use crate::stack::provisioned::ProvisionedStack;
use crate::DriverError;
use btmesh_common::{InsufficientBuffer, Seq, SeqZero};
use btmesh_pdu::provisioned::lower::{BlockAck, InvalidBlock};
use btmesh_pdu::provisioned::upper::UpperPDU;
use heapless::Vec;

pub struct TransmitQueue<const N: usize = 5> {
    queue: Vec<Option<QueueEntry>, N>,
}

#[derive(Clone)]
struct QueueEntry {
    upper_pdu: UpperPDU<ProvisionedStack>,
    acked: Acked,
}

impl<const N: usize> Default for TransmitQueue<N> {
    fn default() -> Self {
        let mut queue = Vec::new();
        queue.resize(N, None);
        Self { queue }
    }
}

impl<const N: usize> TransmitQueue<N> {
    pub fn add(
        &mut self,
        upper_pdu: UpperPDU<ProvisionedStack>,
        num_segments: u8,
    ) -> Result<(), InsufficientBuffer> {
        info!("add to queue");
        let slot = self.queue.iter_mut().find(|e| e.is_none());

        let seq_zero = upper_pdu.meta().seq().into();

        if let Some(slot) = slot {
            info!("added to queue {}", seq_zero);
            slot.replace(QueueEntry {
                upper_pdu,
                acked: Acked::new(seq_zero, num_segments),
            });
        } else {
            info!("no space in queue");
        }

        info!("queue size {}", self.queue.len());

        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = UpperPDU<ProvisionedStack>> + '_ {
        QueueIter {
            inner: self.queue.iter(),
        }
    }

    pub fn receive_ack(&mut self, block_ack: BlockAck) -> Result<(), DriverError> {
        if let Some(slot) = self.queue.iter_mut().find(|e| {
            if let Some(entry) = e {
                let seq_zero: SeqZero = entry.upper_pdu.meta().seq().into();
                seq_zero == block_ack.seq_zero()
            } else {
                false
            }
        }) {
            if let Some(entry) = slot {
                let fully_acked = entry.acked.ack(block_ack)?;
                if fully_acked {
                    info!("fully acked, removing from retransmit queue");
                    slot.take();
                }
            }
        }
        Ok(())
    }
}

struct QueueIter<'i, I: Iterator<Item = &'i Option<QueueEntry>>> {
    inner: I,
}

impl<'i, I: Iterator<Item = &'i Option<QueueEntry>>> Iterator for QueueIter<'i, I> {
    type Item = UpperPDU<ProvisionedStack>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.inner.next() {
            if let Some(next) = next {
                return Some(next.upper_pdu.clone());
            }
        }

        None
    }
}

#[derive(Clone)]
struct Acked {
    num_segments: u8,
    block_ack: BlockAck,
}

impl Acked {
    fn new(seq_zero: SeqZero, num_segments: u8) -> Self {
        Self {
            num_segments,
            block_ack: BlockAck::new(seq_zero),
        }
    }

    fn ack(&mut self, block_ack: BlockAck) -> Result<bool, InvalidBlock> {
        for ack in block_ack.acked_iter() {
            self.block_ack.ack(ack)?;
        }

        Ok(self.block_ack.is_fully_acked(self.num_segments))
    }
}
