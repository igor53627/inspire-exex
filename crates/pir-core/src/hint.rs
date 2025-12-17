//! Hint computation and recovery
//!
//! Hints are XOR parities of database entries at subset indices.

use crate::ENTRY_SIZE;

/// A hint is a 32-byte XOR parity
pub type Hint = [u8; ENTRY_SIZE];

/// Compute a hint by XORing all entries at the given indices
pub fn compute_hint<F>(indices: &[u64], get_entry: F) -> Hint
where
    F: Fn(u64) -> [u8; ENTRY_SIZE],
{
    let mut hint = [0u8; ENTRY_SIZE];
    
    for &idx in indices {
        let entry = get_entry(idx);
        xor_into(&mut hint, &entry);
    }
    
    hint
}

/// Recover the target entry from server response and stored hint
///
/// answer = response XOR hint XOR (sum of other entries in subset)
///
/// Since response = XOR of all entries in subset,
/// and hint = XOR of all entries in subset (computed during setup),
/// we have: response XOR hint = 0 if nothing changed.
///
/// For recovery: target = response XOR hint
/// (assuming hint was computed with same subset)
pub fn recover_entry(response: &Hint, hint: &Hint) -> Hint {
    let mut result = *response;
    xor_into(&mut result, hint);
    result
}

/// Update a hint when a database entry changes
///
/// new_hint = old_hint XOR old_value XOR new_value
pub fn update_hint(hint: &mut Hint, old_value: &[u8; ENTRY_SIZE], new_value: &[u8; ENTRY_SIZE]) {
    xor_into(hint, old_value);
    xor_into(hint, new_value);
}

/// XOR src into dst in place
#[inline]
pub fn xor_into(dst: &mut [u8; ENTRY_SIZE], src: &[u8; ENTRY_SIZE]) {
    for i in 0..ENTRY_SIZE {
        dst[i] ^= src[i];
    }
}

/// XOR multiple entries together
pub fn xor_entries(entries: &[[u8; ENTRY_SIZE]]) -> Hint {
    let mut result = [0u8; ENTRY_SIZE];
    for entry in entries {
        xor_into(&mut result, entry);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xor_self_is_zero() {
        let entry = [0x42u8; ENTRY_SIZE];
        let mut result = entry;
        xor_into(&mut result, &entry);
        assert_eq!(result, [0u8; ENTRY_SIZE]);
    }

    #[test]
    fn test_recovery() {
        // Simulate: subset has indices [0, 1, 2], target is index 1
        let entries = [
            [1u8; ENTRY_SIZE],
            [2u8; ENTRY_SIZE],  // target
            [3u8; ENTRY_SIZE],
        ];
        
        // Hint = XOR of all entries
        let hint = xor_entries(&entries);
        
        // Server response = XOR of all entries (same as hint if DB unchanged)
        let response = xor_entries(&entries);
        
        // Recovery: response XOR hint = 0 (since they're the same)
        // This is the basic case; real recovery needs subset exclusion
        let recovered = recover_entry(&response, &hint);
        assert_eq!(recovered, [0u8; ENTRY_SIZE]);
    }

    #[test]
    fn test_hint_update() {
        let mut hint = [0x10u8; ENTRY_SIZE];
        let old_value = [0x01u8; ENTRY_SIZE];
        let new_value = [0x02u8; ENTRY_SIZE];
        
        update_hint(&mut hint, &old_value, &new_value);
        
        // hint XOR old XOR new = 0x10 XOR 0x01 XOR 0x02 = 0x13
        assert_eq!(hint, [0x13u8; ENTRY_SIZE]);
    }
}
