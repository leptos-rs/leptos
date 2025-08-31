// Simple test to understand view macro behavior
use leptos::prelude::*;

#[test]
fn test_simple_three_elements() {
    let _view = view! {
        <div>"First"</div>
        <span>"Second"</span>
        <p>"Third"</p>
    };
    // This should work
}

#[test]
fn test_simple_five_elements() {
    let _view = view! {
        <div>"1"</div>
        <div>"2"</div>
        <div>"3"</div>
        <div>"4"</div>
        <div>"5"</div>
    };
    // This should work with our fix
}
