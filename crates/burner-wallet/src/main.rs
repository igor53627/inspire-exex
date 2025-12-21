mod handlers;

use axum::{routing::get, Router};
use std::sync::Arc;
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct AppState {
    pub pir_server_url: String,
    pub network: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "burner_wallet=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pir_server_url =
        std::env::var("PIR_SERVER_URL").unwrap_or_else(|_| "http://localhost:3001".to_string());
    let network = std::env::var("NETWORK").unwrap_or_else(|_| "sepolia".to_string());

    let state = Arc::new(AppState {
        pir_server_url,
        network,
    });

    let app = Router::new()
        .route("/", get(handlers::wallet::handler))
        .route("/health", get(|| async { "OK" }))
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/pkg", ServeDir::new("static/pkg"))
        .nest_service("/pir-pkg", ServeDir::new("static/pir-pkg"))
        .nest_service("/helios", ServeDir::new("static/helios"))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Burner wallet listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
