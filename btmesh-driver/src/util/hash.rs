use core::hash::Hasher;

const BASIS: u64 = 0x811c9dc5;
const PRIME: u64 = 0x1000193;

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
