use btmesh_common::Seq;
use core::sync::atomic::{AtomicU32, Ordering};

pub struct Sequence {
    seq: AtomicU32,
}

impl Sequence {
    pub fn new(initial_seq: Seq) -> Self {
        Self {
            seq: AtomicU32::new(initial_seq.value()),
        }
    }

    pub fn next(&self) -> Seq {
        Seq::new(self.seq.fetch_add(1, Ordering::Relaxed))
    }

    pub fn current(&self) -> u32 {
        self.seq.load(Ordering::Relaxed)
    }
}
