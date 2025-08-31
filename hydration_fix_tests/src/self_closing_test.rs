// Test to isolate the self-closing elements issue
use leptos::prelude::*;

#[test]
fn test_single_self_closing_element() {
    let _view = view! {
        <link rel="stylesheet" href="style.css" />
    };
}

#[test]
fn test_two_self_closing_elements() {
    let _view = view! {
        <link rel="stylesheet" href="style.css" />
        <link rel="icon" href="icon.ico" />
    };
}

#[test]
fn test_self_closing_with_regular_elements() {
    let _view = view! {
        <link rel="stylesheet" href="style.css" />
        <div>"Content"</div>
        <link rel="icon" href="icon.ico" />
    };
}

#[test]
fn test_script_elements() {
    let _view = view! {
        <script src="script1.js"></script>
        <script src="script2.js"></script>
    };
}

#[test]
fn test_mixed_self_closing() {
    let _view = view! {
        <link rel="stylesheet" href="style.css" />
        <script src="script.js"></script>
        <meta name="viewport" content="width=device-width" />
    };
}
