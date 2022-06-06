use crate::{Driver, DriverError, NetworkKeyHandle};
use btmesh_common::{crypto, Nid};
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

impl NetworkKey {
    pub fn new(network_key: [u8; 16]) -> Result<Self, DriverError> {
        let (nid, encryption_key, privacy_key) =
            crypto::k2(&network_key, &[0x00]).map_err(|_| DriverError::CryptoError)?;

        Ok(Self {
            privacy_key,
            encryption_key,
            nid: Nid::new(nid),
        })
    }
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

    #[test]
    fn network_key_derivation() {
        // 8.2.2 Encryption and privacy keys (Master)
        let network_key: [u8; 16] = [
            0x7d, 0xd7, 0x36, 0x4c, 0xd8, 0x42, 0xad, 0x18, 0xc1, 0x7c, 0x2b, 0x82, 0x0c, 0x84,
            0xc3, 0xd6,
        ];

        let encryption_key: [u8; 16] = [
            0x09, 0x53, 0xfa, 0x93, 0xe7, 0xca, 0xac, 0x96, 0x38, 0xf5, 0x88, 0x20, 0x22, 0x0a,
            0x39, 0x8e,
        ];

        let privacy_key: [u8; 16] = [
            0x8b, 0x84, 0xee, 0xde, 0xc1, 0x00, 0x06, 0x7d, 0x67, 0x09, 0x71, 0xdd, 0x2a, 0xa7,
            0x00, 0xcf,
        ];

        let network_key = NetworkKey::new(network_key).unwrap();

        assert_eq!(Nid::new(0x68), network_key.nid);
        assert_eq!(privacy_key, network_key.privacy_key);
        assert_eq!(encryption_key, network_key.encryption_key);
    }
}
