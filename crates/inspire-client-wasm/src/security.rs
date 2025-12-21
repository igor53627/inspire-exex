//! Security utilities for WASM PIR client
//!
//! Provides secure memory handling for cryptographic secrets.

use wasm_bindgen::prelude::*;
use zeroize::Zeroize;

use crate::error::PirError;
use inspire_pir::rlwe::RlweSecretKey;

/// Wrapper around RlweSecretKey that zeroizes on drop
pub struct SecureSecretKey {
    inner: RlweSecretKey,
}

impl SecureSecretKey {
    pub fn new(key: RlweSecretKey) -> Self {
        Self { inner: key }
    }

    pub fn as_ref(&self) -> &RlweSecretKey {
        &self.inner
    }
}

impl Drop for SecureSecretKey {
    fn drop(&mut self) {
        unsafe {
            let poly = &mut self.inner.poly;
            let coeffs_ptr = poly as *mut _ as *mut u8;
            let coeffs_len = std::mem::size_of_val(poly);
            let slice = std::slice::from_raw_parts_mut(coeffs_ptr, coeffs_len);
            slice.zeroize();
        }
    }
}

/// Check if WebCrypto CSPRNG is available
///
/// The getrandom crate with "js" feature uses crypto.getRandomValues.
/// This function provides an early check to fail fast with a clear error.
pub fn check_webcrypto_available() -> Result<(), JsValue> {
    let mut test_bytes = [0u8; 8];
    
    if getrandom::getrandom(&mut test_bytes).is_err() {
        return Err(PirError::CryptoUnavailable.into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_webcrypto_available() {
        assert!(check_webcrypto_available().is_ok());
    }
}
