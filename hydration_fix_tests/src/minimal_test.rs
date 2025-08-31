// Minimal test to isolate the view macro issue
use leptos::prelude::*;

#[test]
fn test_minimal_view() {
    // Test with just basic elements
    let _view = view! {
        <div>"Test"</div>
    };
}

#[test]
fn test_two_elements() {
    let _view = view! {
        <div>"First"</div>
        <div>"Second"</div>
    };
}

#[test]
fn test_three_elements() {
    let _view = view! {
        <div>"First"</div>
        <div>"Second"</div>
        <div>"Third"</div>
    };
}
