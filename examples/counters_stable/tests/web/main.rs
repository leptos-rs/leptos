use wasm_bindgen_test::*;

// Test Suites
pub mod add_1k_counters;
pub mod add_counter;
pub mod clear_counters;
pub mod decrement_counter;
pub mod enter_count;
pub mod increment_counter;
pub mod remove_counter;
pub mod view_counters;

pub mod fixtures;
pub use fixtures::*;

wasm_bindgen_test_configure!(run_in_browser);
