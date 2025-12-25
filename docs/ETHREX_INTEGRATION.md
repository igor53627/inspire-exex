# ethrex Integration Guide

This guide describes how to integrate inspire-exex with [ethrex](https://github.com/igor53627/ethrex) for private Ethereum state queries using PIR.

## Overview

```
ethrex (UBT @ block N)
    --> ethrex-pir-export (iterate PLAIN_STORAGE)
    --> state.bin (84-byte records)
    --> inspire-setup (encode PIR database)
    --> database.bin
    --> inspire-server (serve queries)
```

## Prerequisites

1. **ethrex node** with UBT sync completed
   - Must have `ubt` feature enabled during sync
   - `PLAIN_STORAGE` table populated with preimages

2. **Binaries built**:
   - `ethrex-pir-export` (from ethrex repo)
   - `inspire-setup` (from inspire-rs repo)
   - `inspire-server` (from inspire-rs repo)

## Data Formats

### state.bin (ethrex-pir-export output)

Fixed 84-byte records, no headers:

| Field | Size | Description |
|-------|------|-------------|
| address | 20 bytes | Contract address |
| slot | 32 bytes | Storage slot key |
| value | 32 bytes | Storage value (UBT native) |

### database.bin (inspire-setup output)

PIR-encoded database. See inspire-rs documentation for format details.

## Step-by-Step Integration

### 1. Export State from ethrex

```bash
ethrex-pir-export \
    --datadir /path/to/ethrex \
    --block 9900000 \
    --output state.bin
```

### 2. Encode PIR Database

```bash
inspire-setup state.bin database.bin
```

### 3. Start PIR Server

```bash
inspire-server database.bin --port 3000
```

### 4. Query Privately

```bash
# Clients compute stem/subindex via EIP-7864, then derive the PIR index:
#   stem = pedersen_hash(address || slot[:31])
#   subindex = slot[31]
#   index = stem_to_db_offset(stem) + subindex

# Using inspire-client with stem/subindex
inspire-client http://localhost:3000 --stem 0x... --subindex 0

# For debugging, you can pass a raw index directly:
# inspire-client http://localhost:3000 --index 12345
```

## Integration Test

Run the automated integration test:

```bash
./scripts/ethrex-integration-test.sh /path/to/ethrex-datadir /tmp/pir-test
```

## Architecture Notes

### Sidecar Deployment

The PIR components run as sidecars to ethrex:

- **ethrex**: Standard Ethereum node with UBT tracking
- **ethrex-pir-export**: Periodic snapshots (e.g., every hour)
- **inspire-server**: Separate service with hot-reload

This keeps heavy PIR crypto out of the consensus path.

### State Freshness

Current model uses **periodic snapshots**:

1. Export state at finalized block N
2. Encode PIR database
3. Hot-reload into running server

For real-time updates, see advanced integration in issue #54.

## Troubleshooting

### "Incompatible DB Version" Error

The ethrex database was created with a different schema version. Options:
- Re-sync with the current ethrex version
- Use the same ethrex binary version for both node and export

### Empty PLAIN_STORAGE

The node must be synced with `ubt` feature enabled from genesis (or from a snapshot that includes preimages).

### Export Too Slow

- Ensure RocksDB is using SSD storage
- Consider exporting during low-traffic periods
- Future: implement streaming directly from RocksDB iterator

## Related Issues

- [ethrex#13](https://github.com/igor53627/ethrex/issues/13) - UBT state export implementation
- [inspire-exex#54](https://github.com/igor53627/inspire-exex/issues/54) - MPT→UBT migration research
