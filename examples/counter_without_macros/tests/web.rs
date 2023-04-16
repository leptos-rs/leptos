use counter_without_macros::counter;
use leptos::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::HtmlElement;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn inc() {
    mount_to_body(move |cx| counter(cx, 0, 1));

    click_increment();
    click_increment();

    assert_eq!(see_text(), Some("Value: 2!".to_string()));

    click_decrement();
    click_decrement();
    click_decrement();
    click_decrement();

    assert_eq!(see_text(), Some("Value: -2!".to_string()));

    click_clear();

    assert_eq!(see_text(), Some("Value: 0!".to_string()));
}

fn click_clear() {
    find_by_text("Clear").click();
}

fn click_decrement() {
    find_by_text("-1").click();
}

fn click_increment() {
    find_by_text("+1").click();
}

fn see_text() -> Option<String> {
    find_by_text("Value: ").text_content()
}

fn find_by_text(text: &str) -> HtmlElement {
    let xpath = format!("//*[text()='{}']", text);
    let document = leptos::document();
    document
        .evaluate(&xpath, &document)
        .unwrap()
        .iterate_next()
        .unwrap()
        .unwrap()
        .dyn_into::<HtmlElement>()
        .unwrap()
}
