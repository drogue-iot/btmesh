use crate::{Driver, DriverError, NetworkKeyHandle};
use btmesh_common::Nid;
use core::iter::Filter;
use core::slice::Iter;

pub(crate) struct Secrets {
    network_keys: NetworkKeys,
}

impl Secrets {
    pub(crate) fn network_keys_by_nid(
        &self,
        nid: Nid,
    ) -> NetworkKeyIter<'_, Iter<'_, Option<NetworkKey>>> {
        self.network_keys.by_nid_iter(nid)
    }

    pub(crate) fn privacy_key(
        &self,
        network_key: NetworkKeyHandle,
    ) -> Result<[u8; 16], DriverError> {
        self.network_keys.keys[network_key.0 as usize]
            .as_ref()
            .ok_or(DriverError::InvalidKeyHandle)
            .map(|key| key.privacy_key)
    }

    pub(crate) fn encryption_key(
        &self,
        network_key: NetworkKeyHandle,
    ) -> Result<[u8; 16], DriverError> {
        self.network_keys.keys[network_key.0 as usize]
            .as_ref()
            .ok_or(DriverError::InvalidKeyHandle)
            .map(|key| key.encryption_key)
    }
}

struct NetworkKeys<const N: usize = 4> {
    keys: [Option<NetworkKey>; N],
}

impl<const N: usize> Default for NetworkKeys<N> {
    fn default() -> Self {
        let keys = [None; N];
        Self { keys }
    }
}

impl<const N: usize> NetworkKeys<N> {
    fn by_nid_iter(&self, nid: Nid) -> NetworkKeyIter<'_, Iter<'_, Option<NetworkKey>>> {
        NetworkKeyIter {
            iter: self.keys.iter(),
            nid,
            index: 0,
        }
    }

    fn set(&mut self, index: u8, network_key: NetworkKey) -> Result<(), DriverError> {
        if index as usize >= N {
            Err(DriverError::InsufficientSpace)?
        }

        self.keys[index as usize].replace(network_key);

        Ok(())
    }
}

#[derive(Copy, Clone)]
pub(crate) struct NetworkKey {
    privacy_key: [u8; 16],
    encryption_key: [u8; 16],
    nid: Nid,
}

pub(crate) struct NetworkKeyIter<'i, I: Iterator<Item = &'i Option<NetworkKey>>> {
    iter: I,
    nid: Nid,
    index: u8,
}

impl<'i, I: Iterator<Item = &'i Option<NetworkKey>>> Iterator for NetworkKeyIter<'i, I> {
    type Item = NetworkKeyHandle;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(slot) = self.iter.next() {
                if let Some(key) = slot {
                    if key.nid == self.nid {
                        return Some(NetworkKeyHandle(self.index));
                    } // else loop
                } // else loop
                self.index += 1;
            } else {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::secrets::{NetworkKey, NetworkKeys};
    use btmesh_common::Nid;

    #[test]
    fn network_key_iteration_empty() {
        let keys = NetworkKeys::<4>::default();

        let mut found = 0;

        for _ in keys.by_nid_iter(Nid::new(42)) {
            found += 1;
        }

        assert_eq!(0, found)
    }

    #[test]
    fn network_key_iteration() {
        let mut keys = NetworkKeys::<4>::default();

        keys.set(
            0,
            NetworkKey {
                privacy_key: Default::default(),
                encryption_key: Default::default(),
                nid: Nid::new(42),
            },
        )
        .unwrap();

        keys.set(
            1,
            NetworkKey {
                privacy_key: Default::default(),
                encryption_key: Default::default(),
                nid: Nid::new(18),
            },
        )
        .unwrap();

        keys.set(
            2,
            NetworkKey {
                privacy_key: Default::default(),
                encryption_key: Default::default(),
                nid: Nid::new(42),
            },
        )
        .unwrap();

        keys.set(
            3,
            NetworkKey {
                privacy_key: Default::default(),
                encryption_key: Default::default(),
                nid: Nid::new(18),
            },
        )
        .unwrap();

        let mut found = 0;

        for _ in keys.by_nid_iter(Nid::new(42)) {
            found += 1;
        }

        assert_eq!(2, found);

        found = 0;

        for _ in keys.by_nid_iter(Nid::new(44)) {
            found += 1;
        }

        assert_eq!(0, found);
    }
}
