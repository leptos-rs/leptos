pub mod api;
pub mod app;
pub mod consts;
pub mod hljs;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use app::*;
    use consts::LEPTOS_HYDRATED;
    use std::panic;
    panic::set_hook(Box::new(|info| {
        // this custom hook will call out to show the usual error log at
        // the console while also attempt to update the UI to indicate
        // a restart of the application is required to continue.
        console_error_panic_hook::hook(info);
        let window = leptos::prelude::window();
        if !matches!(
            js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str(LEPTOS_HYDRATED)),
            Ok(t) if t == true
        ) {
            let document = leptos::prelude::document();
            let _ = document.query_selector("#reset").map(|el| {
                el.map(|el| {
                    el.set_class_name("panicked");
                })
            });
            let _ = document.query_selector("#notice").map(|el| {
                el.map(|el| {
                    el.set_class_name("panicked");
                })
            });
        }
    }));
    leptos::mount::hydrate_body(App);

    let window = leptos::prelude::window();
    js_sys::Reflect::set(
        &window,
        &wasm_bindgen::JsValue::from_str(LEPTOS_HYDRATED),
        &wasm_bindgen::JsValue::TRUE,
    )
    .expect("error setting hydrated status");
    let event = web_sys::Event::new(LEPTOS_HYDRATED)
        .expect("error creating hydrated event");
    let document = leptos::prelude::document();
    document
        .dispatch_event(&event)
        .expect("error dispatching hydrated event");
    leptos::logging::log!("dispatched hydrated event");
}
