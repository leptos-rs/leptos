use super::*;
use crate::counters_page as ui;
use pretty_assertions::assert_eq;

#[wasm_bindgen_test]
fn should_increase_the_total_count() {
    // Given
    ui::view_counters();
    ui::add_counter();

    // When
    ui::increment_counter(1);
    ui::increment_counter(1);
    ui::increment_counter(1);

    // Then
    assert_eq!(ui::total(), 3);
}
