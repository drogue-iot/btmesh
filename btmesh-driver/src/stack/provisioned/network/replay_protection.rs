use crate::stack::provisioned::ProvisionedStack;
use crate::UpperMetadata;
use btmesh_common::address::UnicastAddress;
use btmesh_common::Seq;
use btmesh_pdu::provisioned::lower::BlockAck;
use btmesh_pdu::provisioned::network::CleartextNetworkPDU;
use core::cmp::Ordering;
use uluru::LRUCache;

#[derive(PartialEq)]
struct NetworkCacheEntry {
    seq: Seq,
    src: UnicastAddress,
    iv_index: u16,
}

struct UpperCacheEntry {
    seq: Seq,
    src: UnicastAddress,
    iv_index: u16,
    block_ack: BlockAck,
}

#[derive(Default)]
pub struct ReplayProtection<const N: usize = 32> {
    network: LRUCache<NetworkCacheEntry, N>,
    upper: LRUCache<UpperCacheEntry, N>,
}

impl<const N: usize> ReplayProtection<N> {
    pub fn check_network_pdu(&mut self, pdu: &mut CleartextNetworkPDU<ProvisionedStack>) {
        let iv_index = (pdu.meta().iv_index().value() & 0xFFFF) as u16;

        if let Some(entry) = self.network.find(|e| e.src == pdu.src()) {
            match iv_index.cmp(&entry.iv_index) {
                Ordering::Less => {
                    pdu.meta_mut().replay_protected(true);
                }
                Ordering::Equal => {
                    if pdu.seq() <= entry.seq {
                        pdu.meta_mut().replay_protected(true);
                    } else {
                        entry.seq = pdu.seq();
                        pdu.meta_mut().replay_protected(false);
                    }
                }
                Ordering::Greater => {
                    entry.iv_index = iv_index;
                    entry.seq = pdu.seq();
                    pdu.meta_mut().replay_protected(false);
                }
            }
        } else {
            self.network.insert(NetworkCacheEntry {
                seq: pdu.seq(),
                src: pdu.src(),
                iv_index,
            });
            pdu.meta_mut().replay_protected(false);
        }
    }

    pub fn check_upper_pdu(
        &mut self,
        meta: &UpperMetadata,
        block_ack: &BlockAck,
        is_complete: bool,
    ) -> Option<BlockAck> {
        let iv_index = (meta.iv_index().value() & 0xFFFF) as u16;

        if let Some(entry) = self.upper.find(|e| e.src == meta.src()) {
            match iv_index.cmp(&entry.iv_index) {
                Ordering::Less => None,
                Ordering::Equal => {
                    if let Some(replay_seq) = meta.replay_seq() {
                        if replay_seq == entry.seq {
                            Some(entry.block_ack)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Ordering::Greater => {
                    if is_complete {
                        if let Some(replay_seq) = meta.replay_seq() {
                            entry.iv_index = iv_index;
                            entry.seq = replay_seq;
                            entry.block_ack = *block_ack;
                        }
                    }
                    None
                }
            }
        } else {
            if is_complete {
                if let Some(replay_seq) = meta.replay_seq() {
                    self.upper.insert(UpperCacheEntry {
                        seq: replay_seq,
                        src: meta.src(),
                        iv_index,
                        block_ack: *block_ack,
                    });
                }
            }
            None
        }
    }
}
