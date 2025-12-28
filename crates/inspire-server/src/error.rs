//! Server error types

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

/// Structured error response for API clients
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: &'static str,
}

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Lane not loaded: {0}")]
    LaneNotLoaded(String),

    #[error("Bucket index not loaded")]
    BucketIndexNotLoaded,

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("PIR error: {0}")]
    PirError(String),

    #[error(
        "Config mismatch: {field} - config says {config_value}, but loaded data has {actual_value}"
    )]
    ConfigMismatch {
        field: String,
        config_value: String,
        actual_value: String,
    },

    #[error("PIR params version mismatch for {lane} lane: CRS was generated with v{crs_version}, but server expects v{expected_version}. Regenerate CRS/DB with lane-builder.")]
    ParamsVersionMismatch {
        crs_version: u16,
        expected_version: u16,
        lane: String,
    },

    #[error(
        "CRS metadata not found for {lane} lane at {path}. Regenerate with lane-builder >= 0.1.0."
    )]
    CrsMetadataNotFound { lane: String, path: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl ServerError {
    /// Get the error code for structured responses
    fn code(&self) -> &'static str {
        match self {
            ServerError::LaneNotLoaded(_) => "LANE_NOT_LOADED",
            ServerError::BucketIndexNotLoaded => "BUCKET_INDEX_NOT_LOADED",
            ServerError::InvalidQuery(_) => "INVALID_QUERY",
            ServerError::PirError(_) => "PIR_ERROR",
            ServerError::ConfigMismatch { .. } => "CONFIG_MISMATCH",
            ServerError::ParamsVersionMismatch { .. } => "PARAMS_VERSION_MISMATCH",
            ServerError::CrsMetadataNotFound { .. } => "CRS_METADATA_NOT_FOUND",
            ServerError::Io(_) => "IO_ERROR",
            ServerError::Json(_) => "JSON_ERROR",
            ServerError::Internal(_) => "INTERNAL_ERROR",
        }
    }

    /// Get the HTTP status code for this error
    fn status(&self) -> StatusCode {
        match self {
            ServerError::LaneNotLoaded(_) => StatusCode::SERVICE_UNAVAILABLE,
            ServerError::BucketIndexNotLoaded => StatusCode::SERVICE_UNAVAILABLE,
            ServerError::InvalidQuery(_) => StatusCode::BAD_REQUEST,
            ServerError::PirError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::ConfigMismatch { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::ParamsVersionMismatch { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::CrsMetadataNotFound { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::Json(_) => StatusCode::BAD_REQUEST,
            ServerError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = ErrorResponse {
            error: self.to_string(),
            code: self.code(),
        };

        (status, Json(body)).into_response()
    }
}

pub type Result<T> = std::result::Result<T, ServerError>;
