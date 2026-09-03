//! A deterministic, dependency-free hash used to derive identifiers that must
//! agree across separate builds (e.g. the server and the wasm client of an
//! islands or `#[lazy]` app).
//!
//! `std::hash::DefaultHasher` is explicitly documented as not stable across
//! Rust releases, and `Hash` integer writes are native-endian, so neither is
//! safe for cross-build/cross-target identifiers. FNV-1a is a fixed algorithm
//! with constant offset/prime, so the same bytes always produce the same
//! 64-bit value regardless of toolchain, platform, or target word size.

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// 64-bit FNV-1a hash of `bytes`, with a final avalanche step.
///
/// Plain FNV-1a has weak bit diffusion on short, low-entropy inputs (the
/// `line:col:file` and component-name strings hashed here are exactly that),
/// so the low bits can cluster for inputs that differ by a single byte. The
/// `fmix64` finalizer (MurmurHash3's mixing step) spreads every input bit
/// across all 64 output bits, so the realistic collision probability matches
/// the ideal-random birthday bound (~3e-14 for 1000 distinct identifiers).
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    fmix64(hash)
}

/// MurmurHash3 64-bit finalizer: a fixed, reversible avalanche mix.
fn fmix64(mut h: u64) -> u64 {
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    h ^= h >> 33;
    h
}

#[cfg(test)]
mod tests {
    use super::fnv1a_64;
    use std::collections::HashSet;

    #[test]
    fn deterministic() {
        // The whole point: identical bytes always hash to the same value, so
        // the server and client builds agree on the generated identifier.
        assert_eq!(fnv1a_64(b"my_island_42"), fnv1a_64(b"my_island_42"));
    }

    #[test]
    fn no_collisions_on_near_identical_inputs() {
        // The inputs we hash differ only by a few low-entropy bytes
        // (line/column numbers, sequential component names). After the
        // avalanche finalizer these must all map to distinct values.
        let mut seen = HashSet::new();
        for line in 0..1_000u32 {
            for col in 0..32u32 {
                let key = format!("{line}:{col}:src/app.rs");
                assert!(
                    seen.insert(fnv1a_64(key.as_bytes())),
                    "collision for {key}"
                );
            }
        }
    }

    #[test]
    fn single_byte_change_avalanches() {
        // A one-byte difference should flip roughly half the output bits;
        // require at least a quarter to catch any regression to weak mixing.
        let a = fnv1a_64(b"component_a");
        let b = fnv1a_64(b"component_b");
        assert!((a ^ b).count_ones() >= 16, "weak avalanche: {a:x} vs {b:x}");
    }
}
