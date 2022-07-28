use core::hash::{Hash, Hasher};

const BASIS: u64 = 0xcbf29ce484222325;
const PRIME: u64 = 0x100000001b3;

/// 64-bit Fowler-Noll-Vo hasher
pub struct FnvHasher {
    state: u64,
}

impl Default for FnvHasher {
    fn default() -> Self {
        Self { state: BASIS }
    }
}

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state ^= u64::from(*byte);
            self.state = self.state.wrapping_mul(PRIME);
        }
    }
}

pub fn hash_of<T: Hash>(obj: &T) -> u64 {
    let mut hasher = FnvHasher::default();
    obj.hash(&mut hasher);
    hasher.finish()
}
