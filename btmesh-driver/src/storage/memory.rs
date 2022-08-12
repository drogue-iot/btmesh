use crate::storage::{BackingStore, Configuration, StorageError};
use core::future::{ready, Future};

pub struct MemoryBackingStore {
    content: Configuration,
}

impl BackingStore for MemoryBackingStore {
    type LoadFuture<'m> =  impl Future<Output = Result<Configuration, StorageError>> + 'm
        where
            Self: 'm;
    type StoreFuture<'m> = impl Future<Output = Result<(), StorageError>> + 'm
        where
            Self: 'm;
    type ClearFuture<'m> = impl Future<Output = Result<(), StorageError>> + 'm
        where
            Self: 'm;

    fn load(&mut self) -> Self::LoadFuture<'_> {
        ready(Ok(self.content.clone()))
    }

    fn store(&mut self, content: &Configuration) -> Self::StoreFuture<'_> {
        self.content = content.clone();
        ready(Ok(()))
    }

    fn clear(&mut self) -> Self::ClearFuture<'_> {
        // TODO: should self.content be an Option<Configuration>?
        ready(Ok(()))
    }
}
