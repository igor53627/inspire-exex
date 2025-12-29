//! Range-based delta file writer for efficient bucket index sync
//!
//! Maintains cumulative delta ranges (1, 10, 100, 1000, 10000 blocks) in a single file.
//! Clients can download just the range they need via HTTP range requests.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use inspire_core::bucket_index::range_delta::{
    RangeDeltaHeader, RangeEntry, DEFAULT_RANGES, HEADER_SIZE, RANGE_ENTRY_SIZE, VERSION,
};
use inspire_core::bucket_index::BucketDelta;

/// Writes and maintains range-based delta files
pub struct RangeDeltaWriter {
    data_dir: PathBuf,
    /// Recent deltas for each range tier
    /// deltas[i] contains last DEFAULT_RANGES[i] blocks worth of deltas
    deltas: Vec<VecDeque<BucketDelta>>,
    current_block: u64,
}

impl RangeDeltaWriter {
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        let num_ranges = DEFAULT_RANGES.len();
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
            deltas: vec![VecDeque::new(); num_ranges],
            current_block: 0,
        }
    }

    /// Load existing state from delta file if present
    pub fn load(&mut self) -> anyhow::Result<()> {
        let path = self.delta_file_path();
        if !path.exists() {
            return Ok(());
        }

        let mut file = File::open(&path)?;
        let mut header_buf = [0u8; HEADER_SIZE];
        file.read_exact(&mut header_buf)?;

        if let Some(header) = RangeDeltaHeader::from_bytes(&header_buf) {
            self.current_block = header.current_block;
            tracing::info!(block = self.current_block, "Loaded delta file state");
        }

        Ok(())
    }

    /// Add a new block's delta
    pub fn add_delta(&mut self, delta: BucketDelta) {
        self.current_block = delta.block_number;

        // Add to each range tier, trimming old entries
        for (i, range_blocks) in DEFAULT_RANGES.iter().enumerate() {
            self.deltas[i].push_back(delta.clone());

            // Trim to max blocks for this range
            while self.deltas[i].len() > *range_blocks as usize {
                self.deltas[i].pop_front();
            }
        }
    }

    /// Write the delta file with all ranges
    pub fn write(&self) -> anyhow::Result<PathBuf> {
        use inspire_core::bucket_index::range_delta::merge_deltas;

        std::fs::create_dir_all(&self.data_dir)?;
        let path = self.delta_file_path();

        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);

        // Write header
        let header = RangeDeltaHeader {
            version: VERSION,
            current_block: self.current_block,
            num_ranges: DEFAULT_RANGES.len() as u32,
        };
        writer.write_all(&header.to_bytes())?;

        // Calculate range data
        let mut range_data: Vec<Vec<u8>> = Vec::new();
        for deltas in &self.deltas {
            let deltas_vec: Vec<BucketDelta> = deltas.iter().cloned().collect();
            let merged = if deltas_vec.is_empty() {
                BucketDelta {
                    block_number: self.current_block,
                    updates: vec![],
                }
            } else {
                merge_deltas(&deltas_vec)
            };
            range_data.push(merged.to_bytes());
        }

        // Calculate offsets
        let directory_size = DEFAULT_RANGES.len() * RANGE_ENTRY_SIZE;
        let mut offset = (HEADER_SIZE + directory_size) as u32;

        // Write range directory
        for (i, data) in range_data.iter().enumerate() {
            let entry = RangeEntry {
                blocks_covered: DEFAULT_RANGES[i],
                offset,
                size: data.len() as u32,
                entry_count: self.deltas[i].len() as u32,
            };
            writer.write_all(&entry.to_bytes())?;
            offset += data.len() as u32;
        }

        // Write range data
        for data in &range_data {
            writer.write_all(data)?;
        }

        writer.flush()?;

        tracing::info!(
            path = %path.display(),
            block = self.current_block,
            ranges = DEFAULT_RANGES.len(),
            "Wrote range delta file"
        );

        Ok(path)
    }

    fn delta_file_path(&self) -> PathBuf {
        self.data_dir.join("bucket-deltas.bin")
    }

    /// Get current block number
    pub fn current_block(&self) -> u64 {
        self.current_block
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_delta_writer_basic() {
        let dir = tempdir().unwrap();
        let mut writer = RangeDeltaWriter::new(dir.path());

        // Add some deltas
        for block in 1..=5 {
            let delta = BucketDelta {
                block_number: block,
                updates: vec![(block as usize, block as u16)],
            };
            writer.add_delta(delta);
        }

        // Write file
        let path = writer.write().unwrap();
        assert!(path.exists());

        // Read back and verify header
        let data = std::fs::read(&path).unwrap();
        let header = RangeDeltaHeader::from_bytes(&data).unwrap();
        assert_eq!(header.current_block, 5);
        assert_eq!(header.num_ranges, DEFAULT_RANGES.len() as u32);
    }

    #[test]
    fn test_delta_trimming() {
        let dir = tempdir().unwrap();
        let mut writer = RangeDeltaWriter::new(dir.path());

        // Add more deltas than range-1 can hold
        for block in 1..=10 {
            let delta = BucketDelta {
                block_number: block,
                updates: vec![(0, block as u16)],
            };
            writer.add_delta(delta);
        }

        // Range-1 should only have 1 entry (last block)
        assert_eq!(writer.deltas[0].len(), 1);
        assert_eq!(writer.deltas[0].back().unwrap().block_number, 10);

        // Range-10 should have all 10 entries
        assert_eq!(writer.deltas[1].len(), 10);
    }
}
