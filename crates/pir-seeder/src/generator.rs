//! Hint generation from database

use pir_core::{hint, prf::Seed, subset::Subset, Hint, ENTRY_SIZE};
use std::path::Path;

/// Configuration for hint generation
pub struct GeneratorConfig {
    /// Number of hints to generate
    pub num_hints: usize,
    /// Subset size (sqrt(N))
    pub subset_size: usize,
    /// Total database entries
    pub domain_size: u64,
}

impl GeneratorConfig {
    /// Default config for Ethereum mainnet state
    pub fn ethereum_mainnet() -> Self {
        Self {
            num_hints: 6_700_000,      // ~128 * sqrt(N)
            subset_size: 52_250,        // sqrt(2.73B)
            domain_size: 2_730_000_000, // accounts + storage
        }
    }
}

/// Generate all hints from a database file
pub fn generate_hints<P: AsRef<Path>>(
    db_path: P,
    config: &GeneratorConfig,
) -> anyhow::Result<Vec<(Subset, Hint)>> {
    // Memory-map the database for efficient access
    let file = std::fs::File::open(db_path)?;
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    
    let get_entry = |idx: u64| -> [u8; ENTRY_SIZE] {
        let offset = (idx as usize) * ENTRY_SIZE;
        if offset + ENTRY_SIZE <= mmap.len() {
            mmap[offset..offset + ENTRY_SIZE].try_into().unwrap()
        } else {
            [0u8; ENTRY_SIZE] // Padding for out-of-bounds
        }
    };

    let mut hints = Vec::with_capacity(config.num_hints);
    
    for i in 0..config.num_hints {
        // Generate deterministic seed for this hint
        let mut seed: Seed = [0u8; 32];
        seed[..8].copy_from_slice(&(i as u64).to_le_bytes());
        
        let subset = Subset::new(seed, config.subset_size, config.domain_size);
        let indices = subset.expand();
        
        let hint_value = hint::compute_hint(&indices, &get_entry);
        hints.push((subset, hint_value));
        
        if i % 100_000 == 0 {
            tracing::info!("Generated {}/{} hints", i, config.num_hints);
        }
    }
    
    Ok(hints)
}

/// Generate hints in parallel (faster for large databases)
pub fn generate_hints_parallel<P: AsRef<Path>>(
    db_path: P,
    config: &GeneratorConfig,
    num_threads: usize,
) -> anyhow::Result<Vec<(Subset, Hint)>> {
    use std::sync::Arc;
    
    let file = std::fs::File::open(db_path)?;
    let mmap = Arc::new(unsafe { memmap2::Mmap::map(&file)? });
    
    let hints_per_thread = config.num_hints / num_threads;
    let mut handles = Vec::new();
    
    for t in 0..num_threads {
        let mmap = Arc::clone(&mmap);
        let config_subset_size = config.subset_size;
        let config_domain_size = config.domain_size;
        let start = t * hints_per_thread;
        let end = if t == num_threads - 1 {
            config.num_hints
        } else {
            (t + 1) * hints_per_thread
        };
        
        let handle = std::thread::spawn(move || {
            let get_entry = |idx: u64| -> [u8; ENTRY_SIZE] {
                let offset = (idx as usize) * ENTRY_SIZE;
                if offset + ENTRY_SIZE <= mmap.len() {
                    mmap[offset..offset + ENTRY_SIZE].try_into().unwrap()
                } else {
                    [0u8; ENTRY_SIZE]
                }
            };
            
            let mut thread_hints = Vec::with_capacity(end - start);
            
            for i in start..end {
                let mut seed: Seed = [0u8; 32];
                seed[..8].copy_from_slice(&(i as u64).to_le_bytes());
                
                let subset = Subset::new(seed, config_subset_size, config_domain_size);
                let indices = subset.expand();
                let hint_value = hint::compute_hint(&indices, &get_entry);
                thread_hints.push((subset, hint_value));
            }
            
            thread_hints
        });
        
        handles.push(handle);
    }
    
    let mut all_hints = Vec::with_capacity(config.num_hints);
    for handle in handles {
        all_hints.extend(handle.join().unwrap());
    }
    
    Ok(all_hints)
}
