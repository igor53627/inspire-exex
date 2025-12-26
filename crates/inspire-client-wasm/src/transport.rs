//! Browser-compatible HTTP transport using fetch API

use gloo_net::http::Request;
use serde::{de::DeserializeOwned, Serialize};

use crate::error::PirError;

pub struct HttpClient {
    base_url: String,
}

impl HttpClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, PirError> {
        let url = format!("{}{}", self.base_url, path);
        
        let response = Request::get(&url)
            .send()
            .await?;
        
        if !response.ok() {
            return Err(PirError::Network(format!(
                "HTTP {} from {}",
                response.status(),
                url
            )));
        }
        
        let json = response.json().await?;
        Ok(json)
    }

    pub async fn post_json<T: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R, PirError> {
        let url = format!("{}{}", self.base_url, path);
        
        let response = Request::post(&url)
            .header("Content-Type", "application/json")
            .json(body)?
            .send()
            .await?;
        
        if !response.ok() {
            return Err(PirError::Network(format!(
                "HTTP {} from {}",
                response.status(),
                url
            )));
        }
        
        let json = response.json().await?;
        Ok(json)
    }

    pub async fn post_json_binary<T: Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<Vec<u8>, PirError> {
        let url = format!("{}{}", self.base_url, path);
        
        let response = Request::post(&url)
            .header("Content-Type", "application/json")
            .json(body)?
            .send()
            .await?;
        
        if !response.ok() {
            return Err(PirError::Network(format!(
                "HTTP {} from {}",
                response.status(),
                url
            )));
        }
        
        let bytes = response.binary().await?;
        Ok(bytes)
    }

    pub async fn get_binary(&self, path: &str) -> Result<Vec<u8>, PirError> {
        let url = format!("{}{}", self.base_url, path);
        
        let response = Request::get(&url)
            .send()
            .await?;
        
        if !response.ok() {
            return Err(PirError::Network(format!(
                "HTTP {} from {}",
                response.status(),
                url
            )));
        }
        
        let bytes = response.binary().await?;
        Ok(bytes)
    }
}
