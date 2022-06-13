use crate::Driver;
use btmesh_common::address::UnicastAddress;
use btmesh_common::Seq;
use btmesh_pdu::network::CleartextNetworkPDU;
use uluru::LRUCache;

#[derive(PartialEq)]
struct CacheEntry {
    seq: Seq,
    src: UnicastAddress,
    iv_index: u16,
}

pub struct ReplayProtection<const N: usize = 100> {
    lru: LRUCache<CacheEntry, N>,
}

impl<const N: usize> Default for ReplayProtection<N> {
    fn default() -> Self {
        Self {
            lru: Default::default(),
        }
    }
}

impl<const N: usize> ReplayProtection<N> {
    pub fn check(&mut self, pdu: &mut CleartextNetworkPDU<Driver>) {
        let iv_index = (pdu.meta().iv_index().value() & 0xFFFF) as u16;

        if let Some(entry) = self.lru.find(|e| e.src == pdu.src()) {
            if iv_index < entry.iv_index {
                pdu.meta_mut().replay_protected(true);
            } else if iv_index == entry.iv_index {
                if pdu.seq() <= entry.seq {
                    pdu.meta_mut().replay_protected(true);
                } else {
                    entry.seq = pdu.seq();
                    pdu.meta_mut().replay_protected(false);
                }
            } else {
                entry.iv_index = iv_index;
                entry.seq = pdu.seq();
                pdu.meta_mut().replay_protected(false);
            }
        } else {
            self.lru.insert(CacheEntry {
                seq: pdu.seq(),
                src: pdu.src(),
                iv_index,
            });
            pdu.meta_mut().replay_protected(false);
        }
    }
}
