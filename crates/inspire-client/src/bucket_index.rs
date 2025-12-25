//! Bucket Index for sparse client-side PIR index lookups
//!
//! Uses 256K buckets (18-bit hash prefix) for O(1) lookup of (address, slot) -> index range.
//! Client downloads ~150 KB compressed index once, then computes indices locally.

use std::io::Read;

use tiny_keccak::{Hasher, Keccak};

/// Number of buckets (2^18 = 256K)
pub const NUM_BUCKETS: usize = 262_144;

/// Bucket index for sparse PIR lookups
///
/// Maps keccak256(address || slot) to a bucket, enabling O(1) index computation.
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

        let cumulative = Self::compute_cumulative(&counts);

        Ok(Self { counts, cumulative })
    }

    /// Load bucket index from zstd-compressed binary (~150 KB)
    pub fn from_compressed(data: &[u8]) -> Result<Self, BucketIndexError> {
        let mut decoder = zstd::Decoder::new(data)?;
        let mut decompressed = Vec::with_capacity(NUM_BUCKETS * 2);
        decoder.read_to_end(&mut decompressed)?;
        Self::from_bytes(&decompressed)
    }

    /// Compute cumulative sums for O(1) start index lookup
    fn compute_cumulative(counts: &[u16]) -> Vec<u64> {
        let mut cumulative = Vec::with_capacity(NUM_BUCKETS + 1);
        cumulative.push(0);
        
        let mut sum = 0u64;
        for &count in counts {
            sum += count as u64;
            cumulative.push(sum);
        }
        
        cumulative
    }

    /// Look up the index range for a (address, slot) pair
    ///
    /// Returns (start_index, count) for the bucket containing this entry.
    /// The exact index within the bucket requires additional lookup.
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
        self.cumulative = Self::compute_cumulative(&self.counts);
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

/// Range of indices within a bucket
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BucketRange {
    /// Bucket ID (0 to NUM_BUCKETS-1)
    pub bucket_id: usize,
    /// Start index in the database
    pub start_index: u64,
    /// Number of entries in this bucket
    pub count: u64,
}

/// Delta update for streaming bucket index updates
#[derive(Debug, Clone)]
pub struct BucketDelta {
    /// Block number this delta applies to
    pub block_number: u64,
    /// Updated bucket counts: (bucket_id, new_count)
    pub updates: Vec<(usize, u16)>,
}

impl BucketDelta {
    /// Create from bytes (simple format: block_num:8 + count:4 + (bucket_id:4 + count:2)*)
    pub fn from_bytes(data: &[u8]) -> Result<Self, BucketIndexError> {
        if data.len() < 12 {
            return Err(BucketIndexError::InvalidDelta);
        }

        let block_number = u64::from_le_bytes(data[0..8].try_into().unwrap());
        let update_count = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;

        let mut updates = Vec::with_capacity(update_count);
        let mut offset = 12;

        for _ in 0..update_count {
            if offset + 6 > data.len() {
                return Err(BucketIndexError::InvalidDelta);
            }
            let bucket_id = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            let new_count = u16::from_le_bytes(data[offset + 4..offset + 6].try_into().unwrap());
            updates.push((bucket_id, new_count));
            offset += 6;
        }

        Ok(Self {
            block_number,
            updates,
        })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(12 + self.updates.len() * 6);
        data.extend_from_slice(&self.block_number.to_le_bytes());
        data.extend_from_slice(&(self.updates.len() as u32).to_le_bytes());
        for &(bucket_id, count) in &self.updates {
            data.extend_from_slice(&(bucket_id as u32).to_le_bytes());
            data.extend_from_slice(&count.to_le_bytes());
        }
        data
    }
}

/// Compute bucket ID from address and slot using keccak256
pub fn compute_bucket_id(address: &[u8; 20], slot: &[u8; 32]) -> usize {
    let mut hasher = Keccak::v256();
    hasher.update(address);
    hasher.update(slot);

    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);

    // Take first 18 bits as bucket ID
    let bucket_id = ((hash[0] as usize) << 10) | ((hash[1] as usize) << 2) | ((hash[2] as usize) >> 6);
    bucket_id & (NUM_BUCKETS - 1)
}

/// Errors for bucket index operations
#[derive(Debug, thiserror::Error)]
pub enum BucketIndexError {
    #[error("Invalid bucket index size: expected {expected}, got {actual}")]
    InvalidSize { expected: usize, actual: usize },

    #[error("Invalid delta format")]
    InvalidDelta,

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
}
