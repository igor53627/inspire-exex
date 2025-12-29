//! UBT (Unified Binary Trie) tree key computation for EIP-7864
//!
//! This module provides deterministic tree key computation for mapping Ethereum
//! state to PIR database indices per the EIP-7864 tree embedding specification.
//!
//! ## EIP-7864 Tree Embedding
//!
//! EIP-7864 co-locates account data to reduce unique stems. Each address has an
//! "account stem" (stem_pos=0) that contains:
//!
//! | Subindex | Content |
//! |----------|---------|
//! | 0 | basic_data (version, code_size, nonce, balance) |
//! | 1 | code_hash |
//! | 2-63 | reserved |
//! | 64-127 | storage slots 0-63 |
//! | 128-255 | code chunks 0-127 |
//!
//! Higher storage slots and code chunks use overflow stems, grouped 256 per stem.
//!
//! ## Tree Key Computation
//!
//! ```text
//! tree_index = compute_tree_index(leaf_type, logical_index)
//! stem_pos = tree_index[:31]  (31 bytes, big-endian)
//! subindex = tree_index[31]   (1 byte, 0-255)
//! stem = blake3(address32 || stem_pos)[:31]
//! tree_key = stem || subindex
//! ```
//!
//! Where `address32` is the 20-byte address left-padded with 12 zero bytes.
//!
//! ## Database Ordering
//!
//! The PIR database must be ordered by tree_key (lexicographically).
//! This allows O(log N) index computation with a small stem offset table.

use crate::{Address, StorageKey};

/// 31-byte stem (first 31 bytes of tree key)
pub type Stem = [u8; 31];

/// 32-byte tree key (stem || subindex)
pub type TreeKey = [u8; 32];

/// 32-byte tree index (stem_pos || subindex) - input to stem computation
pub type TreeIndex = [u8; 32];

/// EIP-7864 constants for tree embedding
pub mod constants {
    /// Subindex for basic_data leaf (nonce, balance, code_size, version)
    pub const BASIC_DATA_LEAF_KEY: u8 = 0;

    /// Subindex for code_hash leaf
    pub const CODE_HASH_LEAF_KEY: u8 = 1;

    /// First subindex for storage slots (slots 0-63 map to 64-127)
    pub const HEADER_STORAGE_OFFSET: u8 = 64;

    /// First subindex for code chunks (chunks 0-127 map to 128-255)
    pub const CODE_OFFSET: u8 = 128;

    /// Width of each stem subtree (256 entries per stem)
    pub const STEM_SUBTREE_WIDTH: u64 = 256;

    /// Offset for main storage (256^31) - storage slots >= 64 start here
    /// This is effectively 1 << 248 (256^31 = 2^248)
    pub const MAIN_STORAGE_OFFSET_BYTES: [u8; 32] = {
        let mut bytes = [0u8; 32];
        bytes[0] = 1; // Big-endian: 0x01 followed by 31 zero bytes = 256^31
        bytes
    };
}

/// Compute the tree_index for a storage slot per EIP-7864.
///
/// For slots 0-63: placed in account stem at subindex 64-127
/// For slots >= 64: placed in overflow stems at MAIN_STORAGE_OFFSET + slot
///
/// # Arguments
/// - `slot`: 32-byte storage slot key (interpreted as big-endian U256)
///
/// # Returns
/// 32-byte tree_index (stem_pos[31] || subindex[1])
pub fn compute_storage_tree_index(slot: &StorageKey) -> TreeIndex {
    // Check if slot < 64 (only low byte matters if rest is zero)
    let is_small_slot = slot[..31].iter().all(|&b| b == 0) && slot[31] < 64;

    if is_small_slot {
        // Small slot: place in account stem at subindex 64 + slot
        let mut tree_index = [0u8; 32];
        tree_index[31] = constants::HEADER_STORAGE_OFFSET + slot[31];
        tree_index
    } else {
        // Large slot: MAIN_STORAGE_OFFSET + slot
        // stem_pos = (MAIN_STORAGE_OFFSET + slot) / 256
        // subindex = (MAIN_STORAGE_OFFSET + slot) % 256
        add_with_offset(slot, &constants::MAIN_STORAGE_OFFSET_BYTES)
    }
}

/// Compute the tree_index for basic_data header.
///
/// Always placed in account stem at subindex 0.
#[inline]
pub fn compute_basic_data_tree_index() -> TreeIndex {
    let mut tree_index = [0u8; 32];
    tree_index[31] = constants::BASIC_DATA_LEAF_KEY;
    tree_index
}

/// Compute the tree_index for code_hash header.
///
/// Always placed in account stem at subindex 1.
#[inline]
pub fn compute_code_hash_tree_index() -> TreeIndex {
    let mut tree_index = [0u8; 32];
    tree_index[31] = constants::CODE_HASH_LEAF_KEY;
    tree_index
}

/// Compute the tree_index for a code chunk.
///
/// For chunks 0-127: placed in account stem at subindex 128-255
/// For chunks >= 128: placed in overflow stems
///
/// # Arguments
/// - `chunk_id`: The code chunk index (0, 1, 2, ...)
///
/// # Returns
/// 32-byte tree_index (stem_pos[31] || subindex[1])
pub fn compute_code_chunk_tree_index(chunk_id: u32) -> TreeIndex {
    let pos = constants::CODE_OFFSET as u64 + chunk_id as u64;
    let subindex = (pos % constants::STEM_SUBTREE_WIDTH) as u8;
    let stem_pos = pos / constants::STEM_SUBTREE_WIDTH;

    let mut tree_index = [0u8; 32];
    // stem_pos goes into bytes [0..31] (first 31 bytes), subindex goes into byte [31]
    // For small stem_pos values, write the low 8 bytes at the end of the 31-byte stem_pos
    // stem_pos as 31-byte BE: we put it at bytes [23..31] (last 8 bytes of stem_pos portion)
    tree_index[23..31].copy_from_slice(&stem_pos.to_be_bytes());
    tree_index[31] = subindex;
    tree_index
}

/// Add offset to slot, returning (stem_pos || subindex).
///
/// Computes: result = offset + slot, then splits into stem_pos (high 31 bytes)
/// and subindex (low byte).
fn add_with_offset(slot: &[u8; 32], offset: &[u8; 32]) -> TreeIndex {
    let mut result = [0u8; 32];
    let mut carry: u16 = 0;

    // Add from least significant byte
    for i in (0..32).rev() {
        let sum = slot[i] as u16 + offset[i] as u16 + carry;
        result[i] = sum as u8;
        carry = sum >> 8;
    }

    // Result is already (stem_pos || subindex) because:
    // - MAIN_STORAGE_OFFSET = 256^31 means offset[0] = 1, rest = 0
    // - Adding slot effectively shifts the position into overflow stems
    // - Low byte is subindex, high 31 bytes is stem_pos
    result
}

/// Compute the UBT stem from address and tree_index per EIP-7864.
///
/// The stem is the first 31 bytes of blake3(address32 || tree_index[:31]),
/// where address32 is the address left-padded to 32 bytes.
///
/// # Arguments
/// - `address`: 20-byte contract address
/// - `tree_index`: 32-byte tree index (stem_pos[31] || subindex[1])
///
/// # Returns
/// 31-byte stem
pub fn compute_stem(address: &Address, tree_index: &TreeIndex) -> Stem {
    let mut input = [0u8; 63]; // 32 (padded address) + 31 (stem_pos)
    input[12..32].copy_from_slice(address); // Left-pad address to 32 bytes
    input[32..63].copy_from_slice(&tree_index[..31]); // stem_pos (first 31 bytes)

    let hash = blake3::hash(&input);
    let mut stem = [0u8; 31];
    stem.copy_from_slice(&hash.as_bytes()[..31]);
    stem
}

/// Get the subindex from a tree_index.
///
/// The subindex is the last byte of tree_index, determining position within
/// a stem's 256-entry subtree.
#[inline]
pub fn get_subindex(tree_index: &TreeIndex) -> u8 {
    tree_index[31]
}

/// Compute the full 32-byte tree key for an address and tree_index.
///
/// This is the key that determines position in the UBT: stem || subindex.
pub fn compute_tree_key(address: &Address, tree_index: &TreeIndex) -> TreeKey {
    let stem = compute_stem(address, tree_index);
    let subindex = get_subindex(tree_index);

    let mut key = [0u8; 32];
    key[..31].copy_from_slice(&stem);
    key[31] = subindex;
    key
}

/// Compute tree key for a storage slot (convenience function).
///
/// Combines tree_index computation and stem hashing in one step.
pub fn compute_storage_tree_key(address: &Address, slot: &StorageKey) -> TreeKey {
    let tree_index = compute_storage_tree_index(slot);
    compute_tree_key(address, &tree_index)
}

/// Compute PIR database index for an address and tree_index.
///
/// # Arguments
/// - `address`: 20-byte contract address
/// - `tree_index`: 32-byte tree index
/// - `stem_offsets`: Sorted list of (stem, start_index) pairs
///
/// # Returns
/// - `Some(index)` if the stem is found in the database
/// - `None` if the stem is not in the database
pub fn compute_db_index(
    address: &Address,
    tree_index: &TreeIndex,
    stem_offsets: &[(Stem, u64)],
) -> Option<u64> {
    let stem = compute_stem(address, tree_index);
    let subindex = get_subindex(tree_index) as u64;

    // Binary search for the stem
    match stem_offsets.binary_search_by_key(&&stem, |(s, _)| s) {
        Ok(idx) => Some(stem_offsets[idx].1 + subindex),
        Err(_) => None,
    }
}

/// Pack basic_data value per EIP-7864 format.
///
/// | Offset | Size | Field |
/// |--------|------|-------|
/// | 0 | 1 | version (0) |
/// | 1 | 4 | reserved |
/// | 5 | 3 | code_size (big-endian) |
/// | 8 | 8 | nonce (big-endian) |
/// | 16 | 16 | balance (big-endian) |
pub fn pack_basic_data(nonce: u64, balance: u128, code_size: u32) -> [u8; 32] {
    let mut value = [0u8; 32];
    // version = 0 at offset 0 (already zero)
    // reserved at offset 1-4 (already zero)

    // code_size at offset 5-7 (3 bytes, big-endian)
    let cs_bytes = code_size.to_be_bytes();
    value[5..8].copy_from_slice(&cs_bytes[1..4]); // Take low 3 bytes

    // nonce at offset 8-15 (8 bytes, big-endian)
    value[8..16].copy_from_slice(&nonce.to_be_bytes());

    // balance at offset 16-31 (16 bytes, big-endian)
    value[16..32].copy_from_slice(&balance.to_be_bytes());

    value
}

/// Pack a code chunk value per EIP-7864 format.
///
/// | Offset | Size | Field |
/// |--------|------|-------|
/// | 0 | 1 | leading_pushdata_len (0-31) |
/// | 1 | 31 | code bytes |
///
/// # Arguments
/// - `code`: Full contract bytecode
/// - `chunk_id`: Chunk index (0, 1, 2, ...)
///
/// # Returns
/// 32-byte chunk value, or None if chunk_id is beyond code length
pub fn pack_code_chunk(code: &[u8], chunk_id: u32) -> Option<[u8; 32]> {
    const CHUNK_SIZE: usize = 31;
    let start = chunk_id as usize * CHUNK_SIZE;

    if start >= code.len() {
        return None;
    }

    let end = (start + CHUNK_SIZE).min(code.len());
    let chunk_bytes = &code[start..end];

    // Calculate leading_pushdata_len: how many bytes at the start of this chunk
    // are push data from a PUSH instruction that started in a previous chunk
    let leading = compute_leading_pushdata(code, chunk_id);

    let mut value = [0u8; 32];
    value[0] = leading;
    value[1..1 + chunk_bytes.len()].copy_from_slice(chunk_bytes);

    Some(value)
}

/// Compute how many leading bytes in a chunk are pushdata from previous chunks.
///
/// This scans from the start to find if a PUSH instruction spans into this chunk.
/// Handles truncated bytecode where PUSH immediates may be incomplete.
fn compute_leading_pushdata(code: &[u8], chunk_id: u32) -> u8 {
    const CHUNK_SIZE: usize = 31;
    let chunk_start = chunk_id as usize * CHUNK_SIZE;

    if chunk_start == 0 {
        return 0;
    }

    let mut pos = 0usize;
    while pos < chunk_start && pos < code.len() {
        let opcode = code[pos];

        // For PUSHn opcodes, clamp push_size to available bytes
        // This handles truncated bytecode correctly
        let push_size = if (0x60..=0x7f).contains(&opcode) {
            let declared = (opcode - 0x5f) as usize; // PUSH1=1, PUSH32=32
            let available = code.len().saturating_sub(pos + 1);
            declared.min(available)
        } else {
            0
        };

        let next_pos = pos + 1 + push_size;
        if next_pos > chunk_start {
            // This PUSH instruction spans into our chunk
            return (next_pos - chunk_start).min(CHUNK_SIZE) as u8;
        }
        pos = next_pos;
    }

    0
}

/// Calculate number of code chunks for a given code size.
pub fn code_chunk_count(code_size: usize) -> u32 {
    const CHUNK_SIZE: usize = 31;
    if code_size == 0 {
        0
    } else {
        ((code_size + CHUNK_SIZE - 1) / CHUNK_SIZE) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_storage_tree_index_small_slot() {
        // Slot 0 -> subindex 64
        let slot = [0u8; 32];
        let tree_index = compute_storage_tree_index(&slot);
        assert_eq!(tree_index[..31], [0u8; 31], "stem_pos should be 0");
        assert_eq!(tree_index[31], 64, "subindex should be 64");

        // Slot 63 -> subindex 127
        let mut slot = [0u8; 32];
        slot[31] = 63;
        let tree_index = compute_storage_tree_index(&slot);
        assert_eq!(tree_index[..31], [0u8; 31], "stem_pos should be 0");
        assert_eq!(tree_index[31], 127, "subindex should be 127");
    }

    #[test]
    fn test_compute_storage_tree_index_large_slot() {
        // Slot 64 -> overflow stem
        let mut slot = [0u8; 32];
        slot[31] = 64;
        let tree_index = compute_storage_tree_index(&slot);

        // MAIN_STORAGE_OFFSET + 64 = 256^31 + 64
        // Result = 0x01_00...00_40 (32 bytes: 0x01 at [0], 0x40 at [31])
        // stem_pos = result[0..31] = 0x01_00...00 (0x01 at [0])
        // subindex = result[31] = 0x40 = 64

        assert_eq!(tree_index[0], 1); // MAIN_STORAGE_OFFSET has 0x01 at byte 0
        assert_eq!(tree_index[31], 64, "subindex should be 64");
    }

    #[test]
    fn test_compute_stem_deterministic() {
        let address = [0x42u8; 20];
        let tree_index = [0x01u8; 32];

        let stem1 = compute_stem(&address, &tree_index);
        let stem2 = compute_stem(&address, &tree_index);

        assert_eq!(stem1, stem2, "Stem computation must be deterministic");
    }

    #[test]
    fn test_basic_data_tree_index() {
        let tree_index = compute_basic_data_tree_index();
        assert_eq!(tree_index[..31], [0u8; 31]);
        assert_eq!(tree_index[31], 0);
    }

    #[test]
    fn test_code_hash_tree_index() {
        let tree_index = compute_code_hash_tree_index();
        assert_eq!(tree_index[..31], [0u8; 31]);
        assert_eq!(tree_index[31], 1);
    }

    #[test]
    fn test_code_chunk_tree_index_small() {
        // Chunk 0 -> subindex 128
        let tree_index = compute_code_chunk_tree_index(0);
        assert_eq!(tree_index[..31], [0u8; 31]);
        assert_eq!(tree_index[31], 128);

        // Chunk 127 -> subindex 255
        let tree_index = compute_code_chunk_tree_index(127);
        assert_eq!(tree_index[..31], [0u8; 31]);
        assert_eq!(tree_index[31], 255);
    }

    #[test]
    fn test_code_chunk_tree_index_overflow() {
        // Chunk 128 -> pos = CODE_OFFSET + 128 = 128 + 128 = 256
        // stem_pos = 256 / 256 = 1, subindex = 256 % 256 = 0
        let tree_index = compute_code_chunk_tree_index(128);
        // stem_pos = 1 stored as 8-byte BE in [23..31], so [30] = 1
        assert_eq!(tree_index[30], 1, "stem_pos should be 1");
        assert_eq!(tree_index[31], 0, "subindex should be 0");

        // Chunk 383 -> pos = 128 + 383 = 511
        // stem_pos = 511 / 256 = 1, subindex = 511 % 256 = 255
        let tree_index = compute_code_chunk_tree_index(383);
        assert_eq!(tree_index[30], 1, "stem_pos should be 1");
        assert_eq!(tree_index[31], 255, "subindex should be 255");

        // Chunk 384 -> pos = 128 + 384 = 512
        // stem_pos = 512 / 256 = 2, subindex = 512 % 256 = 0
        let tree_index = compute_code_chunk_tree_index(384);
        assert_eq!(tree_index[30], 2, "stem_pos should be 2");
        assert_eq!(tree_index[31], 0, "subindex should be 0");
    }

    #[test]
    fn test_pack_basic_data() {
        let nonce = 42u64;
        let balance = 1_000_000_000_000_000_000u128; // 1 ETH in wei
        let code_size = 1234u32;

        let value = pack_basic_data(nonce, balance, code_size);

        // Check version = 0
        assert_eq!(value[0], 0);

        // Check code_size (3 bytes at offset 5)
        assert_eq!(value[5..8], [0x00, 0x04, 0xd2]); // 1234 in BE

        // Check nonce (8 bytes at offset 8)
        assert_eq!(value[8..16], 42u64.to_be_bytes());

        // Check balance (16 bytes at offset 16)
        assert_eq!(value[16..32], balance.to_be_bytes());
    }

    #[test]
    fn test_pack_code_chunk() {
        let code = vec![0x60, 0x80, 0x60, 0x40]; // PUSH1 0x80, PUSH1 0x40

        let chunk = pack_code_chunk(&code, 0).unwrap();
        assert_eq!(chunk[0], 0); // No leading pushdata
        assert_eq!(&chunk[1..5], &code[..4]);

        // Test beyond code length
        assert!(pack_code_chunk(&code, 100).is_none());
    }

    #[test]
    fn test_code_chunk_count() {
        assert_eq!(code_chunk_count(0), 0);
        assert_eq!(code_chunk_count(1), 1);
        assert_eq!(code_chunk_count(31), 1);
        assert_eq!(code_chunk_count(32), 2);
        assert_eq!(code_chunk_count(62), 2);
        assert_eq!(code_chunk_count(63), 3);
    }

    #[test]
    fn test_different_addresses_different_stems() {
        let address1 = [0x11u8; 20];
        let address2 = [0x22u8; 20];
        let tree_index = [0u8; 32];

        let stem1 = compute_stem(&address1, &tree_index);
        let stem2 = compute_stem(&address2, &tree_index);

        assert_ne!(
            stem1, stem2,
            "Different addresses should have different stems"
        );
    }

    #[test]
    fn test_same_address_same_stem_for_account_leaves() {
        let address = [0x42u8; 20];

        // All account leaves (basic_data, code_hash, slots 0-63, chunks 0-127)
        // should share the same stem (stem_pos = 0)
        let basic_data = compute_basic_data_tree_index();
        let code_hash = compute_code_hash_tree_index();
        let slot_0 = compute_storage_tree_index(&[0u8; 32]);
        let chunk_0 = compute_code_chunk_tree_index(0);

        let stem_basic = compute_stem(&address, &basic_data);
        let stem_code = compute_stem(&address, &code_hash);
        let stem_slot = compute_stem(&address, &slot_0);
        let stem_chunk = compute_stem(&address, &chunk_0);

        assert_eq!(stem_basic, stem_code);
        assert_eq!(stem_code, stem_slot);
        assert_eq!(stem_slot, stem_chunk);
    }

    #[test]
    fn test_compute_db_index() {
        let address = [0x42u8; 20];
        let tree_index = compute_storage_tree_index(&[0u8; 32]); // slot 0

        let stem = compute_stem(&address, &tree_index);

        // Create a stem offset table
        let stem_offsets = vec![
            ([0x00u8; 31], 0u64),
            (stem, 1000u64), // Our stem starts at index 1000
            ([0xffu8; 31], 2000u64),
        ];

        let index = compute_db_index(&address, &tree_index, &stem_offsets);
        // slot 0 has subindex 64, so index = 1000 + 64
        assert_eq!(index, Some(1000 + 64));
    }

    #[test]
    fn test_compute_db_index_not_found() {
        let address = [0x42u8; 20];
        let tree_index = [0x01u8; 32];

        // Empty stem offset table
        let stem_offsets: Vec<(Stem, u64)> = vec![];

        let index = compute_db_index(&address, &tree_index, &stem_offsets);
        assert_eq!(index, None);
    }

    #[test]
    fn test_known_vector() {
        // Test vector for cross-implementation compatibility
        // address: 0x0000000000000000000000000000000000000001
        // tree_index for basic_data (stem_pos=0, subindex=0)
        let mut address = [0u8; 20];
        address[19] = 1;
        let tree_index = compute_basic_data_tree_index();

        let stem = compute_stem(&address, &tree_index);
        let key = compute_tree_key(&address, &tree_index);

        // Verify stem computation
        // input = address32 || stem_pos = [0;12] ++ [0;19,1] ++ [0;31]
        let mut expected_input = [0u8; 63];
        expected_input[31] = 1; // address byte
        let expected_hash = blake3::hash(&expected_input);
        let expected_stem: [u8; 31] = expected_hash.as_bytes()[..31].try_into().unwrap();

        assert_eq!(stem, expected_stem);
        assert_eq!(&key[..31], &expected_stem);
        assert_eq!(key[31], 0); // subindex for basic_data
    }
}
