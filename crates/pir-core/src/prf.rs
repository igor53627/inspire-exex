//! PRF (Pseudorandom Function) for subset generation
//!
//! Uses AES-128 in counter mode to expand a 32-byte seed
//! into a deterministic subset of indices.

use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes128;

/// PRF seed (32 bytes)
pub type Seed = [u8; 32];

/// PRF for generating pseudorandom subsets
pub struct Prf {
    cipher: Aes128,
    domain_size: u64,
}

impl Prf {
    /// Create a new PRF instance from a seed
    pub fn new(seed: &Seed, domain_size: u64) -> Self {
        // Use first 16 bytes of seed as AES key
        let key: [u8; 16] = seed[..16].try_into().unwrap();
        let cipher = Aes128::new(&key.into());
        Self { cipher, domain_size }
    }

    /// Generate a single pseudorandom index in [0, domain_size)
    pub fn generate_index(&self, counter: u64) -> u64 {
        let mut block = [0u8; 16];
        block[..8].copy_from_slice(&counter.to_le_bytes());
        
        let mut encrypted = block.into();
        self.cipher.encrypt_block(&mut encrypted);
        
        let value = u64::from_le_bytes(encrypted[..8].try_into().unwrap());
        value % self.domain_size
    }

    /// Generate a subset of `size` indices
    pub fn generate_subset(&self, size: usize) -> Vec<u64> {
        // Use rejection sampling to avoid duplicates
        let mut indices = std::collections::HashSet::with_capacity(size);
        let mut counter = 0u64;
        
        while indices.len() < size {
            let idx = self.generate_index(counter);
            indices.insert(idx);
            counter += 1;
        }
        
        let mut result: Vec<_> = indices.into_iter().collect();
        result.sort_unstable();
        result
    }
}

/// Expand a seed into a subset (convenience function)
pub fn expand_seed(seed: &Seed, subset_size: usize, domain_size: u64) -> Vec<u64> {
    let prf = Prf::new(seed, domain_size);
    prf.generate_subset(subset_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prf_deterministic() {
        let seed = [0u8; 32];
        let prf = Prf::new(&seed, 1_000_000);
        
        let subset1 = prf.generate_subset(100);
        let subset2 = prf.generate_subset(100);
        
        assert_eq!(subset1, subset2);
    }

    #[test]
    fn test_prf_different_seeds() {
        let seed1 = [0u8; 32];
        let mut seed2 = [0u8; 32];
        seed2[0] = 1;
        
        let prf1 = Prf::new(&seed1, 1_000_000);
        let prf2 = Prf::new(&seed2, 1_000_000);
        
        let subset1 = prf1.generate_subset(100);
        let subset2 = prf2.generate_subset(100);
        
        assert_ne!(subset1, subset2);
    }

    #[test]
    fn test_subset_size() {
        let seed = [42u8; 32];
        let prf = Prf::new(&seed, 1_000_000);
        let subset = prf.generate_subset(1000);
        
        assert_eq!(subset.len(), 1000);
    }
}
