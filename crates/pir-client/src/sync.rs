//! Hint synchronization from DHT/IPFS

use crate::hint_store::HintStore;
use pir_core::{subset::Subset, Hint};
use serde::{Deserialize, Serialize};

/// Manifest from DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HintManifest {
    pub block_number: u64,
    pub merkle_root: [u8; 32],
    pub hint_cids: Vec<String>,
    pub subset_size: usize,
    pub domain_size: u64,
}

/// Delta update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HintDelta {
    pub block_number: u64,
    pub changes: Vec<(usize, Hint)>,
}

/// Sync client for downloading hints from DHT
pub struct SyncClient {
    /// IPFS gateway URL
    pub gateway_url: String,
    /// HTTP client
    client: reqwest::Client,
}

impl SyncClient {
    pub fn new(gateway_url: String) -> Self {
        Self {
            gateway_url,
            client: reqwest::Client::new(),
        }
    }

    /// Download full hint set from manifest
    pub async fn download_hints(&self, manifest: &HintManifest) -> anyhow::Result<HintStore> {
        let mut store = HintStore::new();
        let mut hints = Vec::with_capacity(manifest.hint_cids.len());
        
        for (i, cid) in manifest.hint_cids.iter().enumerate() {
            // Download hint from IPFS
            let url = format!("{}/ipfs/{}", self.gateway_url, cid);
            let response = self.client.get(&url).send().await?;
            let hint_bytes = response.bytes().await?;
            
            let hint: Hint = hint_bytes
                .as_ref()
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid hint size"))?;
            
            // Reconstruct subset from index
            let mut seed = [0u8; 32];
            seed[..8].copy_from_slice(&(i as u64).to_le_bytes());
            let subset = Subset::new(seed, manifest.subset_size, manifest.domain_size);
            
            hints.push((subset, hint));
            
            if i % 10_000 == 0 {
                tracing::info!("Downloaded {}/{} hints", i, manifest.hint_cids.len());
            }
        }
        
        store.add_hints(hints, manifest.block_number);
        
        Ok(store)
    }

    /// Download and apply deltas since a block
    pub async fn sync_deltas(
        &self,
        store: &mut HintStore,
        delta_cids: &[String],
    ) -> anyhow::Result<()> {
        for cid in delta_cids {
            let url = format!("{}/ipfs/{}", self.gateway_url, cid);
            let response = self.client.get(&url).send().await?;
            let delta: HintDelta = response.json().await?;
            
            // Apply delta to store
            for (hint_id, new_value) in delta.changes {
                if let Some(stored) = store.hints.get_mut(hint_id) {
                    stored.hint = new_value;
                }
            }
            
            store.block_number = delta.block_number;
        }
        
        Ok(())
    }
}
