use crate::storage::provisioned::ProvisionedConfiguration;
use crate::storage::unprovisioned::UnprovisionedConfiguration;
use crate::util::hash::{hash_of, FnvHasher};
use btmesh_pdu::provisioning::Capabilities;
use core::cell::RefCell;
use core::future::Future;
use core::hash::{Hash, Hasher};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub(crate) mod provisioned;
pub(crate) mod unprovisioned;

#[cfg(feature = "flash")]
mod flash;
#[cfg(feature = "memory")]
mod memory;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum StorageError {
    Load,
    Store,
    Serialization,
    Deserialization,
}

pub trait BackingStore {
    type LoadFuture<'m>: Future<Output = Result<Configuration, StorageError>> + 'm
    where
        Self: 'm;

    type StoreFuture<'m>: Future<Output = Result<(), StorageError>> + 'm
    where
        Self: 'm;

    fn load(&mut self) -> Self::LoadFuture<'_>;
    fn store<'f>(&'f mut self, config: &'f Configuration) -> Self::StoreFuture<'f>;
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Hash, Debug)]
pub enum Configuration {
    Unprovisioned(UnprovisionedConfiguration),
    Provisioned(ProvisionedConfiguration),
}

pub struct Storage<B: BackingStore> {
    backing_store: RefCell<B>,
    capabilities: Capabilities,
    config: RefCell<Option<Configuration>>,
    sequence_threshold: u32,
}

impl<B: BackingStore> Storage<B> {
    pub fn new(backing_store: B, capabilities: Capabilities, sequence_threshold: u32) -> Self {
        Self {
            backing_store: RefCell::new(backing_store),
            capabilities,
            config: RefCell::new(None),
            sequence_threshold,
        }
    }

    pub async fn get(&self) -> Result<Configuration, StorageError> {
        self.load_if_needed().await?;
        if let Some(config) = &*self.config.borrow() {
            Ok(config.clone())
        } else {
            Err(StorageError::Load)
        }
    }

    pub async fn put(&self, config: &Configuration) -> Result<(), StorageError> {
        if matches!(config, Configuration::Unprovisioned(..)) {
            return Ok(());
        }

        if should_writeback(
            self.config.borrow().as_ref().map(|inner| inner),
            config,
            self.sequence_threshold,
        ) {
            self.backing_store.borrow_mut().store(config).await?;
            self.config.borrow_mut().replace(config.clone());
        }

        Ok(())
    }

    async fn load_if_needed(&self) -> Result<(), StorageError> {
        if let None = &*self.config.borrow() {
            let config = self.backing_store.borrow_mut().load().await?;
            match &config {
                Configuration::Unprovisioned(..) => {
                    self.config.borrow_mut().replace(config);
                }
                Configuration::Provisioned(..) => {
                    self.config.borrow_mut().replace(config);
                }
            }
        }

        Ok(())
    }

    pub async fn is_provisioned(&self) -> Result<bool, StorageError> {
        self.load_if_needed().await?;
        Ok(matches!(
            &*self.config.borrow(),
            Some(Configuration::Provisioned(..))
        ))
    }

    pub async fn is_unprovisioned(&self) -> Result<bool, StorageError> {
        self.load_if_needed().await?;
        Ok(matches!(
            &*self.config.borrow(),
            Some(Configuration::Unprovisioned(..))
        ))
    }

    pub fn capabilities(&self) -> Capabilities {
        self.capabilities.clone()
    }
}

pub fn should_writeback(
    current: Option<&Configuration>,
    new: &Configuration,
    sequence_threshold: u32,
) -> bool {
    match (current, new) {
        (None, _) => {
            // we had nothing, so scribble.
            true
        }
        (Some(Configuration::Unprovisioned(..)), Configuration::Provisioned(..)) => {
            // unprovisioned -> provisioned
            true
        }
        (Some(Configuration::Provisioned(..)), Configuration::Unprovisioned(..)) => {
            // provisioned -> unprovisioned
            true
        }
        (
            Some(current @ Configuration::Provisioned(current_provisioned_config)),
            Configuration::Provisioned(new_provisioned_config),
        ) => {
            let current_hash = hash_of(current);
            let new_hash = hash_of(new);
            if new_hash != current_hash {
                true
            } else {
                if new_provisioned_config.sequence() == current_provisioned_config.sequence() {
                    false
                } else if new_provisioned_config.sequence() % sequence_threshold == 0
                    || (new_provisioned_config.sequence() - current_provisioned_config.sequence())
                        >= sequence_threshold
                {
                    true
                } else {
                    false
                }
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod test {
    use crate::stack::provisioned::secrets::application::ApplicationKeys;
    use crate::stack::provisioned::secrets::network::NetworkKeys;
    use crate::storage::provisioned::ProvisionedConfiguration;
    use crate::storage::should_writeback;
    use crate::storage::unprovisioned::UnprovisionedConfiguration;
    use crate::util::hash::hash_of;
    use crate::{Configuration, DeviceInfo, NetworkState, Secrets, Storage};
    use btmesh_common::address::UnicastAddress;
    use btmesh_common::crypto::device::DeviceKey;
    use btmesh_common::{IvIndex, IvUpdateFlag, Uuid};

    #[test]
    pub fn hashing() {
        let config_a = Configuration::Unprovisioned(UnprovisionedConfiguration {
            uuid: Uuid::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
        });

        let config_b = Configuration::Unprovisioned(UnprovisionedConfiguration {
            uuid: Uuid::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
        });

        assert_eq!(hash_of(&config_a), hash_of(&config_b));
    }

    #[test]
    pub fn should_writeback_from_none() {
        let unprovisioned_config = Configuration::Unprovisioned(UnprovisionedConfiguration {
            uuid: Uuid::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
        });

        assert_eq!(true, should_writeback(None, &unprovisioned_config, 100))
    }

    #[test]
    pub fn should_writeback_from_unprovisioned_to_provisioned() {
        let unprovisioned_config = Configuration::Unprovisioned(UnprovisionedConfiguration {
            uuid: Uuid::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
        });

        let provisioned_config = Configuration::Provisioned(ProvisionedConfiguration {
            network_state: NetworkState::new(IvIndex::new(100), IvUpdateFlag::Normal),
            secrets: Secrets::new(
                DeviceKey::new([
                    0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
                    0xEE, 0xFF, 0x00,
                ]),
                NetworkKeys::default(),
                ApplicationKeys::default(),
            ),
            device_info: DeviceInfo::new(UnicastAddress::new(0x00A1).unwrap(), 1),
            sequence: 0,
        });

        assert_eq!(
            true,
            should_writeback(Some(&unprovisioned_config), &provisioned_config, 100)
        )
    }

    #[test]
    pub fn should_writeback_provisioned_sequence_unchanged() {
        let provisioned_config = Configuration::Provisioned(ProvisionedConfiguration {
            network_state: NetworkState::new(IvIndex::new(100), IvUpdateFlag::Normal),
            secrets: Secrets::new(
                DeviceKey::new([
                    0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
                    0xEE, 0xFF, 0x00,
                ]),
                NetworkKeys::default(),
                ApplicationKeys::default(),
            ),
            device_info: DeviceInfo::new(UnicastAddress::new(0x00A1).unwrap(), 1),
            sequence: 100,
        });

        assert_eq!(
            false,
            should_writeback(Some(&provisioned_config), &provisioned_config, 100)
        )
    }

    #[test]
    pub fn should_writeback_provisioned_sequence_changed_threshold_not_met() {
        let provisioned_config_a = Configuration::Provisioned(ProvisionedConfiguration {
            network_state: NetworkState::new(IvIndex::new(100), IvUpdateFlag::Normal),
            secrets: Secrets::new(
                DeviceKey::new([
                    0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
                    0xEE, 0xFF, 0x00,
                ]),
                NetworkKeys::default(),
                ApplicationKeys::default(),
            ),
            device_info: DeviceInfo::new(UnicastAddress::new(0x00A1).unwrap(), 1),
            sequence: 100,
        });

        let provisioned_config_b = Configuration::Provisioned(ProvisionedConfiguration {
            network_state: NetworkState::new(IvIndex::new(100), IvUpdateFlag::Normal),
            secrets: Secrets::new(
                DeviceKey::new([
                    0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
                    0xEE, 0xFF, 0x00,
                ]),
                NetworkKeys::default(),
                ApplicationKeys::default(),
            ),
            device_info: DeviceInfo::new(UnicastAddress::new(0x00A1).unwrap(), 1),
            sequence: 199,
        });

        assert_eq!(
            false,
            should_writeback(Some(&provisioned_config_a), &provisioned_config_b, 100)
        )
    }

    #[test]
    pub fn should_writeback_provisioned_sequence_changed_threshold_is_met() {
        let provisioned_config_a = Configuration::Provisioned(ProvisionedConfiguration {
            network_state: NetworkState::new(IvIndex::new(100), IvUpdateFlag::Normal),
            secrets: Secrets::new(
                DeviceKey::new([
                    0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
                    0xEE, 0xFF, 0x00,
                ]),
                NetworkKeys::default(),
                ApplicationKeys::default(),
            ),
            device_info: DeviceInfo::new(UnicastAddress::new(0x00A1).unwrap(), 1),
            sequence: 100,
        });

        let provisioned_config_b = Configuration::Provisioned(ProvisionedConfiguration {
            network_state: NetworkState::new(IvIndex::new(100), IvUpdateFlag::Normal),
            secrets: Secrets::new(
                DeviceKey::new([
                    0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
                    0xEE, 0xFF, 0x00,
                ]),
                NetworkKeys::default(),
                ApplicationKeys::default(),
            ),
            device_info: DeviceInfo::new(UnicastAddress::new(0x00A1).unwrap(), 1),
            sequence: 200,
        });

        assert_eq!(
            true,
            should_writeback(Some(&provisioned_config_a), &provisioned_config_b, 100)
        )
    }

    #[test]
    pub fn should_writeback_provisioned_sequence_changed_threshold_is_met_skippingly() {
        let provisioned_config_a = Configuration::Provisioned(ProvisionedConfiguration {
            network_state: NetworkState::new(IvIndex::new(100), IvUpdateFlag::Normal),
            secrets: Secrets::new(
                DeviceKey::new([
                    0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
                    0xEE, 0xFF, 0x00,
                ]),
                NetworkKeys::default(),
                ApplicationKeys::default(),
            ),
            device_info: DeviceInfo::new(UnicastAddress::new(0x00A1).unwrap(), 1),
            sequence: 100,
        });

        let provisioned_config_b = Configuration::Provisioned(ProvisionedConfiguration {
            network_state: NetworkState::new(IvIndex::new(100), IvUpdateFlag::Normal),
            secrets: Secrets::new(
                DeviceKey::new([
                    0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
                    0xEE, 0xFF, 0x00,
                ]),
                NetworkKeys::default(),
                ApplicationKeys::default(),
            ),
            device_info: DeviceInfo::new(UnicastAddress::new(0x00A1).unwrap(), 1),
            sequence: 205,
        });

        assert_eq!(
            true,
            should_writeback(Some(&provisioned_config_a), &provisioned_config_b, 100)
        )
    }
}
