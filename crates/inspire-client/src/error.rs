//! Client error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Server error: {status} - {message}")]
    Server { status: u16, message: String },

    #[error("Lane not available: {0}")]
    LaneNotAvailable(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Core error: {0}")]
    Core(#[from] inspire_core::Error),
}

pub type Result<T> = std::result::Result<T, ClientError>;
