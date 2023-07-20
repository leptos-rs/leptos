use js_framework_benchmark_leptos::*;
use leptos::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn add_item() {
    let document = leptos::document();
    let test_wrapper = document.create_element("section").unwrap();
    let _ = document.body().unwrap().append_child(&test_wrapper);

    // start by rendering our counter and mounting it to the DOM
    mount_to(test_wrapper.clone().unchecked_into(), || view! { <App/> });

    let table = test_wrapper
        .query_selector("table")
        .unwrap()
        .unwrap()
        .unchecked_into::<web_sys::HtmlTableElement>();

    let create = test_wrapper
        .query_selector("button#runlots")
        .unwrap()
        .unwrap()
        .unchecked_into::<web_sys::HtmlButtonElement>();

    let add = test_wrapper
        .query_selector("button#add")
        .unwrap()
        .unwrap()
        .unchecked_into::<web_sys::HtmlButtonElement>();

    let clear = test_wrapper
        .query_selector("button#clear")
        .unwrap()
        .unwrap()
        .unchecked_into::<web_sys::HtmlButtonElement>();

    // now let's click the `clear` button
    clear.click();

    // now check that table is empty
    assert_eq!(table.rows().length(), 0);

    create.click();

    assert_eq!(table.rows().length(), 10000);
    add.click();

    assert_eq!(table.rows().length(), 11000);

    clear.click();

    assert_eq!(table.rows().length(), 0)
}
