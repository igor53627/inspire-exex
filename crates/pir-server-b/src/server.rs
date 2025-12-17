//! HTTP server for PIR queries

use crate::responder::Responder;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use pir_core::subset::CompressedQuery;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Server state
pub struct AppState {
    pub responder: Responder,
}

/// Query request
#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub query: CompressedQuery,
}

/// Query response
#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub result: String, // hex-encoded 32 bytes
    pub query_time_ms: f64,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub entry_count: u64,
}

/// Create the router
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/query", post(query_handler))
        .with_state(state)
}

async fn health_handler(
    State(state): State<Arc<AppState>>,
) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        entry_count: state.responder.entry_count(),
    })
}

async fn query_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, StatusCode> {
    let start = std::time::Instant::now();
    
    let result = state.responder.respond(&request.query);
    
    let elapsed = start.elapsed();
    
    Ok(Json(QueryResponse {
        result: hex::encode(result),
        query_time_ms: elapsed.as_secs_f64() * 1000.0,
    }))
}
