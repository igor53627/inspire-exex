#!/usr/bin/env rust-script
//! Re-sort a PIR2 state.bin from bucket ordering to stem ordering.
//!
//! Usage: rust-script resort-state.rs input.bin output.bin
//!
//! ```cargo
//! [dependencies]
//! blake3 = "1.5"
//! ```

use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};

const HEADER_SIZE: usize = 64;
const ENTRY_SIZE: usize = 84;

fn compute_stem_key(address: &[u8], slot: &[u8]) -> [u8; 32] {
    let mut input = [0u8; 63];
    input[12..32].copy_from_slice(address);
    input[32..63].copy_from_slice(&slot[..31]);
    
    let hash = blake3::hash(&input);
    let mut key = [0u8; 32];
    key[..31].copy_from_slice(&hash.as_bytes()[..31]);
    key[31] = slot[31];
    key
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input.bin> <output.bin>", args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    eprintln!("Reading {}...", input_path);
    let mut file = BufReader::new(File::open(input_path)?);
    
    // Read header
    let mut header = [0u8; HEADER_SIZE];
    file.read_exact(&mut header)?;
    
    // Verify magic
    if &header[0..4] != b"PIR2" {
        eprintln!("Error: Not a PIR2 file");
        std::process::exit(1);
    }
    
    let entry_count = u64::from_le_bytes(header[8..16].try_into().unwrap());
    eprintln!("Entry count: {}", entry_count);
    
    // Read all entries
    struct SortableEntry {
        sort_key: [u8; 32],
        data: [u8; ENTRY_SIZE],
    }
    
    let mut entries = Vec::with_capacity(entry_count as usize);
    let mut buf = [0u8; ENTRY_SIZE];
    
    for i in 0..entry_count {
        file.read_exact(&mut buf)?;
        
        let address = &buf[0..20];
        let slot = &buf[20..52];
        let sort_key = compute_stem_key(address, slot);
        
        let mut data = [0u8; ENTRY_SIZE];
        data.copy_from_slice(&buf);
        
        entries.push(SortableEntry { sort_key, data });
        
        if (i + 1) % 1_000_000 == 0 {
            eprintln!("Read {} entries...", i + 1);
        }
    }
    
    eprintln!("Sorting by stem key...");
    entries.sort_unstable_by(|a, b| a.sort_key.cmp(&b.sort_key));
    
    eprintln!("Writing {}...", output_path);
    let mut out = BufWriter::new(File::create(output_path)?);
    out.write_all(&header)?;
    
    for (i, entry) in entries.iter().enumerate() {
        out.write_all(&entry.data)?;
        if (i + 1) % 1_000_000 == 0 {
            eprintln!("Written {} entries...", i + 1);
        }
    }
    
    out.flush()?;
    eprintln!("Done!");
    
    Ok(())
}
