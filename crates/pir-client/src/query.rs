//! Query construction and execution

use crate::hint_store::HintStore;
use pir_core::{hint::recover_entry, subset::CompressedQuery, Hint, ENTRY_SIZE};
use serde::{Deserialize, Serialize};

/// PIR client for making private queries
pub struct PirClient {
    /// Local hint store
    pub hints: HintStore,
    /// Query server URL
    pub server_url: String,
    /// HTTP client
    client: reqwest::Client,
}

/// Query result
#[derive(Debug)]
pub struct QueryResult {
    pub entry: [u8; ENTRY_SIZE],
    pub query_time_ms: f64,
    pub server_time_ms: f64,
}

/// Server response
#[derive(Debug, Deserialize)]
struct ServerResponse {
    result: String,
    query_time_ms: f64,
}

/// Query request
#[derive(Debug, Serialize)]
struct QueryRequest {
    query: CompressedQuery,
}

impl PirClient {
    /// Create a new PIR client
    pub fn new(hints: HintStore, server_url: String) -> Self {
        Self {
            hints,
            server_url,
            client: reqwest::Client::new(),
        }
    }

    /// Query for a specific database index
    pub async fn query(&self, target_index: u64) -> anyhow::Result<QueryResult> {
        let start = std::time::Instant::now();
        
        // Find a hint containing the target
        let stored_hint = self
            .hints
            .find_hint_for_target(target_index)
            .ok_or_else(|| anyhow::anyhow!("No hint found for target {}", target_index))?;
        
        // Create compressed query
        let query = CompressedQuery::new(&stored_hint.subset);
        
        // Send to server
        let response: ServerResponse = self
            .client
            .post(format!("{}/query", self.server_url))
            .json(&QueryRequest { query })
            .send()
            .await?
            .json()
            .await?;
        
        // Decode server response
        let server_result: Hint = hex::decode(&response.result)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid response length"))?;
        
        // Recover the entry
        let entry = recover_entry(&server_result, &stored_hint.hint);
        
        let elapsed = start.elapsed();
        
        Ok(QueryResult {
            entry,
            query_time_ms: elapsed.as_secs_f64() * 1000.0,
            server_time_ms: response.query_time_ms,
        })
    }

    /// Query multiple indices (batched)
    pub async fn query_batch(&self, indices: &[u64]) -> anyhow::Result<Vec<QueryResult>> {
        let mut results = Vec::with_capacity(indices.len());
        
        // TODO: Parallelize queries
        for &idx in indices {
            results.push(self.query(idx).await?);
        }
        
        Ok(results)
    }
}
