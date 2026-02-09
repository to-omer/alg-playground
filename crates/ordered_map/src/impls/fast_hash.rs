use std::hash::{BuildHasherDefault, Hasher};

/// A very small, fast, non-cryptographic hasher for integer keys.
///
/// This is intended for benchmarking/algorithm experiments where `SipHash`
/// (std's default) dominates runtime for hash-heavy structures (x-fast, vEB).
#[derive(Default)]
pub(crate) struct FastHasher {
    state: u64,
}

pub(crate) type FastHashMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<FastHasher>>;

impl FastHasher {
    #[inline]
    fn mix(mut z: u64) -> u64 {
        // splitmix64 finalizer (public domain).
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    #[inline]
    fn write_word(&mut self, x: u64) {
        self.state = Self::mix(self.state.wrapping_add(x));
    }
}

impl Hasher for FastHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.state
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut i = 0;
        while i + 8 <= bytes.len() {
            let mut w = [0_u8; 8];
            w.copy_from_slice(&bytes[i..i + 8]);
            self.write_word(u64::from_le_bytes(w));
            i += 8;
        }
        if i < bytes.len() {
            let mut w = [0_u8; 8];
            w[..(bytes.len() - i)].copy_from_slice(&bytes[i..]);
            self.write_word(u64::from_le_bytes(w));
        }
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.write_word(i);
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.write_word(i as u64);
    }
}
