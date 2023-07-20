use counter_without_macros::counter;
use leptos::*;
use pretty_assertions::assert_eq;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::HtmlElement;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn should_increment_counter() {
    open_counter();

    click_increment();
    click_increment();

    assert_eq!(see_text(), Some("Value: 2!".to_string()));
}

#[wasm_bindgen_test]
fn should_decrement_counter() {
    open_counter();

    click_decrement();
    click_decrement();

    assert_eq!(see_text(), Some("Value: -2!".to_string()));
}

#[wasm_bindgen_test]
fn should_clear_counter() {
    open_counter();

    click_increment();
    click_increment();

    click_clear();

    assert_eq!(see_text(), Some("Value: 0!".to_string()));
}

fn open_counter() {
    remove_existing_counter();
    mount_to_body(move || counter(0, 1));
}

fn remove_existing_counter() {
    if let Some(counter) =
        leptos::document().query_selector("body div").unwrap()
    {
        counter.remove();
    }
}

fn click_clear() {
    click_text("Clear");
}

fn click_decrement() {
    click_text("-1");
}

fn click_increment() {
    click_text("+1");
}

fn click_text(text: &str) {
    find_by_text(text).click();
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
