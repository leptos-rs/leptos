// Test to debug attribute processing
use leptos::prelude::*;

// #[test]
// fn test_attribute_with_none_value() {
//     let _view = view! {
//         <div data-test=None />
//     };
// }

// #[test]
// fn test_attribute_with_none_string() {
//     let _view = view! {
//         <div data-test=None::<String> />
//     };
// }

#[test]
fn test_attribute_with_some_value() {
    let _view = view! {
        <div data-test=Some("value") />
    };
}

// #[test]
// fn test_attribute_with_option_none() {
//     let option: Option<String> = None;
//     let _view = view! {
//         <div data-test=option />
//     };
// }

#[test]
fn test_attribute_with_option_some() {
    let option: Option<String> = Some("value".to_string());
    let _view = view! {
        <div data-test=option />
    };
}

#[test]
fn test_attribute_with_none_variable() {
    let none_value: Option<String> = None;
    let _view = view! {
        <div data-test=none_value />
    };
}
