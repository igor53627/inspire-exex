//! Local hint storage

use pir_core::{subset::Subset, Hint, ENTRY_SIZE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Stored hint with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredHint {
    pub subset: Subset,
    pub hint: Hint,
}

/// Local hint store
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HintStore {
    /// Block number of the snapshot
    pub block_number: u64,
    /// All stored hints
    pub hints: Vec<StoredHint>,
    /// Index: target_index -> hint_ids that contain it
    #[serde(skip)]
    pub index: HashMap<u64, Vec<usize>>,
}

impl HintStore {
    /// Create a new empty store
    pub fn new() -> Self {
        Self::default()
    }

    /// Load from file
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let data = std::fs::read(path)?;
        let mut store: Self = bincode::deserialize(&data)?;
        store.rebuild_index();
        Ok(store)
    }

    /// Save to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let data = bincode::serialize(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Add hints from a manifest
    pub fn add_hints(&mut self, hints: Vec<(Subset, Hint)>, block_number: u64) {
        self.block_number = block_number;
        self.hints = hints
            .into_iter()
            .map(|(subset, hint)| StoredHint { subset, hint })
            .collect();
        self.rebuild_index();
    }

    /// Find a hint that contains the target index
    pub fn find_hint_for_target(&self, target: u64) -> Option<&StoredHint> {
        // First check index
        if let Some(hint_ids) = self.index.get(&target) {
            if let Some(&id) = hint_ids.first() {
                return self.hints.get(id);
            }
        }
        
        // Fallback to linear scan
        for hint in &self.hints {
            if hint.subset.contains(target) {
                return Some(hint);
            }
        }
        
        None
    }

    /// Rebuild the index (called after loading or adding hints)
    fn rebuild_index(&mut self) {
        self.index.clear();
        
        for (hint_id, stored) in self.hints.iter().enumerate() {
            let indices = stored.subset.expand();
            for idx in indices {
                self.index
                    .entry(idx)
                    .or_insert_with(Vec::new)
                    .push(hint_id);
            }
        }
    }

    /// Update hints based on state changes
    pub fn apply_delta(&mut self, changes: &[(u64, [u8; ENTRY_SIZE], [u8; ENTRY_SIZE])]) {
        for &(idx, ref old_value, ref new_value) in changes {
            if let Some(hint_ids) = self.index.get(&idx) {
                for &hint_id in hint_ids {
                    if let Some(stored) = self.hints.get_mut(hint_id) {
                        pir_core::hint::update_hint(&mut stored.hint, old_value, new_value);
                    }
                }
            }
        }
    }

    /// Total storage size
    pub fn size_bytes(&self) -> usize {
        self.hints.len() * (std::mem::size_of::<StoredHint>())
    }
}
