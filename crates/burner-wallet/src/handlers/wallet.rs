use std::sync::Arc;

use askama::Template;
use axum::extract::State;

use crate::AppState;

#[derive(Template)]
#[template(path = "wallet.html")]
pub struct WalletTemplate {
    pub pir_server_url: String,
    pub network: String,
}

pub async fn handler(State(state): State<Arc<AppState>>) -> WalletTemplate {
    WalletTemplate {
        pir_server_url: state.pir_server_url.clone(),
        network: state.network.clone(),
    }
}
