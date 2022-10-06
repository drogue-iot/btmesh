use crate::Signal;
use core::ops::Deref;
use core::sync::atomic::{AtomicU8, Ordering};

pub struct AccessCounted<T> {
    count: AtomicU8,
    signal: Signal<()>,
    value: Option<T>,
}

impl<T> AccessCounted<T> {
    pub const fn new() -> Self {
        Self {
            count: AtomicU8::new(0),
            signal: Signal::new(),
            value: None,
        }
    }

    pub fn set(&mut self, value: T) {
        self.value.replace(value);
    }

    pub fn get(&self) -> AccessCountedHandle<'_, T> {
        self.count.fetch_add(1, Ordering::Relaxed);
        AccessCountedHandle { barrier: self }
    }

    fn unget(&self) {
        if self.count.fetch_sub(1, Ordering::Relaxed) == 1 {
            self.signal.signal(())
        }
    }

    pub async fn wait(&self) {
        self.signal.wait().await
    }
}

pub struct AccessCountedHandle<'b, T> {
    barrier: &'b AccessCounted<T>,
}

impl<T> Deref for AccessCountedHandle<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.barrier.value.as_ref().unwrap()
    }
}

impl<T> Drop for AccessCountedHandle<'_, T> {
    fn drop(&mut self) {
        self.barrier.unget()
    }
}

impl<T> Clone for AccessCountedHandle<'_, T> {
    fn clone(&self) -> Self {
        self.barrier.get()
    }
}
