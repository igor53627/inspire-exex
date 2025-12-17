//! inspire-client: Two-lane PIR client
//!
//! Routes queries to hot or cold lane based on contract address,
//! minimizing bandwidth by using the smaller hot lane when possible.

pub mod client;
pub mod error;

pub use client::TwoLaneClient;
pub use error::ClientError;
