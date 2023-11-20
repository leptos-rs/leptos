use super::*;
use crate::counters_page as ui;
use pretty_assertions::assert_eq;

#[wasm_bindgen_test]
fn should_increase_the_total_count() {
    // Given
    ui::view_counters();
    ui::add_counter();

    // When
    ui::enter_count(1, 5);

    // Then
    assert_eq!(ui::total(), 5);
}

#[wasm_bindgen_test]
fn should_decrease_the_total_count() {
    // Given
    ui::view_counters();
    ui::add_counter();
    ui::add_counter();
    ui::add_counter();

    // When
    ui::enter_count(1, 100);
    ui::enter_count(2, 100);
    ui::enter_count(3, 100);
    ui::enter_count(1, 50);

    // Then
    assert_eq!(ui::total(), 250);
}
