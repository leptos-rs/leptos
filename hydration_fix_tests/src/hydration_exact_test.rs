// Test that replicates the exact problematic case from the hydration module
use leptos::prelude::*;

// #[test]
// fn test_hydration_exact_case() {
//     let root = "http://localhost:3000";
//     let pkg_path = "pkg";
//     let js_file_name = "app";
//     let wasm_file_name = "app";
//     let script = "import";
//     let islands_router = "";
//     let nonce = None::<String>;

//     let _view = view! {
//         <link rel="modulepreload" href=format!("{root}/{pkg_path}/{js_file_name}.js") crossorigin=nonce.clone()/>
//         <link rel="preload" href=format!("{root}/{pkg_path}/{wasm_file_name}.wasm") crossorigin=None::<String> />
//         <script type="module" nonce=nonce>
//             {format!("{script}({root:?}, {pkg_path:?}, {js_file_name:?}, {wasm_file_name:?});{islands_router}")}
//         </script>
//     };
// }

// #[test]
// fn test_hydration_exact_case_simplified() {
//     let root = "http://localhost:3000";
//     let pkg_path = "pkg";
//     let js_file_name = "app";
//     let wasm_file_name = "app";
//     let nonce = None::<String>;

//     let _view = view! {
//         <link rel="modulepreload" href=format!("{root}/{pkg_path}/{js_file_name}.js") crossorigin=nonce.clone()/>
//         <link rel="preload" href=format!("{root}/{pkg_path}/{wasm_file_name}.wasm") crossorigin=None::<String> />
//     };
// }

#[test]
fn test_hydration_exact_case_single_link() {
    let root = "http://localhost:3000";
    let pkg_path = "pkg";
    let js_file_name = "app";
    let nonce = None::<String>;

    let _view = view! {
        <link rel="modulepreload" href=format!("{root}/{pkg_path}/{js_file_name}.js") crossorigin=nonce.clone()/>
    };
}
