//! PIR Seeder CLI
//!
//! Generate and publish hints for Dummy Subsets PIR

use anyhow::Result;
use tracing_subscriber::EnvFilter;

mod generator;
mod publisher;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: pir-seeder <database.bin> [--publish]");
        std::process::exit(1);
    }
    
    let db_path = &args[1];
    let should_publish = args.iter().any(|a| a == "--publish");
    
    tracing::info!("Loading database from {}", db_path);
    
    let config = generator::GeneratorConfig::ethereum_mainnet();
    tracing::info!(
        "Generating {} hints (subset_size={}, domain={})",
        config.num_hints,
        config.subset_size,
        config.domain_size
    );
    
    let num_threads = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4);
    
    let hints = generator::generate_hints_parallel(db_path, &config, num_threads)?;
    
    tracing::info!("Generated {} hints", hints.len());
    
    if should_publish {
        let manifest = publisher::publish_to_ipfs(
            &hints,
            0, // TODO: Get actual block number
            "http://localhost:5001",
        ).await?;
        
        tracing::info!("Manifest merkle_root: {:?}", hex::encode(manifest.merkle_root));
    }
    
    Ok(())
}
