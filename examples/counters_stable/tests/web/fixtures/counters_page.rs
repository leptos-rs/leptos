use counters_stable::Counters;
use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement};

// Actions

pub fn add_1k_counters() {
    find_by_text("Add 1000 Counters").click();
}

pub fn add_counter() {
    find_by_text("Add Counter").click();
}

pub fn clear_counters() {
    find_by_text("Clear Counters").click();
}

pub fn decrement_counter(index: u32) {
    counter_html_element(index, "decrement_count").click();
}

pub fn increment_counter(index: u32) {
    counter_html_element(index, "increment_count").click();
}

pub fn remove_counter(index: u32) {
    counter_html_element(index, "remove_counter").click();
}

pub fn view_counters() {
    remove_existing_counters();
    mount_to_body(|cx| view! { cx,  <Counters/> });
}

// Results

pub fn counters() -> i32 {
    data_test_id("counters").parse::<i32>().unwrap()
}

pub fn title() -> String {
    leptos::document().title()
}

pub fn total() -> i32 {
    data_test_id("total").parse::<i32>().unwrap()
}

// Internal

fn counter_element(index: u32, text: &str) -> Element {
    let selector =
        format!("li:nth-child({}) [data-testid=\"{}\"]", index, text);
    leptos::document()
        .query_selector(&selector)
        .unwrap()
        .unwrap()
}

fn counter_html_element(index: u32, text: &str) -> HtmlElement {
    counter_element(index, text)
        .dyn_into::<HtmlElement>()
        .unwrap()
}

fn data_test_id(id: &str) -> String {
    let selector = format!("[data-testid=\"{}\"]", id);
    leptos::document()
        .query_selector(&selector)
        .unwrap()
        .expect("counters not found")
        .text_content()
        .unwrap()
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

fn remove_existing_counters() {
    if let Some(counter) =
        leptos::document().query_selector("body div").unwrap()
    {
        counter.remove();
    }
}
