#!/usr/bin/env python3
"""
Analyze expected EIP-7864 stem counts from a state.bin file.

This reads a PIR2 state.bin file (address, slot, value format) and computes
what the unique stem count would be with EIP-7864 tree embedding.
"""

import sys
import struct
from collections import defaultdict
import hashlib

# EIP-7864 constants
HEADER_STORAGE_OFFSET = 64
CODE_OFFSET = 128
STEM_SUBTREE_WIDTH = 256
MAIN_STORAGE_OFFSET = 256 ** 31  # 2^248

def compute_storage_tree_index(slot_bytes: bytes) -> bytes:
    """Compute tree_index for a storage slot per EIP-7864."""
    slot_int = int.from_bytes(slot_bytes, 'big')
    
    if slot_int < 64:
        # Small slot: account stem at subindex 64-127
        tree_index = bytes(31) + bytes([HEADER_STORAGE_OFFSET + slot_int])
    else:
        # Large slot: MAIN_STORAGE_OFFSET + slot
        # The result is a 32-byte value where:
        # - bytes[0..31] = stem_pos
        # - byte[31] = subindex
        # This is equivalent to adding MAIN_STORAGE_OFFSET (which is 0x01 followed by 31 zeros)
        # to the slot, then treating the result as (stem_pos || subindex)
        
        # Add MAIN_STORAGE_OFFSET to slot (both are 256-bit big-endian)
        offset_bytes = (1).to_bytes(1, 'big') + bytes(31)  # 0x01 followed by 31 zeros
        pos_int = int.from_bytes(offset_bytes, 'big') + slot_int
        # Wrap to 256 bits
        pos_int = pos_int % (2 ** 256)
        tree_index = pos_int.to_bytes(32, 'big')
    
    return tree_index

def compute_stem(address: bytes, tree_index: bytes) -> bytes:
    """Compute stem from address and tree_index."""
    # address32 = 12 zero bytes + 20-byte address
    address32 = bytes(12) + address
    # stem = blake3(address32 || tree_index[:31])[:31]
    # Use hashlib.blake2b as fallback if blake3 not available
    try:
        import blake3
        h = blake3.blake3(address32 + tree_index[:31])
        return h.digest()[:31]
    except ImportError:
        # Fallback: use sha256 for counting (not cryptographically correct but fine for analysis)
        import hashlib
        h = hashlib.sha256(address32 + tree_index[:31])
        return h.digest()[:31]

def main():
    if len(sys.argv) < 2:
        print("Usage: analyze_eip7864_stems.py <state.bin>")
        sys.exit(1)
    
    filename = sys.argv[1]
    
    with open(filename, 'rb') as f:
        # Read header (64 bytes)
        header = f.read(64)
        magic = header[0:4]
        if magic != b'PIR2':
            print(f"Error: Invalid magic {magic!r}, expected b'PIR2'")
            sys.exit(1)
        
        version = struct.unpack('<H', header[4:6])[0]
        entry_size = struct.unpack('<H', header[6:8])[0]
        entry_count = struct.unpack('<Q', header[8:16])[0]
        block_number = struct.unpack('<Q', header[16:24])[0]
        chain_id = struct.unpack('<Q', header[24:32])[0]
        
        print(f"Header: version={version}, entry_size={entry_size}, entries={entry_count}")
        print(f"Block: {block_number}, Chain ID: {chain_id}")
        
        # Track unique addresses and stems
        addresses = set()
        raw_stems = set()  # stem computed from raw slot
        eip7864_stems = set()  # stem computed from tree_index
        
        # Count slots by range
        small_slots = 0  # slots 0-63
        large_slots = 0  # slots >= 64
        
        for i in range(entry_count):
            entry = f.read(entry_size)
            if len(entry) < entry_size:
                print(f"Warning: Truncated at entry {i}")
                break
            
            address = entry[0:20]
            slot = entry[20:52]
            # value = entry[52:84]
            
            addresses.add(address)
            
            # Compute raw stem (current behavior)
            raw_stem = compute_stem(address, slot)  # treating slot as tree_index
            raw_stems.add(raw_stem)
            
            # Compute EIP-7864 tree_index and stem
            tree_index = compute_storage_tree_index(slot)
            eip7864_stem = compute_stem(address, tree_index)
            eip7864_stems.add(eip7864_stem)
            
            # Count slot ranges
            slot_int = int.from_bytes(slot, 'big')
            if slot_int < 64:
                small_slots += 1
            else:
                large_slots += 1
            
            if (i + 1) % 1_000_000 == 0:
                print(f"Processed {i+1:,} entries...")
        
        print()
        print("=== Analysis Results ===")
        print(f"Total entries: {entry_count:,}")
        print(f"Unique addresses: {len(addresses):,}")
        print()
        print("Slot distribution:")
        print(f"  Small slots (0-63): {small_slots:,} ({100*small_slots/entry_count:.1f}%)")
        print(f"  Large slots (>=64): {large_slots:,} ({100*large_slots/entry_count:.1f}%)")
        print()
        print("Stem counts:")
        print(f"  Raw slot stems: {len(raw_stems):,}")
        print(f"  EIP-7864 stems: {len(eip7864_stems):,}")
        print()
        print("Improvement:")
        print(f"  Reduction: {len(raw_stems) - len(eip7864_stems):,} fewer stems")
        print(f"  Factor: {len(raw_stems) / len(eip7864_stems):.1f}x fewer stems")
        print()
        print("Stem index size (stem:31 + offset:8 = 39 bytes per entry):")
        print(f"  Raw:     {len(raw_stems) * 39 / 1_048_576:.1f} MB")
        print(f"  EIP-7864: {len(eip7864_stems) * 39 / 1_048_576:.1f} MB")

if __name__ == '__main__':
    main()
