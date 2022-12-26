use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);
use leptos::*;
use web_sys::HtmlElement;
use counter::*;

#[wasm_bindgen_test]
fn inc() {
    mount_to_body(|cx| view! { cx, <SimpleCounter initial_value=0 step=1/> });

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
