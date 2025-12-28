//! inspire-server: Two-lane PIR server
//!
//! Serves PIR queries for both hot and cold lanes, routing based on
//! the lane specified in the request.

pub mod broadcast;
pub mod error;
pub mod metrics;
pub mod routes;
pub mod server;
pub mod state;

pub use broadcast::BucketBroadcast;
pub use error::ServerError;
pub use metrics::init_prometheus_recorder;
pub use routes::{
    create_admin_router, create_public_router, create_router, create_router_with_metrics,
};
pub use server::{ServerBuilder, TwoLaneServer};
pub use state::{
    create_shared_state, DbSnapshot, LaneData, LaneDatabase, LaneStats, ReloadResult, ServerState,
    SharedState,
};
