//! inspire-client: PIR client with sparse bucket index
//!
//! Uses bucket index (~150 KB) for O(1) client-side index lookups.
//! No manifest download required - clients compute indices locally.

pub mod bucket_index;
pub mod client;
pub mod error;

pub use bucket_index::{compute_bucket_id, BucketDelta, BucketIndex, BucketRange};
pub use client::TwoLaneClient;
pub use error::ClientError;
