use crate::stack::provisioned::DriverError;
use btmesh_common::crypto::network::{NetworkKey, Nid};
use btmesh_device::NetworkKeyHandle;
use btmesh_models::foundation::configuration::NetKeyIndex;
use btmesh_pdu::provisioning::ProvisioningData;
use heapless::Vec;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
pub struct NetworkKeys<const N: usize = 4> {
    pub(crate) keys: Vec<Option<NetworkKey>, N>,
}

impl<const N: usize> Default for NetworkKeys<N> {
    fn default() -> Self {
        let mut keys = Vec::new();
        keys.resize(N, None).ok();
        Self { keys }
    }
}

impl<const N: usize> From<ProvisioningData> for NetworkKeys<N> {
    fn from(data: ProvisioningData) -> Self {
        let mut keys = Self::default();
        keys.keys[0].replace(NetworkKey::new(data.network_key).unwrap());
        keys
    }
}

impl<const N: usize> NetworkKeys<N> {
    pub fn display(&self) {
        for (index, key) in self.keys.iter().enumerate() {
            if let Some(key) = key {
                info!("network_key[{}]: {}", index, key);
            }
        }
    }

    pub(crate) fn by_nid_iter(&self, nid: Nid) -> impl Iterator<Item = NetworkKeyHandle> + '_ {
        self.keys
            .iter()
            .enumerate()
            .filter(move |e| {
                if let (_, Some(network_key)) = e {
                    network_key.nid() == nid
                } else {
                    false
                }
            })
            .map(move |(index, _)| NetworkKeyHandle::new(NetKeyIndex::new(index as u16), nid))
    }

    pub fn set(&mut self, index: u8, network_key: NetworkKey) -> Result<(), DriverError> {
        if index as usize >= N {
            return Err(DriverError::InsufficientSpace);
        }

        self.keys[index as usize].replace(network_key);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::stack::provisioned::secrets::network::NetworkKeys;
    use btmesh_common::crypto::network::{EncryptionKey, NetworkKey, Nid, PrivacyKey};

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
    fn network_key_derivation() {
        // 8.2.2 Encryption and privacy keys (Master)
        let network_key = NetworkKey::new([
            0x7d, 0xd7, 0x36, 0x4c, 0xd8, 0x42, 0xad, 0x18, 0xc1, 0x7c, 0x2b, 0x82, 0x0c, 0x84,
            0xc3, 0xd6,
        ])
        .unwrap();

        let encryption_key = EncryptionKey::new([
            0x09, 0x53, 0xfa, 0x93, 0xe7, 0xca, 0xac, 0x96, 0x38, 0xf5, 0x88, 0x20, 0x22, 0x0a,
            0x39, 0x8e,
        ]);

        let privacy_key = PrivacyKey::new([
            0x8b, 0x84, 0xee, 0xde, 0xc1, 0x00, 0x06, 0x7d, 0x67, 0x09, 0x71, 0xdd, 0x2a, 0xa7,
            0x00, 0xcf,
        ]);

        assert_eq!(Nid::new(0x68), network_key.nid());
        assert_eq!(privacy_key, network_key.privacy_key());
        assert_eq!(encryption_key, network_key.encryption_key());
    }
}
