use crate::storage::{BackingStore, StorageError};
use crate::util::hash::hash_of;
use crate::ProvisionedConfiguration;
use core::future::Future;
use embedded_storage_async::nor_flash::AsyncNorFlash;
use postcard::{from_bytes, to_slice};

#[repr(align(4))]
struct AlignedBuffer<const N: usize>([u8; N]);

#[derive(Copy, Clone)]
pub enum LatestLoad {
    None,
    Provisioned { hash: u64, sequence: u32 },
}

pub struct FlashBackingStore<F: AsyncNorFlash, const PAGE_SIZE: u32 = 4096> {
    flash: F,
    base_address: u32,
    latest_load: LatestLoad,
    sequence_threshold: u32,
    buffer: AlignedBuffer<USEFUL_BUFFER_SIZE>,
}

impl<F: AsyncNorFlash, const PAGE_SIZE: u32> FlashBackingStore<F, PAGE_SIZE> {
    pub fn new(flash: F, base_address: u32, sequence_threshold: u32) -> Self {
        Self {
            flash,
            base_address,
            latest_load: LatestLoad::None,
            sequence_threshold,
            buffer: AlignedBuffer([0; USEFUL_BUFFER_SIZE]),
        }
    }
}

const USEFUL_BUFFER_SIZE: usize = 2048;

impl<F: AsyncNorFlash, const PAGE_SIZE: u32> BackingStore for FlashBackingStore<F, PAGE_SIZE> {
    type LoadFuture<'m> =  impl Future<Output = Result<ProvisionedConfiguration, StorageError>> + 'm
        where
            Self: 'm;
    type StoreFuture<'m> = impl Future<Output = Result<(), StorageError>> + 'm
        where
            Self: 'm;
    type ClearFuture<'m> = impl Future<Output = Result<(), StorageError>> + 'm
        where
            Self: 'm;

    fn load(&mut self) -> Self::LoadFuture<'_> {
        async move {
            self.flash
                .read(self.base_address, &mut self.buffer.0)
                .await
                .map_err(|_| StorageError::Load)?;

            let config: ProvisionedConfiguration =
                from_bytes(&self.buffer.0).map_err(|_| StorageError::Serialization)?;

            let hash = hash_of(&config);
            self.latest_load = LatestLoad::Provisioned {
                hash,
                sequence: config.sequence(),
            };

            Ok(config)
        }
    }

    fn store<'f>(&'f mut self, config: &'f ProvisionedConfiguration) -> Self::StoreFuture<'f> {
        async move {
            if should_writeback(self.latest_load, config, self.sequence_threshold) {
                to_slice(config, &mut self.buffer.0).map_err(|_| StorageError::Serialization)?;
                self.flash
                    .erase(self.base_address, self.base_address + PAGE_SIZE)
                    .await
                    .map_err(|_| StorageError::Store)?;
                self.flash
                    .write(self.base_address, &self.buffer.0)
                    .await
                    .map_err(|_| StorageError::Store)?;

                self.latest_load = LatestLoad::Provisioned {
                    hash: hash_of(config),
                    sequence: config.sequence(),
                };
            }
            Ok(())
        }
    }

    fn clear(&mut self) -> Self::ClearFuture<'_> {
        async move {
            self.flash
                .erase(
                    self.base_address,
                    self.base_address + USEFUL_BUFFER_SIZE as u32,
                )
                .await
                .map_err(|_| StorageError::Store)?;
            self.latest_load = LatestLoad::None;
            Ok(())
        }
    }
}

#[allow(clippy::needless_bool)]
pub fn should_writeback(
    current: LatestLoad,
    new: &ProvisionedConfiguration,
    sequence_threshold: u32,
) -> bool {
    match (current, new) {
        (LatestLoad::None, _) => {
            // we had nothing, so scribble.
            true
        }
        (LatestLoad::Provisioned { hash, sequence }, new_provisioned_config) => {
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
    use crate::{Configuration, DeviceInfo, NetworkState, Secrets};
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

    pub fn should_writeback_from_none() {
        let provisioned_config = ProvisionedConfiguration::new(
            0,
            NetworkState::new(IvIndex::new(100), IvUpdateFlag::Normal),
            Secrets::new(
                DeviceKey::new([
                    0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
                    0xEE, 0xFF, 0x00,
                ]),
                NetworkKeys::default(),
                ApplicationKeys::default(),
            ),
            DeviceInfo::new(UnicastAddress::new(0x00A1).unwrap(), 1),
            Default::default(),
        );

        assert!(should_writeback(LatestLoad::None, &provisioned_config, 100))
    }

    #[test]
    pub fn should_writeback_provisioned_sequence_unchanged() {
        let provisioned_config = ProvisionedConfiguration::new(
            100,
            NetworkState::new(IvIndex::new(100), IvUpdateFlag::Normal),
            Secrets::new(
                DeviceKey::new([
                    0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
                    0xEE, 0xFF, 0x00,
                ]),
                NetworkKeys::default(),
                ApplicationKeys::default(),
            ),
            DeviceInfo::new(UnicastAddress::new(0x00A1).unwrap(), 1),
            Default::default(),
        );

        let hash = hash_of(&provisioned_config);

        assert!(!should_writeback(
            LatestLoad::Provisioned {
                hash,
                sequence: 100
            },
            &provisioned_config,
            100
        ))
    }

    #[test]
    pub fn should_writeback_provisioned_sequence_changed_threshold_not_met() {
        let provisioned_config = ProvisionedConfiguration::new(
            199,
            NetworkState::new(IvIndex::new(100), IvUpdateFlag::Normal),
            Secrets::new(
                DeviceKey::new([
                    0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
                    0xEE, 0xFF, 0x00,
                ]),
                NetworkKeys::default(),
                ApplicationKeys::default(),
            ),
            DeviceInfo::new(UnicastAddress::new(0x00A1).unwrap(), 1),
            Default::default(),
        );

        assert!(!should_writeback(
            LatestLoad::Provisioned {
                hash: hash_of(&provisioned_config),
                sequence: 100
            },
            &provisioned_config,
            100
        ))
    }

    #[test]
    pub fn should_writeback_provisioned_sequence_changed_threshold_is_met() {
        let provisioned_config = ProvisionedConfiguration::new(
            200,
            NetworkState::new(IvIndex::new(100), IvUpdateFlag::Normal),
            Secrets::new(
                DeviceKey::new([
                    0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
                    0xEE, 0xFF, 0x00,
                ]),
                NetworkKeys::default(),
                ApplicationKeys::default(),
            ),
            DeviceInfo::new(UnicastAddress::new(0x00A1).unwrap(), 1),
            Default::default(),
        );

        assert!(should_writeback(
            LatestLoad::Provisioned {
                hash: hash_of(&provisioned_config),
                sequence: 100,
            },
            &provisioned_config,
            100
        ))
    }

    #[test]
    pub fn should_writeback_provisioned_sequence_changed_threshold_is_met_skippingly() {
        let provisioned_config = ProvisionedConfiguration::new(
            205,
            NetworkState::new(IvIndex::new(100), IvUpdateFlag::Normal),
            Secrets::new(
                DeviceKey::new([
                    0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
                    0xEE, 0xFF, 0x00,
                ]),
                NetworkKeys::default(),
                ApplicationKeys::default(),
            ),
            DeviceInfo::new(UnicastAddress::new(0x00A1).unwrap(), 1),
            Default::default(),
        );

        assert!(should_writeback(
            LatestLoad::Provisioned {
                hash: hash_of(&provisioned_config),
                sequence: 100,
            },
            &provisioned_config,
            100
        ))
    }
}
