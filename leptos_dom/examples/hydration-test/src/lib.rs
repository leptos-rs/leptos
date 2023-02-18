#![allow(warnings)]

use leptos::*;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
  let pending_thing = create_resource(
    cx,
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

  view! { cx,
    <div>
      <div>
        "This is some text"
      </div>
      // <Suspense fallback=move || view! { cx, <p>"Loading..."</p> }>
        {move || pending_thing.read().map(|n| view! { cx, <ComponentA/> })}
      // </Suspense>
    </div>
  }
}

#[component]
pub fn ComponentA(cx: Scope) -> impl IntoView {
  let (value, set_value) = create_signal(cx, "Hello?".to_string());
  let (counter, set_counter) = create_signal(cx, 0);

  // Test to make sure hydration isn't broken by
  // something like this
  //let _ = [div(cx)].into_view(cx);

  div(cx)
    .id("the-div")
    .child(
      input(cx)
        .attr("type", "text")
        .prop("value", (cx, value))
        .on(ev::input, move |e| set_value(event_target_value(&e))),
    )
    .child(input(cx).attr("type", "text").prop("value", value))
    .child(p(cx).child("Value: ").child(value))
    .into_view(cx)
}

#[cfg(feature = "hydrate")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(feature = "hydrate")]
#[wasm_bindgen]
pub fn hydrate() {
  console_error_panic_hook::set_once();

  gloo::console::debug!("starting WASM");

  leptos::mount_to_body(move |cx| {
    view! { cx, <App/> }
  });
}
