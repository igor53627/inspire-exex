//! inspire-client-wasm: Browser-based PIR client
//!
//! WASM-compatible PIR client for private Ethereum state queries.
//! Uses browser fetch API for HTTP requests.

mod bucket_index;
mod client;
mod error;
mod security;
mod transport;
mod ubt_index;

pub use bucket_index::{BucketIndex, RangeDeltaInfo};
pub use client::PirClient;
pub use error::PirError;
pub use ubt_index::{compute_stem_js, compute_tree_key_js, get_subindex_js, StemIndex};

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
