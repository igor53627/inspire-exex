//! Bucket Index for sparse client-side PIR index lookups
//!
//! Uses 256K buckets (18-bit hash prefix) for O(1) lookup of (address, slot) -> bucket range.
//! Client downloads ~150 KB compressed index once, then computes bucket ranges locally.
//!
//! ## Limitations
//!
//! The bucket index returns a **range** (start_index, count), not an exact PIR index.
//! To locate a specific entry within a bucket, additional structure is needed (e.g.,
//! within-bucket manifest or secondary hash).
//!
//! ## DB Ordering Invariant
//!
//! **Critical**: The cumulative-sum scheme assumes the PIR database is physically ordered
//! by bucket ID:
//!
//! ```text
//! [bucket 0 entries][bucket 1 entries]...[bucket N entries]
//! ```
//!
//! If entries are not laid out in bucket-ID order, the `start_index` returned by
//! `lookup_bucket()` will be incorrect. The `bucket-index` builder tool must ensure
//! this ordering when generating the database.

use std::io::Read;

// Re-export shared types from inspire-core
pub use inspire_core::bucket_index::{
    compute_bucket_id, compute_cumulative, BucketDelta, BucketRange, NUM_BUCKETS,
};

/// Bucket index for sparse PIR lookups
///
/// Maps keccak256(address || slot) to a bucket, enabling O(1) bucket range lookup.
/// Returns (start_index, count) for the bucket; exact within-bucket index requires
/// additional structure (not yet implemented).
#[derive(Debug, Clone)]
pub struct BucketIndex {
    /// Count of entries in each bucket
    counts: Vec<u16>,
    /// Cumulative sum for O(1) start index lookup
    cumulative: Vec<u64>,
}

impl BucketIndex {
    /// Load bucket index from uncompressed binary (512 KB)
    pub fn from_bytes(data: &[u8]) -> Result<Self, BucketIndexError> {
        if data.len() != NUM_BUCKETS * 2 {
            return Err(BucketIndexError::InvalidSize {
                expected: NUM_BUCKETS * 2,
                actual: data.len(),
            });
        }

        let mut counts = Vec::with_capacity(NUM_BUCKETS);
        for chunk in data.chunks_exact(2) {
            counts.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }

        let cumulative = compute_cumulative(&counts);

        Ok(Self { counts, cumulative })
    }

    /// Load bucket index from zstd-compressed binary (~150 KB)
    pub fn from_compressed(data: &[u8]) -> Result<Self, BucketIndexError> {
        let decoder = zstd::Decoder::new(data)
            .map_err(|e| BucketIndexError::Decompression(e.to_string()))?;
        const MAX_SIZE: u64 = (NUM_BUCKETS * 2 + 1) as u64;
        let mut limited = decoder.take(MAX_SIZE);
        let mut decompressed = Vec::with_capacity(NUM_BUCKETS * 2);
        limited.read_to_end(&mut decompressed)?;
        if decompressed.len() > NUM_BUCKETS * 2 {
            return Err(BucketIndexError::DecompressionBomb {
                size: decompressed.len(),
            });
        }
        Self::from_bytes(&decompressed)
    }

    /// Look up the bucket range for a (address, slot) pair
    ///
    /// Returns (start_index, count) for the bucket containing this entry.
    /// **Note**: This returns a range, not an exact index. The client must either:
    /// - Query all entries in the range (privacy cost: multiple PIR queries)
    /// - Use additional within-bucket structure (not yet implemented)
    ///
    /// Assumes the PIR database is ordered by bucket ID (see module docs).
    pub fn lookup_bucket(&self, address: &[u8; 20], slot: &[u8; 32]) -> BucketRange {
        let bucket_id = compute_bucket_id(address, slot);
        let start = self.cumulative[bucket_id];
        let count = self.counts[bucket_id] as u64;

        BucketRange {
            bucket_id,
            start_index: start,
            count,
        }
    }

    /// Get total number of entries across all buckets
    pub fn total_entries(&self) -> u64 {
        self.cumulative[NUM_BUCKETS]
    }

    /// Get count for a specific bucket
    pub fn bucket_count(&self, bucket_id: usize) -> u16 {
        self.counts.get(bucket_id).copied().unwrap_or(0)
    }

    /// Get start index for a specific bucket
    pub fn bucket_start(&self, bucket_id: usize) -> u64 {
        self.cumulative.get(bucket_id).copied().unwrap_or(0)
    }

    /// Apply a delta update (for websocket streaming)
    pub fn apply_delta(&mut self, delta: &BucketDelta) {
        for &(bucket_id, new_count) in &delta.updates {
            if bucket_id < NUM_BUCKETS {
                self.counts[bucket_id] = new_count;
            }
        }
        // Recompute cumulative sums
        self.cumulative = compute_cumulative(&self.counts);
    }

    /// Serialize to bytes (uncompressed)
    pub fn to_bytes(&self) -> Vec<u8> {
        self.counts
            .iter()
            .flat_map(|&c| c.to_le_bytes())
            .collect()
    }

    /// Serialize to compressed bytes
    pub fn to_compressed(&self) -> Result<Vec<u8>, BucketIndexError> {
        let data = self.to_bytes();
        Ok(zstd::encode_all(&data[..], 19)?)
    }
}

/// Errors for bucket index operations
#[derive(Debug, thiserror::Error)]
pub enum BucketIndexError {
    #[error("Invalid bucket index size: expected {expected}, got {actual}")]
    InvalidSize { expected: usize, actual: usize },

    #[error("Invalid delta: {0}")]
    InvalidDelta(#[from] inspire_core::bucket_index::BucketDeltaError),

    #[error("Decompression bomb detected: payload exceeds maximum size ({size} bytes)")]
    DecompressionBomb { size: usize },

    #[error("Decompression error: {0}")]
    Decompression(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_id_deterministic() {
        let address = [0x42u8; 20];
        let slot = [0x01u8; 32];

        let id1 = compute_bucket_id(&address, &slot);
        let id2 = compute_bucket_id(&address, &slot);
        assert_eq!(id1, id2);
        assert!(id1 < NUM_BUCKETS);
    }

    #[test]
    fn test_bucket_index_from_bytes() {
        // Create test data: 256K buckets with varying counts
        let mut data = vec![0u8; NUM_BUCKETS * 2];
        // Set bucket 0 to count 10
        data[0] = 10;
        data[1] = 0;
        // Set bucket 1 to count 5
        data[2] = 5;
        data[3] = 0;

        let index = BucketIndex::from_bytes(&data).unwrap();

        assert_eq!(index.bucket_count(0), 10);
        assert_eq!(index.bucket_count(1), 5);
        assert_eq!(index.bucket_start(0), 0);
        assert_eq!(index.bucket_start(1), 10);
        assert_eq!(index.bucket_start(2), 15);
    }

    #[test]
    fn test_bucket_lookup() {
        let mut data = vec![0u8; NUM_BUCKETS * 2];
        // Set some buckets
        data[0] = 100;
        data[2] = 50;

        let index = BucketIndex::from_bytes(&data).unwrap();

        // Find an address/slot that hashes to bucket 0
        let address = [0u8; 20];
        let slot = [0u8; 32];
        let range = index.lookup_bucket(&address, &slot);

        // Just verify it returns a valid range
        assert!(range.bucket_id < NUM_BUCKETS);
    }

    #[test]
    fn test_bucket_delta() {
        let delta = BucketDelta {
            block_number: 12345,
            updates: vec![(0, 15), (100, 20)],
        };

        let bytes = delta.to_bytes();
        let recovered = BucketDelta::from_bytes(&bytes).unwrap();

        assert_eq!(recovered.block_number, 12345);
        assert_eq!(recovered.updates.len(), 2);
        assert_eq!(recovered.updates[0], (0, 15));
        assert_eq!(recovered.updates[1], (100, 20));
    }

    #[test]
    fn test_apply_delta() {
        let mut data = vec![0u8; NUM_BUCKETS * 2];
        data[0] = 10; // bucket 0 = 10

        let mut index = BucketIndex::from_bytes(&data).unwrap();
        assert_eq!(index.bucket_count(0), 10);

        let delta = BucketDelta {
            block_number: 1,
            updates: vec![(0, 15)],
        };

        index.apply_delta(&delta);
        assert_eq!(index.bucket_count(0), 15);
    }

    #[test]
    fn test_compression_roundtrip() {
        let mut data = vec![0u8; NUM_BUCKETS * 2];
        for i in 0..NUM_BUCKETS {
            let count = (i % 100) as u16;
            data[i * 2] = count as u8;
            data[i * 2 + 1] = (count >> 8) as u8;
        }

        let index = BucketIndex::from_bytes(&data).unwrap();
        let compressed = index.to_compressed().unwrap();
        let recovered = BucketIndex::from_compressed(&compressed).unwrap();

        assert_eq!(index.total_entries(), recovered.total_entries());
        for i in 0..100 {
            assert_eq!(index.bucket_count(i), recovered.bucket_count(i));
        }
    }

    #[test]
    fn test_delta_huge_update_count_does_not_oom() {
        let mut data = vec![0u8; 12];
        data[0..8].copy_from_slice(&1u64.to_le_bytes()); // block_number
        data[8..12].copy_from_slice(&u32::MAX.to_le_bytes()); // claims 4B updates

        let result = BucketDelta::from_bytes(&data);
        assert!(result.is_err(), "Should reject delta with huge update_count");
    }

    #[test]
    fn test_delta_truncated_updates() {
        let delta = BucketDelta {
            block_number: 1,
            updates: vec![(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)],
        };
        let mut bytes = delta.to_bytes();
        bytes[8..12].copy_from_slice(&10u32.to_le_bytes()); // lie: claim 10 updates

        let result = BucketDelta::from_bytes(&bytes);
        assert!(result.is_err(), "Should reject delta with truncated updates");
    }

    #[test]
    fn test_from_compressed_rejects_oversized() {
        let oversized = vec![0u8; NUM_BUCKETS * 2 + 1000];
        let bomb = zstd::encode_all(&oversized[..], 1).unwrap();

        let result = BucketIndex::from_compressed(&bomb);
        assert!(result.is_err(), "Should reject decompression bomb");
        assert!(matches!(result, Err(BucketIndexError::DecompressionBomb { .. })));
    }

    #[test]
    fn test_bucket_lookup_correctness() {
        let mut data = vec![0u8; NUM_BUCKETS * 2];
        for i in 0..NUM_BUCKETS {
            data[i * 2] = 1;
            data[i * 2 + 1] = 0;
        }

        let index = BucketIndex::from_bytes(&data).unwrap();

        let address = [0x42u8; 20];
        let slot = [0x01u8; 32];
        let bucket_id = compute_bucket_id(&address, &slot);
        let range = index.lookup_bucket(&address, &slot);

        assert_eq!(range.bucket_id, bucket_id);
        assert_eq!(range.start_index, bucket_id as u64);
        assert_eq!(range.count, 1);
    }
}
