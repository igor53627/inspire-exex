#!/usr/bin/env python3
"""
Convert ethrex-pir-export output to plinko-compatible format.

ethrex format: [address:20][slot:32][value:32] = 84 bytes per entry
plinko format:
  - database.bin: [value:32] per entry (sorted by index)
  - storage-mapping.bin: [address:20][slot:32][index:4] = 56 bytes per entry
  - account-mapping.bin: [address:20][index:4] = 24 bytes per entry (empty for storage-only)
  - metadata.json: entry count and sizes
"""

import os
import sys
import json
import struct
from pathlib import Path

def convert(input_file: str, output_dir: str):
    input_path = Path(input_file)
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)
    
    file_size = input_path.stat().st_size
    entry_count = file_size // 84
    
    if file_size % 84 != 0:
        print(f"Warning: file size {file_size} not divisible by 84", file=sys.stderr)
    
    print(f"Converting {entry_count} entries from {input_file}")
    
    # Read all entries
    entries = []
    with open(input_path, 'rb') as f:
        for i in range(entry_count):
            data = f.read(84)
            if len(data) < 84:
                break
            address = data[0:20]
            slot = data[20:52]
            value = data[52:84]
            entries.append((address, slot, value, i))
    
    print(f"Read {len(entries)} entries")
    
    # Write database.bin (just values, in order)
    db_path = output_path / "database.bin"
    with open(db_path, 'wb') as f:
        for _, _, value, _ in entries:
            f.write(value)
    print(f"Wrote {db_path} ({len(entries) * 32} bytes)")
    
    # Write storage-mapping.bin (address + slot + index as 4 bytes)
    mapping_path = output_path / "storage-mapping.bin"
    with open(mapping_path, 'wb') as f:
        for address, slot, _, idx in entries:
            f.write(address)  # 20 bytes
            f.write(slot)     # 32 bytes
            f.write(struct.pack('<I', idx))  # 4 bytes little-endian u32
    print(f"Wrote {mapping_path} ({len(entries) * 56} bytes)")
    
    # Write empty account-mapping.bin (no accounts, storage only)
    account_path = output_path / "account-mapping.bin"
    with open(account_path, 'wb') as f:
        pass  # Empty file
    print(f"Wrote {account_path} (0 bytes - storage only mode)")
    
    # Write metadata.json
    metadata = {
        "chain": "sepolia",
        "numStorageSlots": len(entries),
        "numAccounts": 0,
        "entrySize": 32,
        "mappingEntrySize": 56,
        "formatVersion": "1.0.0",
        "source": "ethrex-pir-export"
    }
    metadata_path = output_path / "metadata.json"
    with open(metadata_path, 'w') as f:
        json.dump(metadata, f, indent=2)
    print(f"Wrote {metadata_path}")
    
    print(f"\nConversion complete: {len(entries)} entries")
    print(f"Output directory: {output_path}")

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <input.bin> <output-dir>")
        print(f"  input.bin: ethrex-pir-export output (84-byte records)")
        print(f"  output-dir: plinko-compatible directory")
        sys.exit(1)
    
    convert(sys.argv[1], sys.argv[2])
