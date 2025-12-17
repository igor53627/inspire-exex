//! PIR Query Server CLI

use anyhow::Result;
use memmap2::Mmap;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

mod responder;
mod server;

use responder::Responder;
use server::{create_router, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: pir-server-b <database.bin> [--port PORT]");
        std::process::exit(1);
    }
    
    let db_path = &args[1];
    let port: u16 = args
        .iter()
        .position(|a| a == "--port")
        .and_then(|i| args.get(i + 1))
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    
    tracing::info!("Loading database from {}", db_path);
    
    let file = std::fs::File::open(db_path)?;
    let mmap = Arc::new(unsafe { Mmap::map(&file)? });
    
    let responder = Responder::new(mmap);
    tracing::info!("Loaded {} entries", responder.entry_count());
    
    let state = Arc::new(AppState { responder });
    let app = create_router(state);
    
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Starting server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
