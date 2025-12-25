//! Bucket Index Builder
//!
//! Builds a sparse bucket index from state.bin for efficient client-side lookups.
//!
//! Input: state.bin (84-byte records: [address:20][slot:32][value:32])
//! Output: bucket-index.bin (256K buckets × 2 bytes = 512 KB)
//!
//! Usage:
//!   cargo run --bin bucket-index --features bucket-index -- \
//!     --input state.bin \
//!     --output bucket-index.bin

#![cfg(feature = "bucket-index")]

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use tiny_keccak::{Hasher, Keccak};

/// Number of buckets (2^18 = 256K)
const NUM_BUCKETS: usize = 262_144;
/// Bits used for bucket ID
const BUCKET_BITS: usize = 18;
/// Size of each input record (address:20 + slot:32 + value:32)
const RECORD_SIZE: usize = 84;

#[derive(Parser, Debug)]
#[command(name = "bucket-index")]
#[command(about = "Build bucket index from state.bin for sparse PIR lookups")]
struct Args {
    /// Input state.bin file (84-byte records)
    #[arg(long)]
    input: PathBuf,

    /// Output bucket index file
    #[arg(long, default_value = "bucket-index.bin")]
    output: PathBuf,

    /// Also output compressed version (zstd)
    #[arg(long)]
    compress: bool,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    tracing::info!(
        input = %args.input.display(),
        output = %args.output.display(),
        "Building bucket index"
    );

    // Get file size to calculate entry count
    let file_size = std::fs::metadata(&args.input)?.len() as usize;
    let entry_count = file_size / RECORD_SIZE;

    if file_size % RECORD_SIZE != 0 {
        tracing::warn!(
            "File size {} not divisible by record size {}, truncating",
            file_size,
            RECORD_SIZE
        );
    }

    tracing::info!(entry_count, "Processing entries");

    // Initialize bucket counts
    let mut bucket_counts: Vec<u32> = vec![0; NUM_BUCKETS];

    // Read and process entries
    let file = File::open(&args.input)?;
    let mut reader = BufReader::with_capacity(64 * 1024 * 1024, file);

    let pb = ProgressBar::new(entry_count as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40}] {pos}/{len} ({per_sec})")
            .unwrap(),
    );

    let mut record = [0u8; RECORD_SIZE];
    let mut processed = 0u64;

    loop {
        match reader.read_exact(&mut record) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }

        let address = &record[0..20];
        let slot = &record[20..52];

        // Compute keccak256(address || slot)
        let bucket_id = compute_bucket_id(address, slot);
        bucket_counts[bucket_id] += 1;

        processed += 1;
        if processed % 1_000_000 == 0 {
            pb.set_position(processed);
        }
    }

    pb.finish_with_message("Done processing entries");

    // Validate counts
    let total: u64 = bucket_counts.iter().map(|&c| c as u64).sum();
    tracing::info!(
        total_entries = total,
        buckets = NUM_BUCKETS,
        avg_per_bucket = total as f64 / NUM_BUCKETS as f64,
        "Bucket distribution computed"
    );

    // Check for overflow (count > u16::MAX)
    let max_count = *bucket_counts.iter().max().unwrap_or(&0);
    if max_count > u16::MAX as u32 {
        tracing::error!(
            max_count,
            "Bucket count overflow! Max count {} exceeds u16::MAX. Need larger count type.",
            max_count
        );
        return Err(anyhow::anyhow!("Bucket count overflow"));
    }

    // Write bucket index (2 bytes per bucket)
    let output_file = File::create(&args.output)?;
    let mut writer = BufWriter::new(output_file);

    for &count in &bucket_counts {
        writer.write_all(&(count as u16).to_le_bytes())?;
    }
    writer.flush()?;

    let output_size = NUM_BUCKETS * 2;
    tracing::info!(
        output = %args.output.display(),
        size_bytes = output_size,
        size_kb = output_size / 1024,
        "Wrote bucket index"
    );

    // Optionally compress with zstd
    if args.compress {
        let compressed_path = args.output.with_extension("bin.zst");
        let raw_data: Vec<u8> = bucket_counts
            .iter()
            .flat_map(|&c| (c as u16).to_le_bytes())
            .collect();

        let compressed = zstd::encode_all(&raw_data[..], 19)?; // Level 19 = high compression

        std::fs::write(&compressed_path, &compressed)?;

        tracing::info!(
            output = %compressed_path.display(),
            uncompressed = output_size,
            compressed = compressed.len(),
            ratio = format!("{:.1}%", compressed.len() as f64 / output_size as f64 * 100.0),
            "Wrote compressed bucket index"
        );
    }

    // Print statistics
    let non_empty = bucket_counts.iter().filter(|&&c| c > 0).count();
    let min_count = *bucket_counts.iter().filter(|&&c| c > 0).min().unwrap_or(&0);

    tracing::info!(
        non_empty_buckets = non_empty,
        empty_buckets = NUM_BUCKETS - non_empty,
        min_count,
        max_count,
        "Bucket statistics"
    );

    Ok(())
}

/// Compute bucket ID from address and slot using keccak256
fn compute_bucket_id(address: &[u8], slot: &[u8]) -> usize {
    let mut hasher = Keccak::v256();
    hasher.update(address);
    hasher.update(slot);

    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);

    // Take first 18 bits as bucket ID
    // hash[0] = bits 0-7, hash[1] = bits 8-15, hash[2] = bits 16-23
    // We need bits 0-17 (18 bits)
    let bucket_id = ((hash[0] as usize) << 10) | ((hash[1] as usize) << 2) | ((hash[2] as usize) >> 6);

    bucket_id & (NUM_BUCKETS - 1) // Mask to ensure in range
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_id_range() {
        let address = [0u8; 20];
        let slot = [0u8; 32];

        let bucket_id = compute_bucket_id(&address, &slot);
        assert!(bucket_id < NUM_BUCKETS);
    }

    #[test]
    fn test_bucket_id_deterministic() {
        let address = [1u8; 20];
        let slot = [2u8; 32];

        let id1 = compute_bucket_id(&address, &slot);
        let id2 = compute_bucket_id(&address, &slot);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_bucket_id_distribution() {
        // Test that different inputs produce different buckets
        let mut buckets = std::collections::HashSet::new();

        for i in 0..1000u32 {
            let mut address = [0u8; 20];
            address[0..4].copy_from_slice(&i.to_le_bytes());
            let slot = [0u8; 32];

            buckets.insert(compute_bucket_id(&address, &slot));
        }

        // Should have good distribution (most inputs in different buckets)
        assert!(buckets.len() > 900); // At least 90% unique
    }
}
