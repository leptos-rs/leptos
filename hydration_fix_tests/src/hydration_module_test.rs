// Test that replicates the exact structure from the hydration module
use leptos::prelude::*;

#[test]
fn test_hydration_module_exact_structure() {
    let root = "http://localhost:3000";
    let pkg_path = "pkg";
    let js_file_name = "app";
    let wasm_file_name = "app";
    let script = "import";
    let islands_router = "";
    let nonce = None::<String>;

    let _view = view! {
        <link rel="modulepreload" href="test1.js" />
        <link rel="preload" href="test2.css" />
        <script type="module" src="test.js"></script>
    };
}
