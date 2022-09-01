use crate::{hash_of, ProvisionedStack};
use btmesh_common::address::UnicastAddress;
use btmesh_pdu::provisioned::network::CleartextNetworkPDU;
use heapless::Vec;
use uluru::LRUCache;

#[derive(Default)]
pub struct NetworkMessageCache<const N: usize = 32> {
    cache: LRUCache<CacheEntry, N>,
}

pub struct CacheEntry {
    src: UnicastAddress,
    hash: u64,
}

impl NetworkMessageCache {
    pub fn check(&mut self, pdu: &mut CleartextNetworkPDU<ProvisionedStack>) {
        if let Ok(content) = Vec::<_, 16>::from_slice(pdu.transport_pdu()) {
            let hash = hash_of(&content);

            if self
                .cache
                .find(|e| e.src == pdu.src() && e.hash == hash)
                .is_some()
            {
                pdu.meta_mut().should_relay(false);
            } else {
                self.cache.insert(CacheEntry {
                    src: pdu.src(),
                    hash,
                });
                pdu.meta_mut().should_relay(true);
            }
        } else {
            pdu.meta_mut().should_relay(false);
        }
    }
}
