use crate::stack::provisioned::ProvisionedStack;
use crate::{DriverError, Watchdog};
use btmesh_common::{address::Address, InsufficientBuffer, SeqZero};
use btmesh_device::CompletionToken;
use btmesh_pdu::provisioned::lower::{BlockAck, InvalidBlock};
use btmesh_pdu::provisioned::upper::UpperPDU;
use embassy_time::{Duration, Instant};
use heapless::Vec;

pub struct TransmitQueue<const N: usize = 8> {
    queue: Vec<Option<QueueEntry>, N>,
}

enum QueueEntry {
    Nonsegmented(NonsegmentedQueueEntry),
    Segmented(SegmentedQueueEntry),
}

struct NonsegmentedQueueEntry {
    upper_pdu: UpperPDU<ProvisionedStack>,
    num_retransmit: u8,
    completion_token: Option<CompletionToken>,
}

struct SegmentedQueueEntry {
    upper_pdu: UpperPDU<ProvisionedStack>,
    acked: Acked,
    completion_token: Option<CompletionToken>,
}

impl<const N: usize> Default for TransmitQueue<N> {
    fn default() -> Self {
        let mut queue = Vec::new();
        for _ in 0..N {
            queue.push(None).ok();
        }
        Self { queue }
    }
}

impl<const N: usize> TransmitQueue<N> {
    pub fn has_ongoing_completion(&self) -> bool {
        self.queue.iter().any(|e| match e {
            Some(entry) => match entry {
                QueueEntry::Nonsegmented(entry) => entry.completion_token.is_some(),
                QueueEntry::Segmented(entry) => entry.completion_token.is_some(),
            },
            _ => false,
        })
    }

    pub fn add_segmented(
        &mut self,
        upper_pdu: UpperPDU<ProvisionedStack>,
        num_segments: u8,
        completion_token: Option<CompletionToken>,
        watchdog: &Watchdog,
        num_retransmits: u8,
    ) -> Result<(), InsufficientBuffer> {
        // Only add as segmented message in the queue if destination is an unicast address that can ack.
        // If not, add it as a non-segmented message.
        //
        // TODO The name 'segmented' really means 'ackable' in this context, so
        // consider doing a proper renaming in this code.
        if let Address::Unicast(_) = upper_pdu.meta().dst() {
            let slot = self.queue.iter_mut().find(|e| e.is_none());
            let seq_zero = upper_pdu.meta().seq().into();
            if let Some(slot) = slot {
                slot.replace(QueueEntry::Segmented(SegmentedQueueEntry {
                    upper_pdu,
                    acked: Acked::new(seq_zero, num_segments),
                    completion_token,
                }));
            } else {
                warn!("no space in retransmit queue");
            }

            for slot in self.queue.iter().flatten() {
                if let QueueEntry::Segmented(slot) = slot {
                    let now = Instant::now();
                    let timeout = Duration::from_millis(
                        200 + (50 * slot.upper_pdu.meta().ttl().value() as u64),
                    );
                    let sz: SeqZero = slot.upper_pdu.meta().seq().into();
                    watchdog.outbound_expiration((now + timeout, sz));
                }
            }
        } else {
            self.add_nonsegmented(upper_pdu, num_retransmits, completion_token)?;
        }

        Ok(())
    }

    pub fn add_nonsegmented(
        &mut self,
        upper_pdu: UpperPDU<ProvisionedStack>,
        num_retransmit: u8,
        completion_token: Option<CompletionToken>,
    ) -> Result<(), InsufficientBuffer> {
        let slot = self.queue.iter_mut().find(|e| e.is_none());

        if let Some(slot) = slot {
            slot.replace(QueueEntry::Nonsegmented(NonsegmentedQueueEntry {
                upper_pdu,
                num_retransmit,
                completion_token,
            }));
        } else {
            warn!("no space in retransmit queue");
        }

        Ok(())
    }

    pub fn iter(&mut self) -> impl Iterator<Item = UpperPDU<ProvisionedStack>> + '_ {
        QueueIter {
            inner: self.queue.iter_mut(),
        }
    }

    pub fn expire_outbound(&mut self, seq_zero: &SeqZero) {
        for slot in self.queue.iter_mut() {
            if let Some(QueueEntry::Segmented(entry)) = slot {
                let sz = SeqZero::from(entry.upper_pdu.meta().seq());
                if sz == *seq_zero {
                    slot.take();
                }
            }
        }
    }

    pub fn receive_ack(
        &mut self,
        block_ack: BlockAck,
        watchdog: &Watchdog,
    ) -> Result<(), DriverError> {
        if let Some(slot) = self.queue.iter_mut().find(|e| {
            if let Some(QueueEntry::Segmented(entry)) = e {
                let seq_zero: SeqZero = entry.upper_pdu.meta().seq().into();
                seq_zero == block_ack.seq_zero()
            } else {
                false
            }
        }) {
            if let Some(QueueEntry::Segmented(entry)) = slot {
                let fully_acked = entry.acked.ack(block_ack, watchdog)?;
                if fully_acked {
                    watchdog.clear_outbound_expiration(entry.upper_pdu.meta().seq().into());
                    if let Some(token) = entry.completion_token.as_ref() {
                        token.complete();
                    };
                    slot.take();
                }
            }
        }

        for slot in self.queue.iter().flatten() {
            if let QueueEntry::Segmented(slot) = slot {
                let now = Instant::now();
                let timeout =
                    Duration::from_millis(200 + (50 * slot.upper_pdu.meta().ttl().value() as u64));
                watchdog.outbound_expiration((now + timeout, slot.upper_pdu.meta().seq().into()));
            }
        }
        Ok(())
    }
}

struct QueueIter<'i, I: Iterator<Item = &'i mut Option<QueueEntry>>> {
    inner: I,
}

impl<'i, I: Iterator<Item = &'i mut Option<QueueEntry>>> Iterator for QueueIter<'i, I> {
    type Item = UpperPDU<ProvisionedStack>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(outer) = self.inner.next() {
            let mut should_take = false;

            let result = if let Some(next) = outer {
                match next {
                    QueueEntry::Nonsegmented(inner) => {
                        if inner.num_retransmit == 0 {
                            should_take = true;
                            if let Some(token) = inner.completion_token.as_ref() {
                                token.complete();
                            }
                        } else {
                            inner.num_retransmit -= 1;
                        }
                        Some(inner.upper_pdu.clone())
                    }
                    QueueEntry::Segmented(inner) => Some(inner.upper_pdu.clone()),
                }
            } else {
                None
            };

            if should_take {
                outer.take();
            }

            result
        } else {
            None
        }
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

    fn ack(&mut self, block_ack: BlockAck, _watchdog: &Watchdog) -> Result<bool, InvalidBlock> {
        for ack in block_ack.acked_iter() {
            self.block_ack.ack(ack)?;
        }

        Ok(self.block_ack.is_fully_acked(self.num_segments))
    }
}
