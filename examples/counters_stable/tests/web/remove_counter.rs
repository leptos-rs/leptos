use super::*;
use crate::counters_page as ui;
use pretty_assertions::assert_eq;

#[wasm_bindgen_test]
fn should_decrement_the_number_of_counters() {
    // Given
    ui::view_counters();
    ui::add_counter();
    ui::add_counter();
    ui::add_counter();

    // When
    ui::remove_counter(2);

    // Then
    assert_eq!(ui::counters(), 2);
}
