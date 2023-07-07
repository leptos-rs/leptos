use super::*;
use crate::counters_page as ui;
use pretty_assertions::assert_eq;

#[wasm_bindgen_test]
fn should_reset_the_counts() {
    // Given
    ui::view_counters();
    ui::add_counter();
    ui::add_counter();
    ui::add_counter();

    // When
    ui::clear_counters();

    // Then
    assert_eq!(ui::total(), 0);
    assert_eq!(ui::counters(), 0);
}
