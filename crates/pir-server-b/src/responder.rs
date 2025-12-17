//! Query responder - computes XOR of subset entries

use pir_core::{hint, prf::expand_seed, subset::CompressedQuery, Hint, ENTRY_SIZE};
use memmap2::Mmap;
use std::sync::Arc;

/// Database handle for responding to queries
pub struct Responder {
    mmap: Arc<Mmap>,
    entry_count: u64,
}

impl Responder {
    /// Create a responder from a memory-mapped database file
    pub fn new(mmap: Arc<Mmap>) -> Self {
        let entry_count = (mmap.len() / ENTRY_SIZE) as u64;
        Self { mmap, entry_count }
    }

    /// Process a compressed query and return the XOR result
    pub fn respond(&self, query: &CompressedQuery) -> Hint {
        // Expand seed to get subset indices
        let indices = expand_seed(
            &query.seed,
            query.subset_size as usize,
            query.domain_size,
        );

        // XOR all entries at those indices
        hint::compute_hint(&indices, |idx| self.get_entry(idx))
    }

    /// Get a single entry from the database
    fn get_entry(&self, idx: u64) -> [u8; ENTRY_SIZE] {
        let offset = (idx as usize) * ENTRY_SIZE;
        if offset + ENTRY_SIZE <= self.mmap.len() {
            self.mmap[offset..offset + ENTRY_SIZE].try_into().unwrap()
        } else {
            [0u8; ENTRY_SIZE]
        }
    }

    /// Entry count
    pub fn entry_count(&self) -> u64 {
        self.entry_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_db(entries: &[[u8; ENTRY_SIZE]]) -> (NamedTempFile, Arc<Mmap>) {
        let mut file = NamedTempFile::new().unwrap();
        for entry in entries {
            file.write_all(entry).unwrap();
        }
        file.flush().unwrap();
        
        let mmap = Arc::new(unsafe { Mmap::map(file.as_file()).unwrap() });
        (file, mmap)
    }

    #[test]
    fn test_responder_basic() {
        let entries = [
            [1u8; ENTRY_SIZE],
            [2u8; ENTRY_SIZE],
            [3u8; ENTRY_SIZE],
        ];
        
        let (_file, mmap) = create_test_db(&entries);
        let responder = Responder::new(mmap);
        
        assert_eq!(responder.entry_count(), 3);
    }
}
