//! Lane routing: determines which lane handles a query

use crate::{Address, HotLaneManifest, Lane, StorageKey};
use crate::indexing::{hot_index, cold_index};
use std::collections::HashSet;

/// Routes queries to the appropriate lane based on contract address
pub struct LaneRouter {
    hot_addresses: HashSet<Address>,
    manifest: HotLaneManifest,
    cold_total_entries: u64,
}

impl LaneRouter {
    /// Create a router from a hot lane manifest
    pub fn new(manifest: HotLaneManifest) -> Self {
        Self::with_cold_entries(manifest, 0)
    }

    /// Create a router with known cold lane total entries
    pub fn with_cold_entries(manifest: HotLaneManifest, cold_total_entries: u64) -> Self {
        let hot_addresses = manifest.address_set();
        Self {
            hot_addresses,
            manifest,
            cold_total_entries,
        }
    }

    /// Set the cold lane total entries (for cold lane index calculation)
    pub fn set_cold_entries(&mut self, total: u64) {
        self.cold_total_entries = total;
    }

    /// Route a query to the appropriate lane
    pub fn route(&self, contract: &Address) -> Lane {
        if self.hot_addresses.contains(contract) {
            Lane::Hot
        } else {
            Lane::Cold
        }
    }

    /// Get the index within the hot lane database for a (contract, slot) pair.
    ///
    /// Uses the contract's start_index and slot_count from the manifest to compute:
    /// `start_index + slot_to_offset(slot, slot_count)`
    ///
    /// Returns `None` if:
    /// - The contract is not in the hot lane
    /// - The contract has `slot_count == 0` (invalid configuration)
    pub fn get_hot_index(&self, contract: &Address, slot: &StorageKey) -> Option<u64> {
        let contract_info = self.manifest.get_contract(contract)?;
        hot_index(contract_info.start_index, slot, contract_info.slot_count)
    }

    /// Get the index within the cold lane database for a (contract, slot) pair.
    ///
    /// Uses a global hash of (contract, slot) mod total_entries.
    ///
    /// Returns `None` if `cold_total_entries == 0` (not initialized).
    pub fn get_cold_index(&self, contract: &Address, slot: &StorageKey) -> Option<u64> {
        cold_index(contract, slot, self.cold_total_entries)
    }

    /// Get the manifest
    pub fn manifest(&self) -> &HotLaneManifest {
        &self.manifest
    }

    /// Number of contracts in hot lane
    pub fn hot_contract_count(&self) -> usize {
        self.hot_addresses.len()
    }

    /// Check if address is in hot lane
    pub fn is_hot(&self, address: &Address) -> bool {
        self.hot_addresses.contains(address)
    }
}

/// Query target: identifies what the client wants to query
#[derive(Debug, Clone)]
pub struct QueryTarget {
    /// Contract address
    pub contract: Address,
    /// Storage slot key
    pub slot: StorageKey,
}

impl QueryTarget {
    pub fn new(contract: Address, slot: StorageKey) -> Self {
        Self { contract, slot }
    }
}

/// Routed query: a query with its determined lane and index
#[derive(Debug, Clone)]
pub struct RoutedQuery {
    /// Original query target
    pub target: QueryTarget,
    /// Determined lane
    pub lane: Lane,
    /// Index within the lane's database
    pub index: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexing::slot_to_offset;

    fn create_test_manifest() -> HotLaneManifest {
        let mut manifest = HotLaneManifest::new(1000);
        manifest.add_contract([0x11u8; 20], "USDC".into(), 1000, "token".into());
        manifest.add_contract([0x22u8; 20], "WETH".into(), 500, "token".into());
        manifest
    }

    #[test]
    fn test_routing() {
        let router = LaneRouter::new(create_test_manifest());
        
        assert_eq!(router.route(&[0x11u8; 20]), Lane::Hot);
        assert_eq!(router.route(&[0x22u8; 20]), Lane::Hot);
        assert_eq!(router.route(&[0x33u8; 20]), Lane::Cold);
    }

    #[test]
    fn test_hot_index_with_slot() {
        let router = LaneRouter::new(create_test_manifest());
        let slot = [0x42u8; 32];
        
        // First contract (USDC): start_index=0, slot_count=1000
        let idx1 = router.get_hot_index(&[0x11u8; 20], &slot).unwrap();
        let expected_offset = slot_to_offset(&slot, 1000).unwrap();
        assert_eq!(idx1, expected_offset);
        assert!(idx1 < 1000, "should be within USDC's range");
        
        // Second contract (WETH): start_index=1000, slot_count=500
        let idx2 = router.get_hot_index(&[0x22u8; 20], &slot).unwrap();
        let expected_offset2 = slot_to_offset(&slot, 500).unwrap();
        assert_eq!(idx2, 1000 + expected_offset2);
        assert!(idx2 >= 1000 && idx2 < 1500, "should be within WETH's range");
        
        // Non-existent contract returns None
        assert_eq!(router.get_hot_index(&[0x33u8; 20], &slot), None);
    }

    #[test]
    fn test_hot_index_different_slots() {
        let router = LaneRouter::new(create_test_manifest());
        let slot1 = [0x01u8; 32];
        let slot2 = [0x02u8; 32];
        let contract = [0x11u8; 20];
        
        let idx1 = router.get_hot_index(&contract, &slot1).unwrap();
        let idx2 = router.get_hot_index(&contract, &slot2).unwrap();
        
        // Different slots should (likely) produce different indices
        assert_ne!(idx1, idx2, "Different slots should map to different indices");
        
        // Both should be within the contract's range
        assert!(idx1 < 1000);
        assert!(idx2 < 1000);
    }

    #[test]
    fn test_cold_index() {
        let router = LaneRouter::with_cold_entries(create_test_manifest(), 2_700_000_000);
        let contract = [0x33u8; 20];
        let slot = [0x44u8; 32];
        
        let idx = router.get_cold_index(&contract, &slot).unwrap();
        assert!(idx < 2_700_000_000, "should be within cold lane range");
        
        // Deterministic
        let idx2 = router.get_cold_index(&contract, &slot).unwrap();
        assert_eq!(idx, idx2);
    }

    #[test]
    fn test_cold_index_uninitialized() {
        let router = LaneRouter::new(create_test_manifest());
        let contract = [0x33u8; 20];
        let slot = [0x44u8; 32];
        
        // Should return None when cold_total_entries is 0
        assert_eq!(router.get_cold_index(&contract, &slot), None);
    }

    #[test]
    fn test_cold_index_different_inputs() {
        let router = LaneRouter::with_cold_entries(create_test_manifest(), 1_000_000_000);
        
        let idx1 = router.get_cold_index(&[0x11u8; 20], &[0x22u8; 32]).unwrap();
        let idx2 = router.get_cold_index(&[0x33u8; 20], &[0x22u8; 32]).unwrap();
        let idx3 = router.get_cold_index(&[0x11u8; 20], &[0x44u8; 32]).unwrap();
        
        // Different inputs should produce different indices (with high probability)
        assert_ne!(idx1, idx2);
        assert_ne!(idx1, idx3);
    }
}
