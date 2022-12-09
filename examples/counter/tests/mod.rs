use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);
use leptos::*;
use web_sys::HtmlElement;

#[wasm_bindgen_test]
fn inc() {
    mount_to_body(|cx| view! { cx, <SimpleCounter/> });

    let document = leptos::document();
    let div = document.query_selector("div").unwrap().unwrap();
    let dec = div
        .first_child()
        .unwrap()
        .dyn_into::<HtmlElement>()
        .unwrap();
    let text = dec
        .next_sibling()
        .unwrap()
        .dyn_into::<HtmlElement>()
        .unwrap();
    let inc = text
        .next_sibling()
        .unwrap()
        .dyn_into::<HtmlElement>()
        .unwrap();

    inc.click();
    inc.click();

    assert_eq!(text.text_content(), Some("2".to_string()));

    dec.click();
    dec.click();
    dec.click();
    dec.click();

    assert_eq!(text.text_content(), Some("-2".to_string()));
}
