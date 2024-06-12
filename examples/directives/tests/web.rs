use gloo_timers::future::sleep;
use std::time::Duration;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);
use directives::App;
use leptos::prelude::*;
use web_sys::HtmlElement;

#[wasm_bindgen_test]
async fn test_directives() {
    mount_to_body(|| view! { <App/> });
    sleep(Duration::ZERO).await;

    let document = leptos::document();
    let paragraphs = document.query_selector_all("p").unwrap();

    assert_eq!(paragraphs.length(), 3);

    for i in 0..paragraphs.length() {
        println!("i: {}", i);
        let p = paragraphs
            .item(i)
            .unwrap()
            .dyn_into::<HtmlElement>()
            .unwrap();
        assert_eq!(
            p.style().get_property_value("background-color").unwrap(),
            ""
        );

        p.click();

        assert_eq!(
            p.style().get_property_value("background-color").unwrap(),
            "yellow"
        );

        p.click();

        assert_eq!(
            p.style().get_property_value("background-color").unwrap(),
            "transparent"
        );
    }

    let a = document
        .query_selector("a")
        .unwrap()
        .unwrap()
        .dyn_into::<HtmlElement>()
        .unwrap();
    assert_eq!(a.inner_html(), "Copy \"Hello World!\" to clipboard");

    a.click();
    assert_eq!(a.inner_html(), "Copied \"Hello World!\"");
}
