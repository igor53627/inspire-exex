# State Format Specification

This document specifies the `state.bin` format for PIR database generation.

## Format Overview

```
+------------------+
| Header (64 bytes)|
+------------------+
| Entry 0 (84 B)   |
+------------------+
| Entry 1 (84 B)   |
+------------------+
| ...              |
+------------------+
| Entry N-1 (84 B) |
+------------------+
```

## Header (64 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | magic | `0x50495232` ("PIR2" in ASCII) |
| 4 | 2 | version | Format version (1) |
| 6 | 2 | entry_size | Bytes per entry (84) |
| 8 | 8 | entry_count | Number of entries |
| 16 | 8 | block_number | Snapshot block number |
| 24 | 8 | chain_id | Ethereum chain ID |
| 32 | 32 | block_hash | UBT root hash for verification (or block hash, zero if unknown) |

All integers are little-endian.

### Magic Number

The magic `0x50495232` ("PIR2") identifies this as an inspire state file. Future formats would use different magic bytes.

## Entry Format (84 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 20 | address | Contract address |
| 20 | 32 | slot | Storage slot key |
| 52 | 32 | value | Storage value |

## Ordering

Entries MUST be sorted by `keccak256(address || slot)` for bucket index compatibility.

The PIR database is laid out by bucket ID:
```
[bucket 0 entries][bucket 1 entries]...[bucket N entries]
```

See [bucket_index.rs](../crates/inspire-core/src/bucket_index.rs) for the 18-bit bucket ID computation.

## Rust Types

```rust
/// State file header
#[repr(C, packed)]
pub struct StateHeader {
    pub magic: [u8; 4],        // b"PIR2"
    pub version: u16,          // 1
    pub entry_size: u16,       // 84
    pub entry_count: u64,
    pub block_number: u64,
    pub chain_id: u64,
    pub block_hash: [u8; 32],
}

/// Storage entry
#[repr(C, packed)]
pub struct StorageEntry {
    pub address: [u8; 20],
    pub slot: [u8; 32],
    pub value: [u8; 32],
}

const STATE_MAGIC: [u8; 4] = *b"PIR2";
const STATE_HEADER_SIZE: usize = 64;
const STATE_ENTRY_SIZE: usize = 84;
```

## Example

A file with 1000 entries at block 20000000 on mainnet (chain_id=1):

```
Offset 0x00: 50 49 52 32  # "PIR2"
Offset 0x04: 01 00        # version = 1
Offset 0x06: 54 00        # entry_size = 84
Offset 0x08: e8 03 00 00 00 00 00 00  # entry_count = 1000
Offset 0x10: 00 2d 31 01 00 00 00 00  # block_number = 20000000
Offset 0x18: 01 00 00 00 00 00 00 00  # chain_id = 1
Offset 0x20: [32 bytes block hash or zeros]
Offset 0x40: [first entry starts here]
```

## References

- [ETHREX_INTEGRATION.md](ETHREX_INTEGRATION.md) - ethrex export pipeline
- [inspire-exex#54](https://github.com/igor53627/inspire-exex/issues/54) - UBT extraction layer
