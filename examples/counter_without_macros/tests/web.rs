use counter_without_macros::counter;
use leptos::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::HtmlElement;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn inc() {
    mount_to_body(move |cx| counter(cx, 0, 1));

    let document = leptos::document();
    let div = document.query_selector("div").unwrap().unwrap();
    let clear = div
        .first_child()
        .unwrap()
        .dyn_into::<HtmlElement>()
        .unwrap();
    let dec = clear
        .next_sibling()
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

    assert_eq!(text.text_content(), Some("Value: 2!".to_string()));

    dec.click();
    dec.click();
    dec.click();
    dec.click();

    assert_eq!(text.text_content(), Some("Value: -2!".to_string()));

    clear.click();

    assert_eq!(text.text_content(), Some("Value: 0!".to_string()));
}
