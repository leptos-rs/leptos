// Test to investigate format! macro in attributes
use leptos::prelude::*;

#[test]
fn test_format_macro_in_attributes() {
    let root = "http://localhost:3000";
    let pkg_path = "pkg";
    let js_file_name = "app";
    
    let _view = view! {
        <link rel="modulepreload" href=format!("{root}/{pkg_path}/{js_file_name}.js") />
    };
}

#[test]
fn test_format_macro_in_attributes_with_crossorigin() {
    let root = "http://localhost:3000";
    let pkg_path = "pkg";
    let js_file_name = "app";
    let nonce = None::<String>;
    
    let _view = view! {
        <link rel="modulepreload" href=format!("{root}/{pkg_path}/{js_file_name}.js") crossorigin=nonce.clone()/>
    };
}

// #[test]
// fn test_multiple_links_with_precomputed_strings() {
//     let root = "http://localhost:3000";
//     let pkg_path = "pkg";
//     let js_file_name = "app";
//     let wasm_file_name = "app";
//     let nonce = None::<String>;
//     
//     // Pre-compute the formatted strings
//     let js_href = format!("{root}/{pkg_path}/{js_file_name}.js");
//     let wasm_href = format!("{root}/{pkg_path}/{wasm_file_name}.wasm");
//     
//     let _view = view! {
//         <link rel="modulepreload" href=js_href crossorigin=nonce.clone()/>
//         <link rel="preload" href=wasm_href crossorigin=None::<String> />
//     };
// }
