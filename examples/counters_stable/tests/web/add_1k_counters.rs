use super::*;
use crate::counters_page as ui;
use pretty_assertions::assert_eq;

#[wasm_bindgen_test]
fn should_increase_the_number_of_counters() {
    // Given
    ui::view_counters();

    // When
    ui::add_1k_counters();
    ui::add_1k_counters();
    ui::add_1k_counters();

    // Then
    assert_eq!(ui::counters(), 3000);
}
