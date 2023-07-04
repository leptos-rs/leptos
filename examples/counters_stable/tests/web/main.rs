use wasm_bindgen_test::*;

// Test Suites
pub mod view_counters;

pub mod fixtures;
pub use fixtures::*;

wasm_bindgen_test_configure!(run_in_browser);
