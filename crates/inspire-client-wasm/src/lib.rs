//! inspire-client-wasm: Browser-based PIR client
//!
//! WASM-compatible PIR client for private Ethereum state queries.
//! Uses browser fetch API for HTTP requests.

mod client;
mod error;
mod transport;

pub use client::PirClient;
pub use error::PirError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

macro_rules! console_log {
    ($($t:tt)*) => {
        web_sys::console::log_1(&format_args!($($t)*).to_string().into())
    }
}

pub(crate) use console_log;
