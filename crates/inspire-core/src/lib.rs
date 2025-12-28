//! inspire-core: Core types and routing logic for Two-Lane InsPIRe PIR
//!
//! This crate defines the foundational types for the two-lane architecture:
//! - Hot Lane: ~1M entries from top ~1000 contracts (~10 KB queries)
//! - Cold Lane: ~2.7B entries for everything else (~500 KB queries)
//!
//! # Privacy & Threat Model
//!
//! The two-lane architecture preserves PIR guarantees **within** each lane.
//!
//! ## Adversary Model
//!
//! - **Server model**: Single-server, honest-but-curious
//! - **Security goal**: Query index confidentiality within each lane (RLWE)
//! - **Non-goals**: Network anonymity, integrity, availability
//!
//! ## What the Server Learns
//!
//! | Information | Server Knowledge |
//! |-------------|------------------|
//! | Query lane (hot/cold) | **YES** - endpoint/size reveals lane |
//! | Target contract | NO - encrypted by PIR |
//! | Target storage slot | NO - encrypted by PIR |
//! | Target index within lane | NO - PIR property |
//! | Query timing, client identity | YES - via network metadata |
//!
//! ## Trade-off
//!
//! This is a deliberate trade-off:
//! - **Privacy cost**: Per query, server learns hot vs cold (~1 bit)
//! - **Bandwidth gain**: 90% reduction in average query size (500KB -> 60KB)
//!
//! ## Public Information
//!
//! The following are intentionally public:
//! - Hot lane manifest (list of contracts in hot lane)
//! - Lane entry counts
//! - CRS (cryptographic reference strings)
//!
//! For full threat model details, see the project README.

mod balance;
pub mod bucket_index;
mod config;
mod error;
mod indexing;
mod lane;
mod manifest;
mod params;
mod routing;
pub mod state_format;

pub use balance::{BalanceDbMetadata, BalanceRecord, BALANCE_RECORD_SIZE};
pub use config::{TwoLaneConfig, PROTOCOL_VERSION};
pub use error::Error;
pub use indexing::{cold_index, hot_index, slot_to_offset};
pub use lane::Lane;
pub use manifest::{HotContract, HotLaneManifest};
pub use params::{CrsMetadata, ParamsVersionError, PirParams, PIR_PARAMS, PIR_PARAMS_VERSION};
pub use routing::{LaneRouter, QueryTarget, RoutedQuery};
pub use state_format::{
    StateFormatError, StateHeader, StorageEntry, STATE_ENTRY_SIZE, STATE_HEADER_SIZE, STATE_MAGIC,
};

pub type Result<T> = std::result::Result<T, Error>;

/// 20-byte Ethereum address
pub type Address = [u8; 20];

/// 32-byte storage slot key
pub type StorageKey = [u8; 32];

/// 32-byte storage value (entry size)
pub type StorageValue = [u8; 32];

/// Constants for the two-lane architecture
pub mod constants {
    /// Entry size in bytes (Ethereum storage slot)
    pub const ENTRY_SIZE: usize = 32;

    /// Target hot lane size (~1M entries)
    pub const HOT_LANE_TARGET_ENTRIES: u64 = 1_000_000;

    /// Approximate number of top contracts in hot lane
    pub const HOT_LANE_CONTRACT_COUNT: usize = 1_000;

    /// Expected hot lane query size in bytes
    pub const HOT_LANE_QUERY_SIZE: usize = 10_000;

    /// Expected cold lane query size in bytes
    pub const COLD_LANE_QUERY_SIZE: usize = 500_000;
}
