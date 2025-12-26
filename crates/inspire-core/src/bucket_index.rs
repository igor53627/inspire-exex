//! Shared bucket index logic for native and WASM clients
//!
//! This module contains the core bucket index algorithms that are shared between
//! the native `inspire-client` and WASM `inspire-client-wasm` crates.
//!
//! ## DB Ordering Invariant
//!
//! The cumulative-sum scheme assumes the PIR database is physically ordered by bucket ID:
//! ```text
//! [bucket 0 entries][bucket 1 entries]...[bucket N entries]
//! ```

use tiny_keccak::{Hasher, Keccak};

/// Number of buckets (2^18 = 256K)
pub const NUM_BUCKETS: usize = 262_144;

/// Compute bucket ID from address and slot using keccak256
///
/// Takes first 18 bits of keccak256(address || slot) as bucket ID.
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

/// Compute cumulative sums for O(1) start index lookup
pub fn compute_cumulative(counts: &[u16]) -> Vec<u64> {
    let mut cumulative = Vec::with_capacity(NUM_BUCKETS + 1);
    cumulative.push(0);

    let mut sum = 0u64;
    for &count in counts {
        sum += count as u64;
        cumulative.push(sum);
    }

    cumulative
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

/// Error type for bucket delta parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BucketDeltaError {
    /// Delta header is too short (need at least 12 bytes)
    HeaderTooShort { actual: usize },
    /// Delta claims more updates than payload contains
    Truncated { expected: usize, actual: usize },
    /// Delta claims an excessive number of updates (potential DoS)
    TooManyUpdates { count: usize },
}

impl core::fmt::Display for BucketDeltaError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BucketDeltaError::HeaderTooShort { actual } => {
                write!(f, "Invalid delta: header too short (need 12 bytes, got {})", actual)
            }
            BucketDeltaError::Truncated { expected, actual } => {
                write!(f, "Invalid delta: truncated (expected {} bytes, got {})", expected, actual)
            }
            BucketDeltaError::TooManyUpdates { count } => {
                write!(f, "Invalid delta: too many updates ({}, max {})", count, NUM_BUCKETS)
            }
        }
    }
}

impl std::error::Error for BucketDeltaError {}

impl BucketDelta {
    /// Create from bytes (simple format: block_num:8 + count:4 + (bucket_id:4 + count:2)*)
    pub fn from_bytes(data: &[u8]) -> Result<Self, BucketDeltaError> {
        const HEADER_LEN: usize = 12;
        const UPDATE_SIZE: usize = 6;

        if data.len() < HEADER_LEN {
            return Err(BucketDeltaError::HeaderTooShort { actual: data.len() });
        }

        let block_number = u64::from_le_bytes(data[0..8].try_into().unwrap());
        let update_count = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;

        // Reject excessive update counts to prevent OOM on 32-bit targets (including WASM)
        if update_count > NUM_BUCKETS {
            return Err(BucketDeltaError::TooManyUpdates { count: update_count });
        }

        // Use checked arithmetic to prevent overflow on 32-bit targets
        let payload_len = update_count
            .checked_mul(UPDATE_SIZE)
            .and_then(|p| HEADER_LEN.checked_add(p))
            .ok_or(BucketDeltaError::TooManyUpdates { count: update_count })?;

        if data.len() < payload_len {
            return Err(BucketDeltaError::Truncated {
                expected: payload_len,
                actual: data.len(),
            });
        }

        let mut updates = Vec::with_capacity(update_count);
        let mut offset = HEADER_LEN;

        for _ in 0..update_count {
            let bucket_id = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            let new_count = u16::from_le_bytes(data[offset + 4..offset + 6].try_into().unwrap());
            updates.push((bucket_id, new_count));
            offset += UPDATE_SIZE;
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
    fn test_compute_cumulative() {
        let counts = vec![10u16, 5, 3, 0, 2];
        let cumulative = compute_cumulative(&counts);

        assert_eq!(cumulative[0], 0);
        assert_eq!(cumulative[1], 10);
        assert_eq!(cumulative[2], 15);
        assert_eq!(cumulative[3], 18);
        assert_eq!(cumulative[4], 18);
        assert_eq!(cumulative[5], 20);
    }

    #[test]
    fn test_bucket_delta_roundtrip() {
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
    fn test_delta_huge_update_count_rejected() {
        let mut data = vec![0u8; 12];
        data[0..8].copy_from_slice(&1u64.to_le_bytes());
        data[8..12].copy_from_slice(&u32::MAX.to_le_bytes());

        let result = BucketDelta::from_bytes(&data);
        assert!(matches!(result, Err(BucketDeltaError::TooManyUpdates { .. })));
    }

    #[test]
    fn test_delta_exceeds_num_buckets_rejected() {
        let mut data = vec![0u8; 12];
        data[0..8].copy_from_slice(&1u64.to_le_bytes());
        data[8..12].copy_from_slice(&((NUM_BUCKETS + 1) as u32).to_le_bytes());

        let result = BucketDelta::from_bytes(&data);
        assert!(matches!(result, Err(BucketDeltaError::TooManyUpdates { count }) if count == NUM_BUCKETS + 1));
    }

    #[test]
    fn test_delta_truncated_rejected() {
        let delta = BucketDelta {
            block_number: 1,
            updates: vec![(0, 1), (1, 2), (2, 3)],
        };
        let mut bytes = delta.to_bytes();
        bytes[8..12].copy_from_slice(&10u32.to_le_bytes()); // lie about count

        let result = BucketDelta::from_bytes(&bytes);
        assert!(matches!(result, Err(BucketDeltaError::Truncated { .. })));
    }

    #[test]
    fn test_delta_header_too_short() {
        let data = vec![0u8; 8]; // only 8 bytes, need 12

        let result = BucketDelta::from_bytes(&data);
        assert!(matches!(result, Err(BucketDeltaError::HeaderTooShort { actual: 8 })));
    }
}
