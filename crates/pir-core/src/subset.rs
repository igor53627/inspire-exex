//! Subset management and query construction

use crate::prf::{Seed, expand_seed};
use serde::{Deserialize, Serialize};

/// A subset defined by its PRF seed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subset {
    /// PRF seed (32 bytes)
    pub seed: Seed,
    /// Number of indices in the subset
    pub size: usize,
    /// Domain size (total database entries)
    pub domain_size: u64,
}

impl Subset {
    /// Create a new subset from a seed
    pub fn new(seed: Seed, size: usize, domain_size: u64) -> Self {
        Self { seed, size, domain_size }
    }

    /// Generate a random subset
    pub fn random(size: usize, domain_size: u64) -> Self {
        let mut seed = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut seed);
        Self::new(seed, size, domain_size)
    }

    /// Expand the seed into actual indices
    pub fn expand(&self) -> Vec<u64> {
        expand_seed(&self.seed, self.size, self.domain_size)
    }

    /// Check if a target index is in this subset
    pub fn contains(&self, target: u64) -> bool {
        self.expand().contains(&target)
    }

    /// Serialized size for network transmission
    pub fn serialized_size() -> usize {
        32 + 8 + 8  // seed + size + domain_size
    }
}

/// Compressed query (just the seed + params)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedQuery {
    pub seed: Seed,
    pub subset_size: u64,
    pub domain_size: u64,
}

impl CompressedQuery {
    pub fn new(subset: &Subset) -> Self {
        Self {
            seed: subset.seed,
            subset_size: subset.size as u64,
            domain_size: subset.domain_size,
        }
    }

    /// Expand to full subset on server side
    pub fn expand(&self) -> Vec<u64> {
        expand_seed(&self.seed, self.subset_size as usize, self.domain_size)
    }

    /// Size in bytes (~48 bytes vs 150KB uncompressed)
    pub fn size_bytes() -> usize {
        32 + 8 + 8
    }
}

/// Find a pre-generated subset that contains the target index
pub fn find_subset_for_target(subsets: &[Subset], target: u64) -> Option<usize> {
    for (i, subset) in subsets.iter().enumerate() {
        if subset.contains(target) {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subset_expansion_deterministic() {
        let subset = Subset::new([42u8; 32], 100, 1_000_000);
        let indices1 = subset.expand();
        let indices2 = subset.expand();
        assert_eq!(indices1, indices2);
    }

    #[test]
    fn test_compressed_query_size() {
        // Should be ~48 bytes, not 150KB
        assert!(CompressedQuery::size_bytes() < 100);
    }

    #[test]
    fn test_find_subset_for_target() {
        let subsets: Vec<Subset> = (0..10)
            .map(|i| {
                let mut seed = [0u8; 32];
                seed[0] = i;
                Subset::new(seed, 1000, 1_000_000)
            })
            .collect();

        // Pick a target from the first subset
        let target = subsets[0].expand()[0];
        let found = find_subset_for_target(&subsets, target);
        
        assert!(found.is_some());
        assert_eq!(found.unwrap(), 0);
    }
}
