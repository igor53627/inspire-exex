//! Bucket Index for WASM client
//!
//! Uses 256K buckets (18-bit hash prefix) for O(1) lookup of (address, slot) -> bucket range.
//! Wraps shared logic from inspire-core with wasm-bindgen annotations.
//!
//! ## Range-Based Delta Sync
//!
//! For efficient sync, use `apply_range_delta()` with data from `/index/deltas`:
//! 1. Fetch `/index/deltas/info` to get range metadata
//! 2. Pick smallest range covering your sync gap
//! 3. Fetch that range via HTTP Range request
//! 4. Call `apply_range_delta()` with the merged delta

use inspire_core::bucket_index::{
    compute_bucket_id, compute_cumulative,
    range_delta::{RangeDeltaHeader, RangeEntry, HEADER_SIZE, RANGE_ENTRY_SIZE},
    BucketDelta as CoreDelta, NUM_BUCKETS,
};
use wasm_bindgen::prelude::*;

/// Bucket index for sparse PIR lookups (WASM-compatible)
#[wasm_bindgen]
pub struct BucketIndex {
    counts: Vec<u16>,
    cumulative: Vec<u64>,
}

#[wasm_bindgen]
impl BucketIndex {
    /// Load bucket index from uncompressed binary (512 KB)
    /// Use /index/raw endpoint which returns uncompressed data for WASM clients.
    #[wasm_bindgen(constructor)]
    pub fn from_bytes(data: &[u8]) -> Result<BucketIndex, JsValue> {
        if data.len() != NUM_BUCKETS * 2 {
            return Err(JsValue::from_str(&format!(
                "Invalid bucket index size: expected {}, got {}",
                NUM_BUCKETS * 2,
                data.len()
            )));
        }

        let mut counts = Vec::with_capacity(NUM_BUCKETS);
        for chunk in data.chunks_exact(2) {
            counts.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }

        let cumulative = compute_cumulative(&counts);

        Ok(BucketIndex { counts, cumulative })
    }

    /// Get total number of entries across all buckets
    #[wasm_bindgen(getter)]
    pub fn total_entries(&self) -> u64 {
        self.cumulative[NUM_BUCKETS]
    }

    /// Look up the bucket range for a (address, slot) pair
    ///
    /// Returns [bucket_id, start_index, count]
    pub fn lookup(&self, address: &[u8], slot: &[u8]) -> Result<Vec<u64>, JsValue> {
        if address.len() != 20 {
            return Err(JsValue::from_str("Address must be 20 bytes"));
        }
        if slot.len() != 32 {
            return Err(JsValue::from_str("Slot must be 32 bytes"));
        }

        let addr: [u8; 20] = address.try_into().unwrap();
        let sl: [u8; 32] = slot.try_into().unwrap();

        let bucket_id = compute_bucket_id(&addr, &sl);
        let start = self.cumulative[bucket_id];
        let count = self.counts[bucket_id] as u64;

        Ok(vec![bucket_id as u64, start, count])
    }

    /// Get count for a specific bucket
    pub fn bucket_count(&self, bucket_id: usize) -> u16 {
        self.counts.get(bucket_id).copied().unwrap_or(0)
    }

    /// Get start index for a specific bucket
    pub fn bucket_start(&self, bucket_id: usize) -> u64 {
        self.cumulative.get(bucket_id).copied().unwrap_or(0)
    }

    /// Apply a delta update (from websocket)
    ///
    /// Delta format: block_num:8 + count:4 + (bucket_id:4 + count:2)*
    /// Returns the block number from the delta.
    pub fn apply_delta(&mut self, data: &[u8]) -> Result<u64, JsValue> {
        let delta = CoreDelta::from_bytes(data).map_err(|e| JsValue::from_str(&e.to_string()))?;

        for &(bucket_id, new_count) in &delta.updates {
            if bucket_id < NUM_BUCKETS {
                self.counts[bucket_id] = new_count;
            }
        }

        // Recompute cumulative sums
        self.cumulative = compute_cumulative(&self.counts);

        Ok(delta.block_number)
    }

    /// Apply a range delta (from /index/deltas endpoint)
    ///
    /// This applies a pre-merged cumulative delta from a specific range.
    /// The data is raw BucketDelta bytes extracted from the range delta file.
    /// Returns the block number from the delta.
    pub fn apply_range_delta(&mut self, data: &[u8]) -> Result<u64, JsValue> {
        // Range delta data is just a BucketDelta
        self.apply_delta(data)
    }
}

/// Range delta file info (returned by /index/deltas/info)
#[wasm_bindgen]
pub struct RangeDeltaInfo {
    current_block: u64,
    ranges: Vec<RangeInfoEntry>,
}

struct RangeInfoEntry {
    blocks_covered: u32,
    offset: u32,
    size: u32,
}

#[wasm_bindgen]
impl RangeDeltaInfo {
    /// Parse range delta info from full file header
    ///
    /// Pass the first 64 + num_ranges*16 bytes of the file
    #[wasm_bindgen(constructor)]
    pub fn from_bytes(data: &[u8]) -> Result<RangeDeltaInfo, JsValue> {
        if data.len() < HEADER_SIZE {
            return Err(JsValue::from_str("Data too short for header"));
        }

        let header = RangeDeltaHeader::from_bytes(data)
            .ok_or_else(|| JsValue::from_str("Invalid range delta header"))?;

        let mut ranges = Vec::new();
        let mut offset = HEADER_SIZE;

        for _ in 0..header.num_ranges {
            if offset + RANGE_ENTRY_SIZE > data.len() {
                break;
            }
            let entry = RangeEntry::from_bytes(&data[offset..])
                .ok_or_else(|| JsValue::from_str("Invalid range entry"))?;
            ranges.push(RangeInfoEntry {
                blocks_covered: entry.blocks_covered,
                offset: entry.offset,
                size: entry.size,
            });
            offset += RANGE_ENTRY_SIZE;
        }

        Ok(RangeDeltaInfo {
            current_block: header.current_block,
            ranges,
        })
    }

    /// Get current block number
    #[wasm_bindgen(getter)]
    pub fn current_block(&self) -> u64 {
        self.current_block
    }

    /// Get number of ranges
    #[wasm_bindgen(getter)]
    pub fn num_ranges(&self) -> usize {
        self.ranges.len()
    }

    /// Select the best range for a given sync gap
    ///
    /// Returns range index, or -1 if too far behind (need full index)
    pub fn select_range(&self, behind_blocks: u64) -> i32 {
        if behind_blocks == 0 {
            return -1;
        }
        for (i, range) in self.ranges.iter().enumerate() {
            if behind_blocks <= range.blocks_covered as u64 {
                return i as i32;
            }
        }
        -1 // Too far behind
    }

    /// Get byte offset for a range
    pub fn range_offset(&self, range_index: usize) -> u32 {
        self.ranges.get(range_index).map(|r| r.offset).unwrap_or(0)
    }

    /// Get byte size for a range
    pub fn range_size(&self, range_index: usize) -> u32 {
        self.ranges.get(range_index).map(|r| r.size).unwrap_or(0)
    }

    /// Get blocks covered by a range
    pub fn range_blocks(&self, range_index: usize) -> u32 {
        self.ranges
            .get(range_index)
            .map(|r| r.blocks_covered)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_bucket_id_deterministic() {
        let address = [0x42u8; 20];
        let slot = [0x01u8; 32];

        let id1 = compute_bucket_id(&address, &slot);
        let id2 = compute_bucket_id(&address, &slot);
        assert_eq!(id1, id2);
        assert!(id1 < NUM_BUCKETS);
    }

    #[wasm_bindgen_test]
    fn test_bucket_index_from_bytes() {
        let mut data = vec![0u8; NUM_BUCKETS * 2];
        data[0] = 10; // bucket 0 = 10
        data[2] = 5; // bucket 1 = 5

        let index = BucketIndex::from_bytes(&data).unwrap();

        assert_eq!(index.bucket_count(0), 10);
        assert_eq!(index.bucket_count(1), 5);
        assert_eq!(index.bucket_start(0), 0);
        assert_eq!(index.bucket_start(1), 10);
        assert_eq!(index.bucket_start(2), 15);
    }

    #[wasm_bindgen_test]
    fn test_apply_delta() {
        let mut data = vec![0u8; NUM_BUCKETS * 2];
        data[0] = 10;

        let mut index = BucketIndex::from_bytes(&data).unwrap();
        assert_eq!(index.bucket_count(0), 10);

        // Create delta bytes
        let delta = CoreDelta {
            block_number: 42,
            updates: vec![(0, 15)],
        };
        let delta_bytes = delta.to_bytes();

        let block = index.apply_delta(&delta_bytes).unwrap();
        assert_eq!(block, 42);
        assert_eq!(index.bucket_count(0), 15);
    }
}
