use crate::storage::provisioned::ProvisionedConfiguration;
use crate::storage::unprovisioned::UnprovisionedConfiguration;
use btmesh_common::Composition;
use btmesh_pdu::provisioning::Capabilities;
use core::cell::RefCell;
use core::future::Future;
use core::hash::Hash;
use embassy_util::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_util::mutex::Mutex;
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
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[derive(Clone, Hash, Debug)]
pub enum Configuration {
    Unprovisioned(UnprovisionedConfiguration),
    Provisioned(ProvisionedConfiguration),
}

impl Configuration {
    pub fn display(&self, composition: &Composition) {
        info!("========================================================================");
        match self {
            Configuration::Unprovisioned(config) => config.display(composition),
            Configuration::Provisioned(config) => config.display(composition),
        }
        info!("========================================================================");
    }
}

pub struct Storage<B: BackingStore> {
    backing_store: RefCell<B>,
    capabilities: RefCell<Option<Capabilities>>,
    composition: RefCell<Option<Composition>>,
    config: Mutex<CriticalSectionRawMutex, Option<Configuration>>,
}

impl<B: BackingStore> Storage<B> {
    pub fn new(backing_store: B) -> Self {
        Self {
            backing_store: RefCell::new(backing_store),
            capabilities: RefCell::new(None),
            composition: RefCell::new(None),
            config: Mutex::new(None),
        }
    }

    pub async fn init(&self) -> Result<(), StorageError> {
        if let Ok(Configuration::Provisioned(mut config)) = self.get().await {
            let seq = config.sequence;

            let mut extra = seq % 100;
            if extra == 100 {
                extra = 0;
            }
            let seq = (seq - extra) + 100;

            config.sequence = seq;
            self.put(&(config.into())).await?;
        }

        Ok(())
    }

    pub async fn get(&self) -> Result<Configuration, StorageError> {
        self.load_if_needed().await?;
        if let Some(config) = &*self.config.lock().await {
            Ok(config.clone())
        } else {
            Err(StorageError::Load)
        }
    }

    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn put(&self, config: &Configuration) -> Result<(), StorageError> {
        if matches!(config, Configuration::Provisioned(_)) {
            // only write it back if it's provisioned.
            // unprovisioned config is ephemeral.
            self.backing_store.borrow_mut().store(config).await?;
        }
        let mut locked_config = self.config.lock().await;
        locked_config.replace(config.clone());
        Ok(())
    }

    pub async fn modify<F: FnOnce(&mut ProvisionedConfiguration) -> Result<(), ()>>(
        &self,
        modification: F,
    ) -> Result<(), StorageError> {
        let config = self.config.lock().await;

        if let Some(Configuration::Provisioned(config)) = &*config {
            let mut config = config.clone();
            if modification(&mut config).is_ok() {
                self.put(&Configuration::Provisioned(config)).await?;
            }
        }

        Ok(())
    }

    #[allow(clippy::await_holding_refcell_ref)]
    async fn load_if_needed(&self) -> Result<(), StorageError> {
        let mut config = self.config.lock().await;
        if config.is_none() {
            let loaded_config = self.backing_store.borrow_mut().load().await?;
            config.replace(loaded_config);
        }

        Ok(())
    }

    pub async fn is_provisioned(&self) -> Result<bool, StorageError> {
        self.load_if_needed().await?;
        Ok(matches!(
            &*self.config.lock().await,
            Some(Configuration::Provisioned(..))
        ))
    }

    pub async fn is_unprovisioned(&self) -> Result<bool, StorageError> {
        self.load_if_needed().await?;
        Ok(matches!(
            &*self.config.lock().await,
            Some(Configuration::Unprovisioned(..))
        ))
    }

    pub fn capabilities(&self) -> Capabilities {
        unwrap!(self.capabilities.borrow().clone())
    }

    pub(crate) fn set_capabilities(&self, capabilities: Capabilities) {
        self.capabilities.borrow_mut().replace(capabilities);
    }

    pub fn composition(&self) -> Composition {
        unwrap!(self.composition.borrow().clone())
    }

    pub(crate) fn set_composition(&self, composition: Composition) {
        self.composition.borrow_mut().replace(composition);
    }
}
