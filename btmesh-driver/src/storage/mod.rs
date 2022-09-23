pub use crate::storage::provisioned::ProvisionedConfiguration;
pub use crate::storage::unprovisioned::UnprovisionedConfiguration;
use crate::DriverError;
use btmesh_common::Composition;
use btmesh_pdu::provisioning::Capabilities;
use core::cell::Ref;
use core::cell::RefCell;
use core::future::Future;
use core::hash::Hash;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::{Mutex, MutexGuard};
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
    type LoadFuture<'m>: Future<Output = Result<ProvisionedConfiguration, StorageError>> + 'm
    where
        Self: 'm;

    type StoreFuture<'m>: Future<Output = Result<(), StorageError>> + 'm
    where
        Self: 'm;

    // TODO: rustc didn't like me returning a StoreFuture from both clear and store... wtf?
    type ClearFuture<'m>: Future<Output = Result<(), StorageError>> + 'm
    where
        Self: 'm;

    fn load(&mut self) -> Self::LoadFuture<'_>;
    fn store<'f>(&'f mut self, config: &'f ProvisionedConfiguration) -> Self::StoreFuture<'f>;
    fn clear(&mut self) -> Self::ClearFuture<'_>;
}

#[allow(clippy::large_enum_variant)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[derive(Hash, Debug)]
pub enum Configuration {
    Unprovisioned(UnprovisionedConfiguration),
    Provisioned(ProvisionedConfiguration),
}

impl Configuration {
    pub fn display(&self, composition: &Composition) {
        match self {
            Configuration::Unprovisioned(inner) => inner.display(composition),
            Configuration::Provisioned(inner) => inner.display(composition),
        }
    }
}

pub struct Storage<B: BackingStore> {
    backing_store: RefCell<B>,
    capabilities: RefCell<Option<Capabilities>>,
    composition: RefCell<Option<Composition>>,
    config: Mutex<CriticalSectionRawMutex, Option<Configuration>>,
    default_config: UnprovisionedConfiguration,
}

impl<B: BackingStore> Storage<B> {
    pub fn new(backing_store: B, default_config: UnprovisionedConfiguration) -> Self {
        Self {
            backing_store: RefCell::new(backing_store),
            capabilities: RefCell::new(None),
            composition: RefCell::new(None),
            config: Mutex::new(None),
            default_config,
        }
    }

    pub async fn init(&self) -> Result<(), StorageError> {
        let mut locked_config = self.config.lock().await;
        let mut backing_store = self.backing_store.borrow_mut();
        if let Ok(mut config) = backing_store.load().await {
            let seq = config.sequence();

            let mut extra = seq % 100;
            if extra == 100 {
                extra = 0;
            }
            let seq = (seq - extra) + 100;

            *config.sequence_mut() = seq;
            backing_store.store(&config).await?;
            locked_config.replace(Configuration::Provisioned(config));
        } else {
            locked_config.replace(Configuration::Unprovisioned(self.default_config.clone()));
        }
        Ok(())
    }

    pub async fn provision(&self, config: ProvisionedConfiguration) -> Result<(), DriverError> {
        let mut locked_config = self.config.lock().await;
        self.backing_store.borrow_mut().store(&config).await?;
        locked_config.replace(Configuration::Provisioned(config));
        Ok(())
    }

    pub async fn modify_provisioned<
        F: FnOnce(&mut ProvisionedConfiguration) -> Result<(), DriverError>,
    >(
        &self,
        modifier: F,
    ) -> Result<(), DriverError> {
        let mut config = self.config.lock().await;
        if let Some(Configuration::Provisioned(config)) = &mut *config {
            modifier(config)?;
            self.backing_store.borrow_mut().store(config).await?;
        }

        Ok(())
    }

    pub async fn lock(&self) -> MutexGuard<'_, CriticalSectionRawMutex, Option<Configuration>> {
        self.config.lock().await
    }

    pub async fn read<F: FnOnce(&Configuration) -> Result<R, DriverError>, R>(
        &self,
        reader: F,
    ) -> Result<R, DriverError> {
        let config = self.config.lock().await;
        if let Some(config) = &*config {
            reader(config)
        } else {
            Err(DriverError::InvalidState)
        }
    }

    pub async fn read_provisioned<
        F: FnOnce(&ProvisionedConfiguration) -> Result<R, DriverError>,
        R,
    >(
        &self,
        reader: F,
    ) -> Result<R, DriverError> {
        let config = self.config.lock().await;
        if let Some(Configuration::Provisioned(config)) = &*config {
            return reader(config);
        }
        Err(DriverError::InvalidState)
    }

    pub async fn is_provisioned(&self) -> Result<bool, StorageError> {
        Ok(matches!(
            &*self.config.lock().await,
            Some(Configuration::Provisioned(..))
        ))
    }

    pub async fn is_unprovisioned(&self) -> Result<bool, StorageError> {
        Ok(matches!(
            &*self.config.lock().await,
            Some(Configuration::Unprovisioned(..))
        ))
    }

    pub async fn reset(&self) -> Result<(), StorageError> {
        let mut locked_config = self.config.lock().await;
        self.backing_store.borrow_mut().clear().await?;
        locked_config.replace(Configuration::Unprovisioned(self.default_config.clone()));
        Ok(())
    }

    pub fn capabilities(&self) -> Capabilities {
        unwrap!(self.capabilities.borrow().clone())
    }

    pub(crate) fn set_capabilities(&self, capabilities: Capabilities) {
        self.capabilities.borrow_mut().replace(capabilities);
    }

    pub fn composition(&self) -> Ref<'_, Option<Composition>> {
        self.composition.borrow()
    }

    pub(crate) fn set_composition(&self, composition: Composition) {
        self.composition.borrow_mut().replace(composition);
    }
}
