//! PIR Seeder - Hint generator for Dummy Subsets PIR
//!
//! This crate generates hints by:
//! 1. Reading Ethereum state from Reth (via ExEx or direct DB access)
//! 2. Computing XOR parities for each subset
//! 3. Publishing hints to IPFS/DHT

pub mod generator;
pub mod publisher;

#[cfg(feature = "exex")]
pub mod exex;
