use crate::storage::provisioned::ProvisionedConfiguration;
use crate::storage::unprovisioned::UnprovisionedConfiguration;
use btmesh_pdu::provisioning::Capabilities;
use core::cell::RefCell;
use core::future::Future;
use core::hash::Hash;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub(crate) mod provisioned;
pub(crate) mod unprovisioned;

#[cfg(feature = "flash")]
pub mod flash;
#[cfg(feature = "memory")]
pub mod memory;

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

#[allow(clippy::large_enum_variant)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Hash, Debug)]
pub enum Configuration {
    Unprovisioned(UnprovisionedConfiguration),
    Provisioned(ProvisionedConfiguration),
}

pub struct Storage<B: BackingStore> {
    backing_store: RefCell<B>,
    capabilities: Option<Capabilities>,
    config: RefCell<Option<Configuration>>,
}

impl<B: BackingStore> Storage<B> {
    pub fn new(backing_store: B) -> Self {
        Self {
            backing_store: RefCell::new(backing_store),
            capabilities: None,
            config: RefCell::new(None),
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

    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn put(&self, config: &Configuration) -> Result<(), StorageError> {
        self.backing_store.borrow_mut().store(config).await?;
        self.config.borrow_mut().replace(config.clone());
        Ok(())
    }

    #[allow(clippy::await_holding_refcell_ref)]
    async fn load_if_needed(&self) -> Result<(), StorageError> {
        if self.config.borrow().is_none() {
            self.config
                .borrow_mut()
                .replace(self.backing_store.borrow_mut().load().await?);
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
        unwrap!(self.capabilities.clone())
    }

    pub fn set_capabilities(&mut self, capabilities: Capabilities) {
        self.capabilities.replace(capabilities);
    }
}
