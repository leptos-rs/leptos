// Test to isolate the crossorigin attribute issue
use leptos::prelude::*;

// #[test]
// fn test_crossorigin_none() {
//     let _view = view! {
//         <link rel="stylesheet" href="style.css" crossorigin=None::<String> />
//     };
// }

#[test]
fn test_crossorigin_some() {
    let nonce = Some("test".to_string());
    let _view = view! {
        <link rel="stylesheet" href="style.css" crossorigin=nonce />
    };
}

// #[test]
// fn test_crossorigin_none_without_type() {
//     let _view = view! {
//         <link rel="stylesheet" href="style.css" crossorigin=None />
//     };
// }

// #[test]
// fn test_crossorigin_none_with_format() {
//     let href = format!("test.css");
//     let _view = view! {
//         <link rel="stylesheet" href=href crossorigin=None::<String> />
//     };
// }

// #[test]
// fn test_crossorigin_none_with_format_inline() {
//     let _view = view! {
//         <link rel="stylesheet" href=format!("test.css") crossorigin=None::<String> />
//     };
// }

// #[test]
// fn test_two_links_with_crossorigin_none() {
//     let _view = view! {
//         <link rel="stylesheet" href="style1.css" crossorigin=None::<String> />
//         <link rel="stylesheet" href="style2.css" crossorigin=None::<String> />
//     };
// }
