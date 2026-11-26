#![cfg(target_family = "wasm")]

use tachys::{
    html::{
        attribute::{
            Attribute, any_attribute::IntoAnyAttribute, id, name, value,
        },
        element::{option, select, textarea},
        property::prop,
    },
    prelude::{AddAnyAttr, ElementChild, Mountable, Render},
    renderer::dom::queue_microtask,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{HtmlSelectElement, HtmlTextAreaElement};

wasm_bindgen_test_configure!(run_in_browser);

async fn flush_microtasks() {
    let (tx, rx) = futures::channel::oneshot::channel::<()>();
    queue_microtask(move || {
        let _ = tx.send(());
    });
    let _ = rx.await;
}

#[wasm_bindgen_test(async)]
async fn rebuilding_erased_value_property_does_not_write_undefined() {
    let mut state = textarea()
        .add_any_attr(vec![
            id("composer").into_any_attr(),
            prop("value", "hello".to_string()).into_any_attr(),
        ])
        .build();
    let el = state.elements().pop().unwrap();
    let textarea_el: HtmlTextAreaElement = el.dyn_into().unwrap();

    flush_microtasks().await;
    assert_eq!(textarea_el.value(), "hello");

    textarea()
        .add_any_attr(vec![
            id("composer").into_any_attr(),
            prop("value", String::new()).into_any_attr(),
        ])
        .rebuild(&mut state);

    // Reading synchronously before microtasks flush must see "" rather than
    // the WebIDL DOMString coercion of JS `undefined` ("undefined") or the
    // stale previous text.
    assert_eq!(textarea_el.value(), "");

    flush_microtasks().await;
    assert_eq!(textarea_el.value(), "");
}

#[wasm_bindgen_test(async)]
async fn dropping_erased_value_property_clears_to_empty_string_not_undefined() {
    let mut state = textarea()
        .add_any_attr(vec![
            id("composer").into_any_attr(),
            prop("value", "queued-before-rebuild".to_string()).into_any_attr(),
        ])
        .build();
    let el = state.elements().pop().unwrap();
    let textarea_el: HtmlTextAreaElement = el.dyn_into().unwrap();

    // Rebuild while the initial queued `prop:value` write is still pending.
    textarea()
        .add_any_attr(vec![id("composer").into_any_attr()])
        .rebuild(&mut state);

    assert_eq!(textarea_el.value(), "");

    flush_microtasks().await;
    assert_eq!(textarea_el.value(), "");
}

#[wasm_bindgen_test(async)]
async fn rebuilding_erased_select_value_preserves_selection() {
    let build_select = |selected: &str| {
        select()
            .child((
                option().add_any_attr(value("a")).child("A"),
                option().add_any_attr(value("b")).child("B"),
            ))
            .add_any_attr(vec![
                name("choice").into_any_attr(),
                prop("value", selected.to_string()).into_any_attr(),
            ])
    };

    let mut state = build_select("b").build();
    let el = state.elements().pop().unwrap();
    let select_el: HtmlSelectElement = el.dyn_into().unwrap();

    flush_microtasks().await;
    assert_eq!(select_el.value(), "b");

    build_select("a").rebuild(&mut state);
    flush_microtasks().await;
    assert_eq!(select_el.value(), "a");
}

#[wasm_bindgen_test(async)]
async fn rebuilding_optional_string_value_property_to_none_clears_to_empty_string()
 {
    let el = textarea().build().elements().pop().unwrap();
    let textarea_el: HtmlTextAreaElement = el.clone().dyn_into().unwrap();

    let mut attr_state = prop("value", Some("draft".to_string())).build(&el);
    flush_microtasks().await;
    assert_eq!(textarea_el.value(), "draft");

    prop("value", None::<String>).rebuild(&mut attr_state);
    flush_microtasks().await;
    assert_eq!(textarea_el.value(), "");
}
