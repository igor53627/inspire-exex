# Burner Wallet

Privacy-preserving burner wallet with:
- **PIR Balance Queries**: Query balances without revealing which address
- **Helios Verification**: Light client verification of snapshot blocks
- **EIP-7702 Signing**: Delegate EOA to smart contracts for batch transactions
- **Tenderly Fork**: Pre-configured Sepolia fork with test funding

## Architecture

```
Browser
  |
  +-- alloy-wasm (wallet, signing)
  +-- @a16z/helios (light client, bundled locally)
  +-- inspire-client-wasm (PIR queries)
  |
Axum Server (SSR HTML)
  |
Tenderly Virtual TestNet (Sepolia Fork)
```

## Quick Start

```bash
# Build WASM packages
./build-wasm.sh

# Run server
cargo run --release

# Open http://localhost:3000
```

## Tenderly Fork

The wallet is pre-configured to use a Tenderly Virtual TestNet fork of Sepolia:

- **Public RPC**: `https://virtual.sepolia.eu.rpc.tenderly.co/1732ab6a-5418-4eb0-acee-3654f6dc79e7`
- **Admin RPC**: Used for `tenderly_setBalance` to fund test accounts
- **Chain ID**: 11155111 (Sepolia)

### Pre-funded Test Account

| Address | ETH | USDC |
|---------|-----|------|
| `0xa158f725512b2f4365bEfc29e144A1f2f48f746f` | 100 | 10,000 |

Click "Fund from Test Account" in the UI to fund your burner wallet with 10 ETH + 1000 USDC.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `LISTEN_ADDR` | `0.0.0.0:3000` | Server bind address |
| `PIR_SERVER_URL` | `http://localhost:3001` | PIR server endpoint |
| `NETWORK` | `sepolia` | Ethereum network |

## Features

- [x] Generate burner wallet (localStorage)
- [x] Import existing private key
- [x] EIP-7702 authorization signing
- [x] Server View (privacy demo)
- [x] Tenderly fork integration
- [x] Fund wallet from test account
- [x] Real ETH/USDC balance display
- [ ] PIR balance queries (requires inspire-server)
- [ ] Helios snapshot verification
- [ ] Batch transaction sending

## E2E Tests

```bash
# Install test dependencies (includes Helios)
npm install

# Run tests (automatically prepares Helios assets)
npm test

# Run headed (visible browser)
npm run test:headed
```

Note: `npm test` automatically runs `prepare-helios` to copy Helios from node_modules to static/helios/.

Tests cover (42 tests):
- WASM loading and initialization
- Wallet generation/import/persistence/clearing
- Tenderly RPC connection and configuration
- Funding from test account (single and multiple)
- Real balance display, refresh, and zero handling
- EIP-7702 authorization signing (default/custom contracts, different nonces)
- Authorization RLP format validation
- Server View privacy panel content
- UI state transitions and initial states
- Error handling (invalid keys, empty imports)
- Balance persistence after page reload
- Input validation (addresses, nonces, RPC URLs)
- Mobile viewport responsiveness
- Chain ID and block number validation
- Log scrolling and rapid operations
- Password input masking for private keys

## WASM Packages

| Package | Size (gzip) | Function |
|---------|-------------|----------|
| `alloy-wasm` | ~89 KB | Wallet, signing, ABI encoding |
| `inspire-client-wasm` | TBD | PIR balance queries |
| `@a16z/helios` | ~1.2 MB | Light client verification (bundled locally) |
