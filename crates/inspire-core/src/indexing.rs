//! Indexing: maps (contract, slot) pairs to database indices
//!
//! This module provides deterministic functions to convert storage slot keys
//! to indices within the PIR database. The same functions must be used at:
//! - Setup time: when building the encoded database
//! - Query time: when the client computes which index to query
//!
//! # Collision Warning
//!
//! This indexing scheme is **probabilistic** (hash-based). If more active slots
//! are mapped into a lane than the lane can support, collisions will occur.
//! Collisions cause two logical storage entries to share one physical slot,
//! which results in incorrect query results.
//!
//! To avoid collisions:
//! - Hot lane: ensure `slot_count` per contract exceeds actual storage slots used
//! - Cold lane: ensure `total_entries` is large enough for the key space
//!
//! DB builders should detect collisions during setup and fail loudly.

use crate::{Address, StorageKey};

/// Compute the offset within a contract's slot range.
///
/// Uses a deterministic 64-bit hash (SipHash-inspired mixing) to map the
/// 32-byte slot key to an offset in the range `[0, num_slots)`. This allows
/// sparse slot usage while maintaining uniform distribution.
///
/// # Arguments
/// - `slot`: The 32-byte storage slot key
/// - `num_slots`: Number of slots allocated to this contract in the manifest
///
/// # Returns
/// - `Some(offset)` in `[0, num_slots)` if `num_slots > 0`
/// - `None` if `num_slots == 0` (invalid configuration)
pub fn slot_to_offset(slot: &StorageKey, num_slots: u64) -> Option<u64> {
    if num_slots == 0 {
        return None;
    }

    let hash = hash_slot(slot);
    Some(hash % num_slots)
}

/// Compute the global hot lane index for a (contract, slot) pair.
///
/// # Arguments
/// - `start_index`: The starting index for this contract in the hot lane database
/// - `slot`: The 32-byte storage slot key
/// - `num_slots`: Number of slots allocated to this contract
///
/// # Returns
/// - `Some(index)` in `[start_index, start_index + num_slots)` if `num_slots > 0`
/// - `None` if `num_slots == 0` (invalid configuration)
pub fn hot_index(start_index: u64, slot: &StorageKey, num_slots: u64) -> Option<u64> {
    slot_to_offset(slot, num_slots).map(|offset| start_index + offset)
}

/// Compute the global cold lane index for a (contract, slot) pair.
///
/// The cold lane uses a single global hash to map any (contract, slot) pair
/// to an index in the cold lane database. This provides uniform distribution
/// across the entire cold lane space.
///
/// # Arguments
/// - `contract`: The 20-byte contract address
/// - `slot`: The 32-byte storage slot key
/// - `total_entries`: Total number of entries in the cold lane database
///
/// # Returns
/// - `Some(index)` in `[0, total_entries)` if `total_entries > 0`
/// - `None` if `total_entries == 0` (invalid configuration)
pub fn cold_index(contract: &Address, slot: &StorageKey, total_entries: u64) -> Option<u64> {
    if total_entries == 0 {
        return None;
    }

    let hash = hash_contract_slot(contract, slot);
    Some(hash % total_entries)
}

/// Hash a storage slot key to a u64 using SipHash-like mixing.
///
/// Uses a simple but deterministic hash function that works with no_std.
/// We mix all 32 bytes through a series of multiply-rotate-xor operations.
fn hash_slot(slot: &StorageKey) -> u64 {
    let mut h: u64 = 0x517cc1b727220a95; // Seed constant

    for chunk in slot.chunks(8) {
        let mut bytes = [0u8; 8];
        bytes[..chunk.len()].copy_from_slice(chunk);
        let val = u64::from_le_bytes(bytes);

        h = h.wrapping_add(val);
        h = h.rotate_left(13);
        h ^= h >> 7;
        h = h.wrapping_mul(0x9e3779b97f4a7c15);
    }

    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;

    h
}

/// Hash a (contract, slot) pair to a u64.
///
/// Combines the contract address and slot into a single hash value.
fn hash_contract_slot(contract: &Address, slot: &StorageKey) -> u64 {
    let mut h: u64 = 0x9e3779b97f4a7c15; // Different seed for cold lane

    // Mix in contract address (20 bytes = 2.5 chunks of 8)
    for chunk in contract.chunks(8) {
        let mut bytes = [0u8; 8];
        bytes[..chunk.len()].copy_from_slice(chunk);
        let val = u64::from_le_bytes(bytes);

        h = h.wrapping_add(val);
        h = h.rotate_left(17);
        h ^= h >> 11;
        h = h.wrapping_mul(0x517cc1b727220a95);
    }

    // Mix in slot (32 bytes = 4 chunks of 8)
    for chunk in slot.chunks(8) {
        let mut bytes = [0u8; 8];
        bytes[..chunk.len()].copy_from_slice(chunk);
        let val = u64::from_le_bytes(bytes);

        h = h.wrapping_add(val);
        h = h.rotate_left(13);
        h ^= h >> 7;
        h = h.wrapping_mul(0x9e3779b97f4a7c15);
    }

    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;

    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_to_offset_deterministic() {
        let slot = [0x42u8; 32];
        let offset1 = slot_to_offset(&slot, 1000);
        let offset2 = slot_to_offset(&slot, 1000);
        assert_eq!(offset1, offset2, "Must be deterministic");
    }

    #[test]
    fn test_slot_to_offset_bounded() {
        let slot = [0xffu8; 32];
        for num_slots in [1, 100, 1000, 1_000_000] {
            let offset = slot_to_offset(&slot, num_slots).unwrap();
            assert!(
                offset < num_slots,
                "offset {} should be < {}",
                offset,
                num_slots
            );
        }
    }

    #[test]
    fn test_slot_to_offset_zero_returns_none() {
        let slot = [0u8; 32];
        assert_eq!(slot_to_offset(&slot, 0), None);
    }

    #[test]
    fn test_different_slots_different_offsets() {
        let slot1 = [0x01u8; 32];
        let slot2 = [0x02u8; 32];
        let num_slots = 1_000_000;

        let offset1 = slot_to_offset(&slot1, num_slots).unwrap();
        let offset2 = slot_to_offset(&slot2, num_slots).unwrap();

        assert_ne!(
            offset1, offset2,
            "Different slots should likely have different offsets"
        );
    }

    #[test]
    fn test_hot_index() {
        let slot = [0x42u8; 32];
        let start_index = 5000;
        let num_slots = 1000;

        let idx = hot_index(start_index, &slot, num_slots).unwrap();
        let offset = slot_to_offset(&slot, num_slots).unwrap();

        assert_eq!(idx, start_index + offset);
        assert!(idx >= start_index);
        assert!(idx < start_index + num_slots);
    }

    #[test]
    fn test_hot_index_zero_slots_returns_none() {
        let slot = [0x42u8; 32];
        assert_eq!(hot_index(5000, &slot, 0), None);
    }

    #[test]
    fn test_cold_index_deterministic() {
        let contract = [0x11u8; 20];
        let slot = [0x22u8; 32];
        let total = 2_700_000_000u64;

        let idx1 = cold_index(&contract, &slot, total);
        let idx2 = cold_index(&contract, &slot, total);

        assert_eq!(idx1, idx2, "Must be deterministic");
    }

    #[test]
    fn test_cold_index_bounded() {
        let contract = [0xffu8; 20];
        let slot = [0xffu8; 32];

        for total in [1, 1000, 1_000_000, 2_700_000_000] {
            let idx = cold_index(&contract, &slot, total).unwrap();
            assert!(idx < total, "index {} should be < {}", idx, total);
        }
    }

    #[test]
    fn test_cold_index_zero_returns_none() {
        let contract = [0x11u8; 20];
        let slot = [0x22u8; 32];
        assert_eq!(cold_index(&contract, &slot, 0), None);
    }

    #[test]
    fn test_cold_index_different_inputs() {
        let contract1 = [0x11u8; 20];
        let contract2 = [0x22u8; 20];
        let slot = [0x33u8; 32];
        let total = 1_000_000_000u64;

        let idx1 = cold_index(&contract1, &slot, total).unwrap();
        let idx2 = cold_index(&contract2, &slot, total).unwrap();

        assert_ne!(
            idx1, idx2,
            "Different contracts should likely have different indices"
        );
    }

    #[test]
    fn test_distribution_uniformity() {
        let num_slots = 100u64;
        let num_samples = 10_000;
        let mut buckets = vec![0u64; num_slots as usize];

        for i in 0..num_samples {
            let mut slot = [0u8; 32];
            slot[0..8].copy_from_slice(&(i as u64).to_le_bytes());
            let offset = slot_to_offset(&slot, num_slots).unwrap();
            buckets[offset as usize] += 1;
        }

        let expected = num_samples / num_slots;
        let tolerance = expected / 2;

        for (i, &count) in buckets.iter().enumerate() {
            assert!(
                count >= expected - tolerance && count <= expected + tolerance,
                "Bucket {} has {} entries, expected ~{} (+/- {})",
                i,
                count,
                expected,
                tolerance
            );
        }
    }
}
