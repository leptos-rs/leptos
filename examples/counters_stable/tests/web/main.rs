use wasm_bindgen_test::*;

// Test Suites
pub mod add_1k_counters;
pub mod add_counter;
pub mod clear_counters;
pub mod view_counters;

pub mod fixtures;
pub use fixtures::*;

wasm_bindgen_test_configure!(run_in_browser);
