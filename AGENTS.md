# AGENTS.md - pse-pir-exex

## Build Commands

```bash
# Build all crates
cargo build --release

# Build specific crate
cargo build -p pir-core --release
cargo build -p pir-seeder --release
cargo build -p pir-server-b --release
cargo build -p pir-client --release

# Run tests
cargo test --workspace

# Run benchmarks
cargo bench -p pir-core
```

## Project Structure

```
pse-pir-exex/
├── crates/
│   ├── pir-core/        # Shared primitives
│   ├── pir-seeder/      # Hint generator (ExEx)
│   ├── pir-server-b/    # Query server
│   └── pir-client/      # Client library
└── docs/
```

## Key Files

| File | Purpose |
|------|---------|
| `pir-core/src/prf.rs` | PRF (AES-based subset generation) |
| `pir-core/src/hint.rs` | XOR hint computation |
| `pir-core/src/subset.rs` | Subset and query compression |
| `pir-seeder/src/generator.rs` | Parallel hint generation |
| `pir-server-b/src/responder.rs` | Query processing |
| `pir-client/src/query.rs` | Client query logic |

## Testing

```bash
# Unit tests
cargo test -p pir-core

# Integration tests (requires database)
cargo test -p pir-server-b -- --ignored

# End-to-end test
./scripts/e2e-test.sh
```

## Related Projects

- `~/pse/inspire` - InsPIRe lattice-based PIR
- `~/pse/plinko-extractor` - Ethereum state extractor
- `~/pse/pse-client` - PSE client (base for extensions)

## Ethereum State Parameters

| Parameter | Value |
|-----------|-------|
| Total entries (N) | 2.73 billion |
| Subset size (sqrt(N)) | 52,250 |
| Number of hints | 6.7 million |
| Hint size | 32 bytes |
| Total hint storage | 214 MB |
| Database size | 87 GB |

## Development Notes

1. PRF uses AES-128 in counter mode
2. Hints are XOR parities (information-theoretic)
3. Query compression: send seed instead of indices (100x smaller)
4. Updates use XOR property: new = old XOR diff
