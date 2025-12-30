//! Storage slot computation for ERC-20 token balance lookups
//!
//! For Solidity mappings like `mapping(address => uint256) balances`,
//! the storage slot for a key is computed as:
//!   slot = keccak256(abi.encode(key, mappingSlot))
//!
//! Where:
//! - key is left-padded to 32 bytes
//! - mappingSlot is the slot number of the mapping variable, left-padded to 32 bytes

use tiny_keccak::{Hasher, Keccak};
use wasm_bindgen::prelude::*;

/// Compute the storage slot for an ERC-20 balance lookup
///
/// For mappings like `mapping(address => uint256)`, the slot is:
///   keccak256(abi.encode(address, slot_base))
///
/// # Arguments
/// - `address`: 20-byte Ethereum address
/// - `slot_base`: The base slot number (e.g., 9 for USDC balances)
///
/// # Returns
/// 32-byte storage slot
#[wasm_bindgen]
pub fn compute_balance_slot(address: &[u8], slot_base: u32) -> Result<Vec<u8>, JsValue> {
    if address.len() != 20 {
        return Err(JsValue::from_str("Address must be 20 bytes"));
    }

    // abi.encode pads address to 32 bytes (left-padded with zeros)
    let mut input = [0u8; 64];

    // First 32 bytes: address left-padded to 32 bytes
    input[12..32].copy_from_slice(address);

    // Second 32 bytes: slot_base as uint256 (big-endian, left-padded)
    input[60..64].copy_from_slice(&slot_base.to_be_bytes());

    let mut hasher = Keccak::v256();
    hasher.update(&input);

    let mut slot = [0u8; 32];
    hasher.finalize(&mut slot);

    Ok(slot.to_vec())
}

/// Compute the storage slot for an ERC-20 balance lookup (hex string interface)
///
/// # Arguments
/// - `address_hex`: Address as hex string (with or without 0x prefix)
/// - `slot_base`: The base slot number (e.g., 9 for USDC balances)
///
/// # Returns
/// Storage slot as hex string (without 0x prefix)
#[wasm_bindgen]
pub fn compute_balance_slot_hex(address_hex: &str, slot_base: u32) -> Result<String, JsValue> {
    let address_hex = address_hex.strip_prefix("0x").unwrap_or(address_hex);

    if address_hex.len() != 40 {
        return Err(JsValue::from_str("Address must be 40 hex characters"));
    }

    let address = hex_decode(address_hex)?;
    let slot = compute_balance_slot(&address, slot_base)?;

    Ok(hex_encode(&slot))
}

/// Well-known token contracts and their balance slot numbers
#[wasm_bindgen]
pub struct TokenInfo {
    address: [u8; 20],
    balance_slot: u32,
    decimals: u8,
    symbol: String,
}

#[wasm_bindgen]
impl TokenInfo {
    #[wasm_bindgen(getter)]
    pub fn address_hex(&self) -> String {
        hex_encode(&self.address)
    }

    #[wasm_bindgen(getter)]
    pub fn balance_slot(&self) -> u32 {
        self.balance_slot
    }

    #[wasm_bindgen(getter)]
    pub fn decimals(&self) -> u8 {
        self.decimals
    }

    #[wasm_bindgen(getter)]
    pub fn symbol(&self) -> String {
        self.symbol.clone()
    }
}

/// Get USDC token info for Sepolia testnet
#[wasm_bindgen]
pub fn sepolia_usdc() -> TokenInfo {
    TokenInfo {
        // 0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238
        address: [
            0x1c, 0x7d, 0x4b, 0x19, 0x6c, 0xb0, 0xc7, 0xb0, 0x1d, 0x74, 0x3f, 0xbc, 0x61, 0x16,
            0xa9, 0x02, 0x37, 0x9c, 0x72, 0x38,
        ],
        balance_slot: 9,
        decimals: 6,
        symbol: "USDC".to_string(),
    }
}

/// Get USDC token info for mainnet
#[wasm_bindgen]
pub fn mainnet_usdc() -> TokenInfo {
    TokenInfo {
        // 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48
        address: [
            0xa0, 0xb8, 0x69, 0x91, 0xc6, 0x21, 0x8b, 0x36, 0xc1, 0xd1, 0x9d, 0x4a, 0x2e, 0x9e,
            0xb0, 0xce, 0x36, 0x06, 0xeb, 0x48,
        ],
        balance_slot: 9,
        decimals: 6,
        symbol: "USDC".to_string(),
    }
}

fn hex_decode(s: &str) -> Result<Vec<u8>, JsValue> {
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|_| JsValue::from_str("Invalid hex string"))
        })
        .collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_compute_balance_slot_usdc() {
        // Test case from Dapper Labs blog:
        // Account: 0x467d543e5e4e41aeddf3b6d1997350dd9820a173
        // USDC slot 9
        // Expected: 0x4065d4ec50c2a4fc400b75cca2760227b773c3e315ed2f2a7784cd505065cb07

        let address = hex_decode("467d543e5e4e41aeddf3b6d1997350dd9820a173").expect("valid hex");
        let slot = compute_balance_slot(&address, 9).expect("valid address");
        let slot_hex = hex_encode(&slot);

        assert_eq!(
            slot_hex,
            "4065d4ec50c2a4fc400b75cca2760227b773c3e315ed2f2a7784cd505065cb07"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_compute_balance_slot_hex() {
        let slot = compute_balance_slot_hex("0x467d543e5e4e41aeddf3b6d1997350dd9820a173", 9)
            .expect("valid address");

        assert_eq!(
            slot,
            "4065d4ec50c2a4fc400b75cca2760227b773c3e315ed2f2a7784cd505065cb07"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_compute_balance_slot_no_prefix() {
        let slot = compute_balance_slot_hex("467d543e5e4e41aeddf3b6d1997350dd9820a173", 9)
            .expect("valid address");

        assert_eq!(
            slot,
            "4065d4ec50c2a4fc400b75cca2760227b773c3e315ed2f2a7784cd505065cb07"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_sepolia_usdc_info() {
        let usdc = sepolia_usdc();
        assert_eq!(usdc.symbol(), "USDC");
        assert_eq!(usdc.decimals(), 6);
        assert_eq!(usdc.balance_slot(), 9);
        assert_eq!(
            usdc.address_hex(),
            "1c7d4b196cb0c7b01d743fbc6116a902379c7238"
        );
    }
}
