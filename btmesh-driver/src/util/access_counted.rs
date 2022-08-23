use core::future::Future;
use core::ops::Deref;
use core::pin::Pin;
use core::sync::atomic::{AtomicU8, Ordering};
use core::task::{Context, Poll};
use embassy_util::channel::signal::Signal;

pub struct AccessCounted<T> {
    count: AtomicU8,
    signal: Signal<()>,
    value: T,
}

impl<T> AccessCounted<T> {
    pub fn new(value: T) -> Self {
        Self {
            count: Default::default(),
            signal: Signal::new(),
            value,
        }
    }

    pub fn get(&self) -> AccessCountedHandle<'_, T> {
        self.count.fetch_add(1, Ordering::Relaxed);
        AccessCountedHandle {
            barrier: self
        }
    }

    fn unget(&self) {
        if self.count.fetch_sub(1, Ordering::Relaxed) == 1 {
            self.signal.signal(())
        }
    }
}

impl<T> Future for AccessCounted<T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.signal.poll_wait(cx)
    }
}

pub struct AccessCountedHandle<'b, T> {
    barrier: &'b AccessCounted<T>
}

impl<T> Deref for AccessCountedHandle<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.barrier.value
    }
}

impl<T> Drop for AccessCountedHandle<'_, T> {
    fn drop(&mut self) {
        self.barrier.unget()
    }
}