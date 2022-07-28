use crate::storage::provisioned::ProvisionedConfiguration;
use crate::storage::unprovisioned::UnprovisionedConfiguration;
use crate::util::hash::FnvHasher;
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
#[derive(Clone, Hash)]
pub enum Configuration {
    Unprovisioned(UnprovisionedConfiguration),
    Provisioned(ProvisionedConfiguration),
}

pub struct Storage<B: BackingStore> {
    backing_store: RefCell<B>,
    capabilities: Capabilities,
    config: RefCell<Option<(Configuration, u64)>>,
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
            Ok(config.0.clone())
        } else {
            Err(StorageError::Load)
        }
    }

    pub async fn put(&self, config: &Configuration) -> Result<(), StorageError> {
        if matches!(config, Configuration::Unprovisioned(..)) {
            return Ok(());
        }

        match (&*self.config.borrow(), config) {
            (None, _) => {
                // we had nothing, so scribble.
                self.store_internal(config, Self::hash_of(config)).await?;
            }
            (Some((Configuration::Unprovisioned(..), _)), Configuration::Provisioned(..)) => {
                // unprovisioned -> provisioned
                self.store_internal(config, Self::hash_of(config)).await?;
            }
            (Some((Configuration::Provisioned(..), _)), Configuration::Unprovisioned(..)) => {
                // provisioned -> unprovisioned
                self.store_internal(config, Self::hash_of(config)).await?;
            }
            (
                Some((Configuration::Provisioned(..), hash)),
                Configuration::Provisioned(new_provisioned_config),
            ) => {
                let new_hash = Self::hash_of(config);
                if new_hash != *hash
                    || new_provisioned_config.sequence() % self.sequence_threshold == 0
                {
                    self.store_internal(config, new_hash).await?;
                }
            }
            _ => {
                // shouldn't reach here, I guess.
            }
        }

        Ok(())
    }

    async fn store_internal(&self, config: &Configuration, hash: u64) -> Result<(), StorageError> {
        self.backing_store.borrow_mut().store(config).await?;
        self.config.borrow_mut().replace((config.clone(), hash));
        Ok(())
    }

    fn hash_of(config: &Configuration) -> u64 {
        let mut hasher = FnvHasher::default();
        config.hash(&mut hasher);
        hasher.finish()
    }

    async fn load_if_needed(&self) -> Result<(), StorageError> {
        if let None = &*self.config.borrow() {
            let config = self.backing_store.borrow_mut().load().await?;
            match &config {
                Configuration::Unprovisioned(..) => {
                    self.config.borrow_mut().replace((config, 0));
                }
                Configuration::Provisioned(..) => {
                    let hash = Self::hash_of(&config);
                    self.config.borrow_mut().replace((config, hash));
                }
            }
        }

        Ok(())
    }

    pub async fn is_provisioned(&self) -> Result<bool, StorageError> {
        self.load_if_needed().await?;
        Ok(matches!(
            &*self.config.borrow(),
            Some((Configuration::Provisioned(..), _))
        ))
    }

    pub async fn is_unprovisioned(&self) -> Result<bool, StorageError> {
        self.load_if_needed().await?;
        Ok(matches!(
            &*self.config.borrow(),
            Some((Configuration::Unprovisioned(..), _))
        ))
    }

    pub fn capabilities(&self) -> Capabilities {
        self.capabilities.clone()
    }
}
