//! PIR Core - Shared primitives for Dummy Subsets PIR
//!
//! This crate provides:
//! - PRF-based subset generation
//! - XOR hint computation and recovery
//! - Seed compression utilities

pub mod prf;
pub mod hint;
pub mod subset;

pub use prf::Prf;
pub use hint::Hint;
pub use subset::Subset;

/// Entry size in bytes (Ethereum storage slot)
pub const ENTRY_SIZE: usize = 32;

/// Default subset size factor (sqrt(N) approximation)
pub const SUBSET_SIZE_FACTOR: u64 = 52_250;
