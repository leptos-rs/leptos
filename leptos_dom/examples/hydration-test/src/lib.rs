#![allow(warnings)]

use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    let pending_thing = create_resource(
        || false,
        |_| async {
            if cfg!(feature = "ssr") {
                let (tx, rx) = futures::channel::oneshot::channel();
                spawn_local(async {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    tx.send(());
                });
                rx.await;
            } else {
            }
            true
        },
    );

    view! {
      <div>
        <div>
          "This is some text"
        </div>
        // <Suspense fallback=move || view! { <p>"Loading..."</p> }>
          {move || pending_thing.read().map(|n| view! { <ComponentA/> })}
        // </Suspense>
      </div>
    }
}

#[component]
pub fn ComponentA() -> impl IntoView {
    let (value, set_value) = create_signal("Hello?".to_string());
    let (counter, set_counter) = create_signal(0);

    // Test to make sure hydration isn't broken by
    // something like this
    //let _ = [div()].into_view();

    div()
        .id("the-div")
        .child(
            input()
                .attr("type", "text")
                .prop("value", (value))
                .on(ev::input, move |e| set_value(event_target_value(&e))),
        )
        .child(input().attr("type", "text").prop("value", value))
        .child(p().child("Value: ").child(value))
        .into_view()
}

#[cfg(feature = "hydrate")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(feature = "hydrate")]
#[wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();

    gloo::console::debug!("starting WASM");

    leptos::mount_to_body(move || {
        view! { <App/> }
    });
}
