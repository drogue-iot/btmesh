use crate::storage::{BackingStore, Configuration, StorageError};
use crate::util::hash::hash_of;
use core::future::Future;
use embedded_storage_async::nor_flash::AsyncNorFlash;
use postcard::{from_bytes, to_slice};

#[derive(Copy, Clone)]
pub enum LatestLoad {
    None,
    Unprovisioned,
    Provisioned { hash: u64, sequence: u32 },
}

pub struct FlashBackingStore<F: AsyncNorFlash> {
    flash: F,
    base_address: u32,
    latest_load: LatestLoad,
    sequence_threshold: u32,
}

impl<F: AsyncNorFlash> FlashBackingStore<F> {
    pub fn new(flash: F, base_address: u32, sequence_threshold: u32) -> Self {
        Self {
            flash,
            base_address,
            latest_load: LatestLoad::None,
            sequence_threshold,
        }
    }
}

impl<F: AsyncNorFlash> BackingStore for FlashBackingStore<F> {
    type LoadFuture<'m> =  impl Future<Output = Result<Configuration, StorageError>> + 'm
        where
            Self: 'm;
    type StoreFuture<'m> = impl Future<Output = Result<(), StorageError>> + 'm
        where
            Self: 'm;

    fn load(&mut self) -> Self::LoadFuture<'_> {
        async move {
            let mut bytes = [0; 4096];
            self.flash
                .read(self.base_address, &mut bytes)
                .await
                .map_err(|_| StorageError::Load)?;
            let config = from_bytes(&bytes).map_err(|_| StorageError::Serialization)?;

            match &config {
                Configuration::Unprovisioned(_) => {
                    self.latest_load = LatestLoad::Unprovisioned;
                }
                Configuration::Provisioned(config) => {
                    let hash = hash_of(&config);
                    self.latest_load = LatestLoad::Provisioned {
                        hash,
                        sequence: config.sequence(),
                    };
                }
            }

            Ok(config)
        }
    }

    fn store<'f>(&'f mut self, config: &'f Configuration) -> Self::StoreFuture<'f> {
        async move {
            if should_writeback(self.latest_load, config, self.sequence_threshold) {
                let mut bytes = [0; 4096];
                to_slice(config, &mut bytes).map_err(|_| StorageError::Serialization)?;
                self.flash
                    .write(self.base_address, &bytes)
                    .await
                    .map_err(|_| StorageError::Store)?;

                self.latest_load = match config {
                    Configuration::Unprovisioned(_) => LatestLoad::Unprovisioned,
                    Configuration::Provisioned(provisioned_config) => LatestLoad::Provisioned {
                        hash: hash_of(config),
                        sequence: provisioned_config.sequence(),
                    },
                }
            }
            Ok(())
        }
    }
}

#[allow(clippy::needless_bool)]
pub fn should_writeback(current: LatestLoad, new: &Configuration, sequence_threshold: u32) -> bool {
    match (current, new) {
        (LatestLoad::None, _) => {
            // we had nothing, so scribble.
            true
        }
        (LatestLoad::Unprovisioned, Configuration::Provisioned(..)) => {
            // unprovisioned -> provisioned
            true
        }
        (LatestLoad::Provisioned { .. }, Configuration::Unprovisioned(..)) => {
            // provisioned -> unprovisioned
            true
        }
        (
            LatestLoad::Provisioned { hash, sequence },
            Configuration::Provisioned(new_provisioned_config),
        ) => {
            let new_hash = hash_of(new);
            if new_hash != hash {
                true
            } else if new_provisioned_config.sequence() == sequence {
                false
            } else if new_provisioned_config.sequence() % sequence_threshold == 0
                || (new_provisioned_config.sequence() - sequence) >= sequence_threshold
            {
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod test {
    use crate::stack::provisioned::secrets::application::ApplicationKeys;
    use crate::stack::provisioned::secrets::network::NetworkKeys;
    use crate::storage::flash::{should_writeback, LatestLoad};
    use crate::storage::provisioned::ProvisionedConfiguration;
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

        assert_eq!(
            true,
            should_writeback(LatestLoad::None, &unprovisioned_config, 100)
        )
    }

    #[test]
    pub fn should_writeback_from_unprovisioned_to_provisioned() {
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
            should_writeback(LatestLoad::Unprovisioned, &provisioned_config, 100)
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

        let hash = hash_of(&provisioned_config);

        assert_eq!(
            false,
            should_writeback(
                LatestLoad::Provisioned {
                    hash,
                    sequence: 100
                },
                &provisioned_config,
                100
            )
        )
    }

    #[test]
    pub fn should_writeback_provisioned_sequence_changed_threshold_not_met() {
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
            sequence: 199,
        });

        assert_eq!(
            false,
            should_writeback(
                LatestLoad::Provisioned {
                    hash: hash_of(&provisioned_config),
                    sequence: 100
                },
                &provisioned_config,
                100
            )
        )
    }

    #[test]
    pub fn should_writeback_provisioned_sequence_changed_threshold_is_met() {
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
            sequence: 200,
        });

        assert_eq!(
            true,
            should_writeback(
                LatestLoad::Provisioned {
                    hash: hash_of(&provisioned_config),
                    sequence: 100,
                },
                &provisioned_config,
                100
            )
        )
    }

    #[test]
    pub fn should_writeback_provisioned_sequence_changed_threshold_is_met_skippingly() {
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
            sequence: 205,
        });

        assert_eq!(
            true,
            should_writeback(
                LatestLoad::Provisioned {
                    hash: hash_of(&provisioned_config),
                    sequence: 100,
                },
                &provisioned_config,
                100
            )
        )
    }
}
