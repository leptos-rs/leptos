use super::*;
use crate::counters_page as ui;
use pretty_assertions::assert_eq;

#[wasm_bindgen_test]
fn should_see_the_initial_counts() {
    // When
    ui::view_counters();

    // Then
    assert_eq!(ui::total(), 0);
    assert_eq!(ui::counters(), 0);
}

#[wasm_bindgen_test]
fn should_see_the_title() {
    // When
    ui::view_counters();

    // Then
    assert_eq!(ui::title(), "Counters (Stable)");
}
