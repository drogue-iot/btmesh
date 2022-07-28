use crate::storage::{BackingStore, Configuration, StorageError};
use core::future::Future;
use embedded_storage_async::nor_flash::AsyncNorFlash;
use postcard::{from_bytes, to_slice};

pub struct FlashBackingStore<F: AsyncNorFlash> {
    flash: F,
    base_address: u32,
}

impl<F: AsyncNorFlash> FlashBackingStore<F> {
    pub fn new(flash: F, base_address: u32) -> Self {
        Self {
            flash,
            base_address,
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
            let content = from_bytes(&bytes).map_err(|_| StorageError::Serialization)?;
            Ok(content)
        }
    }

    fn store<'f>(&'f mut self, content: &'f Configuration) -> Self::StoreFuture<'f> {
        async move {
            let mut bytes = [0; 4096];
            to_slice(content, &mut bytes).map_err(|_| StorageError::Serialization)?;
            self.flash
                .write(self.base_address, &bytes)
                .await
                .map_err(|_| StorageError::Store)?;
            Ok(())
        }
    }
}
