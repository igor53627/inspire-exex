# Hot Contracts List

> **Status**: This is a **future optimization**. The initial version uses a
> single lane with full Ethereum state. This document describes the planned
> two-lane architecture for improved performance.

This document describes the curated list of Ethereum contracts planned for the
**hot lane** in Two-Lane InsPIRe PIR. The hot lane will contain approximately
1,000 of the most frequently queried contracts, reducing average query latency
while maintaining full PIR privacy within the lane.

## Overview

| Metric | Value |
|--------|-------|
| Target contracts | ~1,000 |
| Target entries | ~1,000,000 |
| Database size | ~32 MB |
| Query size | ~10 KB (InsPIRe^2+) |

## Data Sources

The hot lane contract list is derived from multiple sources:

### Primary Sources

| Source | Data Type | Update Frequency | Weight |
|--------|-----------|------------------|--------|
| [Etherscan Gas Tracker](https://etherscan.io/gastracker) | Real-time gas usage | Live (24h rolling) | High |
| [Dune Analytics](https://dune.com/nbphuoc/eth-top-contracts) | Historical gas/tx/users | Daily | High |
| [DeFiLlama](https://defillama.com/chain/ethereum) | TVL rankings | Daily | Medium |

### Supplementary Sources

- Curated protocol lists (privacy protocols, bridges)
- Manual additions for known high-value contracts

## Categories

Contracts are organized into categories with associated weight multipliers for scoring:

| Category | Weight | Description | Examples |
|----------|--------|-------------|----------|
| `privacy` | 3.0x | Privacy-preserving protocols | Tornado Cash, Railgun, Aztec |
| `bridge` | 2.0x | Cross-chain bridges | Arbitrum, Optimism, Polygon bridges |
| `stablecoin` | 1.5x | Stablecoins | USDC, USDT, DAI |
| `dex` | 1.5x | Decentralized exchanges | Uniswap, Curve, Balancer |
| `lending` | 1.5x | Lending protocols | Aave, Compound, Spark |
| `liquid_staking` | 1.5x | Liquid staking derivatives | Lido stETH, Rocket Pool |
| `restaking` | 1.5x | Restaking protocols | EigenLayer, ether.fi |
| `nft` | 1.0x | NFT marketplaces | Seaport, Blur, OpenSea |
| `token` | 1.0x | Wrapped/governance tokens | WETH, WBTC, UNI, AAVE |
| `l2` | 1.0x | L2 contracts on L1 | Rollup contracts |

### Category Weights Rationale

- **Privacy (3x)**: Users of privacy protocols have the strongest need for query privacy
- **Bridge (2x)**: Bridge usage reveals cross-chain activity patterns
- **DeFi (1.5x)**: Financial activity is privacy-sensitive
- **NFT/Token (1x)**: Standard weight for general activity

## Current Contract List

See [data/hot-contracts.json](../data/hot-contracts.json) for the machine-readable list.

### Stablecoins

| Contract | Address | Est. Storage Slots |
|----------|---------|-------------------|
| USDC | `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` | ~500K |
| USDT | `0xdAC17F958D2ee523a2206206994597C13D831ec7` | ~500K |
| DAI | `0x6B175474E89094C44Da98b954EescdeCB5BE3d842` | ~300K |
| FRAX | `0x853d955aCEf822Db058eb8505911ED77F175b99e` | ~50K |
| LUSD | `0x5f98805A4E8be255a32880FDeC7F6728C6568bA0` | ~30K |

### Wrapped & Governance Tokens

| Contract | Address | Est. Storage Slots |
|----------|---------|-------------------|
| WETH | `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2` | ~1M |
| WBTC | `0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599` | ~100K |
| stETH | `0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84` | ~500K |
| UNI | `0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984` | ~100K |
| AAVE | `0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9` | ~50K |
| LDO | `0x5A98FcBEA516Cf06857215779Fd812CA3beF1B32` | ~50K |

### DEX Protocols

| Contract | Address | Est. Storage Slots |
|----------|---------|-------------------|
| Uniswap V2 Router | `0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D` | ~10K |
| Uniswap V3 Router | `0xE592427A0AEce92De3Edee1F18E0157C05861564` | ~10K |
| Uniswap Universal Router | `0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45` | ~10K |
| Curve 3pool | `0xbEbc44782C7dB0a1A60Cb6fe97d0b483032FF1C7` | ~5K |
| 1inch Router v5 | `0x1111111254EEB25477B68fb85Ed929f73A960582` | ~5K |

### Lending Protocols

| Contract | Address | Est. Storage Slots |
|----------|---------|-------------------|
| Aave V3 Pool | `0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2` | ~200K |
| Aave V2 Pool | `0x7d2768dE32b0b80b7a3454c06BdAc94A69DDc7A9` | ~200K |
| Compound Comptroller | `0x3d9819210A31b4961b30EF54bE2aeD79B9c9Cd3B` | ~50K |
| Spark Pool | `0xC13e21B648A5Ee794902342038FF3aDAB66BE987` | ~100K |

### Privacy Protocols

| Contract | Address | Est. Storage Slots |
|----------|---------|-------------------|
| Tornado Cash 0.1 ETH | `0x910Cbd523D972eb0a6f4cAe4618aD62622b39DbF` | ~10K |
| Tornado Cash 1 ETH | `0xA160cdAB225685dA1d56aa342Ad8841c3b53f291` | ~50K |
| Tornado Cash 10 ETH | `0xD4B88Df4D29F5CedD6857912842cff3b20C8Cfa3` | ~30K |
| Tornado Cash 100 ETH | `0xFD8610d20aA15b7B2E3Be39B396a1bC3516c7144` | ~10K |
| Railgun Smart Wallet | `0xFA7093CDD9EE6932B4eb2c9e1cde7CE00B1FA4b9` | ~20K |
| Aztec L2 Rollup | `0x603bb2c05d474794ea97805e8de69bccfb3bca12` | ~50K |

### Bridges

| Contract | Address | Est. Storage Slots |
|----------|---------|-------------------|
| Arbitrum Bridge | `0x8315177aB297bA92A06054cE80a67Ed4DBd7ed3a` | ~100K |
| Optimism Bridge | `0x99C9fc46f92E8a1c0deC1b1747d010903E884bE1` | ~100K |
| Polygon Bridge | `0x40ec5B33f54e0E8A33A975908C5BA1c14e5BbbDf` | ~50K |
| Base Portal | `0x49048044D57e1C92A77f79988d21Fa8fAF74E97e` | ~100K |
| zkSync Bridge | `0x32400084C286CF3E17e7B677ea9583e60a000324` | ~50K |

### Liquid Staking

| Contract | Address | Est. Storage Slots |
|----------|---------|-------------------|
| Lido stETH | `0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84` | ~500K |
| Rocket Pool rETH | `0xae78736Cd615f374D3085123A210448E74Fc6393` | ~100K |
| Coinbase cbETH | `0xBe9895146f7AF43049ca1c1AE358B0541Ea49704` | ~100K |
| Binance BETH | `0xa2E3356610840701BDf5611a53974510Ae27E2e1` | ~100K |

### Restaking

| Contract | Address | Est. Storage Slots |
|----------|---------|-------------------|
| EigenLayer StrategyManager | `0x858646372CC42E1A627fcE94aa7A7033e7CF075A` | ~50K |
| EigenLayer DelegationManager | `0x39053D51B77DC0d36036Fc1fCc8Cb819df8Ef37A` | ~50K |
| ether.fi eETH | `0x35fA164735182de50811E8e2E824cFb9B6118ac2` | ~100K |

### NFT Marketplaces

| Contract | Address | Est. Storage Slots |
|----------|---------|-------------------|
| Seaport 1.6 | `0x0000000000000068F116a894984e2DB1123eB395` | ~20K |
| Seaport 1.5 | `0x00000000000000ADc04C56Bf30aC9d3c0aAF14dC` | ~20K |
| Blur Marketplace | `0x39da41747a83aeE658334415666f3EF92DD0D541` | ~10K |
| OpenSea Shared | `0x495f947276749Ce646f68AC8c248420045cb7b5e` | ~1M |

## Scoring Algorithm

Contracts are scored using a hybrid approach:

```
score = (gas_score * 0.4 + tx_score * 0.3 + tvl_score * 0.2 + curated_score * 0.1) * category_weight
```

Where:
- `gas_score`: Normalized gas usage from Etherscan/Dune (0-100)
- `tx_score`: Normalized transaction count (0-100)
- `tvl_score`: Normalized TVL from DeFiLlama (0-100)
- `curated_score`: Manual boost for known important contracts (0-100)
- `category_weight`: Multiplier from category table above

## Exclusions

The following contract types are explicitly **excluded** from the hot lane:

| Type | Reason |
|------|--------|
| MEV bots | Ephemeral, not user-facing |
| Phishing contracts | Malicious |
| Spam token contracts | No legitimate use |
| Short-lived contracts | Unstable |
| Exchange hot wallets | Not contracts (EOAs) |

## Update Process

### Weekly Updates

1. **Data Collection** (Automated)
   - Fetch 7-day gas usage from Etherscan API
   - Query Dune Analytics for transaction counts
   - Pull TVL data from DeFiLlama API

2. **Scoring** (Automated)
   - Apply hybrid scoring algorithm
   - Filter exclusions
   - Rank by score

3. **Review** (Manual)
   - Verify top 100 changes
   - Check for new protocols to add
   - Update curated list if needed

4. **Deployment**
   - Update `data/hot-contracts.json`
   - Regenerate lane databases via `lane-builder`
   - Trigger server reload via `/admin/reload`

### Commands

```bash
# Backfill gas data from recent blocks
cargo run --bin lane-backfill --features backfill -- \
    --rpc-url http://localhost:8545 \
    --blocks 100000

# Build hot lane from scored contracts
cargo run --bin lane-builder -- \
    --config config.json \
    --output-dir ./data/lanes
```

## Versioning

The hot contracts list is versioned to track changes:

| Field | Description |
|-------|-------------|
| `version` | Semantic version (e.g., `1.0.0`) |
| `generated_at` | ISO 8601 timestamp |
| `block_number` | Ethereum block at generation time |
| `checksum` | SHA256 of contract list |

### Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2024-12-21 | Initial curated list from Etherscan + DeFiLlama |

## Storage Slot Estimation

Storage slot counts are estimated using:

1. **Balance mappings**: `~1 slot per holder`
2. **Allowance mappings**: `~0.5 slots per holder` (sparse)
3. **Protocol state**: `~1000-10000 slots` (fixed overhead)

Total hot lane entries target: **~1M slots** across all contracts.

## References

- [PROTOCOL.md](./PROTOCOL.md) - Full protocol specification
- [Etherscan Gas Tracker](https://etherscan.io/gastracker)
- [Dune: Top Contracts](https://dune.com/nbphuoc/eth-top-contracts)
- [DeFiLlama](https://defillama.com/chain/ethereum)
- [Seaport Deployments](https://github.com/ProjectOpenSea/seaport#deployments)
