//! Two-lane PIR client implementation

use reqwest::Client;
use serde::{Deserialize, Serialize};

use inspire_core::{Address, Lane, LaneRouter, StorageKey, StorageValue};

use crate::error::{ClientError, Result};

/// Response from CRS endpoint
#[derive(Deserialize)]
pub struct CrsResponse {
    pub crs: String,
    pub lane: Lane,
    pub entry_count: u64,
}

/// Request to query endpoint
#[derive(Serialize)]
struct QueryRequest {
    query: String,
}

/// Response from query endpoint
#[derive(Deserialize)]
pub struct QueryResponse {
    pub response: String,
    pub lane: Lane,
}

/// Two-lane PIR client that routes queries to the appropriate lane
pub struct TwoLaneClient {
    router: LaneRouter,
    http: Client,
    server_url: String,
    hot_crs: Option<String>,
    cold_crs: Option<String>,
}

impl TwoLaneClient {
    /// Create a new client with the given router and server URL
    pub fn new(router: LaneRouter, server_url: String) -> Self {
        Self {
            router,
            http: Client::new(),
            server_url: server_url.trim_end_matches('/').to_string(),
            hot_crs: None,
            cold_crs: None,
        }
    }

    /// Initialize the client by fetching CRS from server
    pub async fn init(&mut self) -> Result<()> {
        let hot_crs = self.fetch_crs(Lane::Hot).await?;
        self.hot_crs = Some(hot_crs.crs);
        
        let cold_crs = self.fetch_crs(Lane::Cold).await?;
        self.cold_crs = Some(cold_crs.crs);
        
        tracing::info!(
            hot_entries = hot_crs.entry_count,
            cold_entries = cold_crs.entry_count,
            "Client initialized with both lanes"
        );
        
        Ok(())
    }

    /// Fetch CRS for a specific lane
    pub async fn fetch_crs(&self, lane: Lane) -> Result<CrsResponse> {
        let url = format!("{}/crs/{}", self.server_url, lane);
        let resp = self.http.get(&url).send().await?;
        
        if !resp.status().is_success() {
            return Err(ClientError::Server {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }
        
        let crs_resp: CrsResponse = resp.json().await?;
        Ok(crs_resp)
    }

    /// Query a storage slot
    pub async fn query(&self, contract: Address, slot: StorageKey) -> Result<StorageValue> {
        let lane = self.router.route(&contract);
        
        tracing::debug!(
            contract = hex::encode(contract),
            lane = %lane,
            "Routing query"
        );

        let _query_json = self.build_query(lane, &contract, &slot)?;
        
        todo!("Implement actual PIR query - requires inspire-rs integration")
    }

    /// Build a PIR query for the given target
    fn build_query(&self, lane: Lane, _contract: &Address, _slot: &StorageKey) -> Result<String> {
        let _crs = match lane {
            Lane::Hot => self.hot_crs.as_ref().ok_or_else(|| {
                ClientError::LaneNotAvailable("Hot lane CRS not loaded".to_string())
            })?,
            Lane::Cold => self.cold_crs.as_ref().ok_or_else(|| {
                ClientError::LaneNotAvailable("Cold lane CRS not loaded".to_string())
            })?,
        };

        todo!("Build PIR query using inspire-rs")
    }

    /// Send a query to the server
    async fn send_query(&self, lane: Lane, query_json: &str) -> Result<QueryResponse> {
        let url = format!("{}/query/{}", self.server_url, lane);
        
        let resp = self.http
            .post(&url)
            .json(&QueryRequest {
                query: query_json.to_string(),
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ClientError::Server {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let query_resp: QueryResponse = resp.json().await?;
        Ok(query_resp)
    }

    /// Get which lane a contract would be routed to
    pub fn get_lane(&self, contract: &Address) -> Lane {
        self.router.route(contract)
    }

    /// Check if a contract is in the hot lane
    pub fn is_hot(&self, contract: &Address) -> bool {
        self.router.is_hot(contract)
    }

    /// Get the number of contracts in the hot lane
    pub fn hot_contract_count(&self) -> usize {
        self.router.hot_contract_count()
    }
}

/// Builder for TwoLaneClient
pub struct ClientBuilder {
    server_url: String,
    manifest_path: Option<std::path::PathBuf>,
}

impl ClientBuilder {
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
            manifest_path: None,
        }
    }

    pub fn manifest(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.manifest_path = Some(path.into());
        self
    }

    pub fn build(self) -> Result<TwoLaneClient> {
        let manifest = if let Some(path) = self.manifest_path {
            inspire_core::HotLaneManifest::load(&path)?
        } else {
            inspire_core::HotLaneManifest::new(0)
        };
        
        let router = LaneRouter::new(manifest);
        Ok(TwoLaneClient::new(router, self.server_url))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inspire_core::HotLaneManifest;

    fn create_test_manifest() -> HotLaneManifest {
        let mut manifest = HotLaneManifest::new(1000);
        manifest.add_contract([0x11u8; 20], "Test1".into(), 100, "token".into());
        manifest.add_contract([0x22u8; 20], "Test2".into(), 200, "defi".into());
        manifest
    }

    #[test]
    fn test_client_routing() {
        let router = LaneRouter::new(create_test_manifest());
        let client = TwoLaneClient::new(router, "http://localhost:3000".into());
        
        assert!(client.is_hot(&[0x11u8; 20]));
        assert!(client.is_hot(&[0x22u8; 20]));
        assert!(!client.is_hot(&[0x33u8; 20]));
        
        assert_eq!(client.get_lane(&[0x11u8; 20]), Lane::Hot);
        assert_eq!(client.get_lane(&[0x33u8; 20]), Lane::Cold);
    }

    #[test]
    fn test_hot_contract_count() {
        let router = LaneRouter::new(create_test_manifest());
        let client = TwoLaneClient::new(router, "http://localhost:3000".into());
        
        assert_eq!(client.hot_contract_count(), 2);
    }
}
