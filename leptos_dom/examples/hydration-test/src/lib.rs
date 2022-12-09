#![allow(warnings)]

use leptos::*;

#[component]
pub fn App(cx: Scope) -> View {
  view! { cx,
    <>
      <div>
        "This is some text"
      </div>
      <ComponentA>
        <div>"Hello!"</div>
      </ComponentA>
    </>
  }
}

#[component]
pub fn ComponentA(cx: Scope, children: Box<dyn Fn() -> Vec<View>>) -> View {
  let (value, set_value) = create_signal(cx, "Hello?".to_string());
  let (counter, set_counter) = create_signal(cx, 0);

  // Test to make sure hydration isn't broken by
  // something like this
  let _ = [div(cx)].into_view(cx);

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
    .child(children)
    .into_view(cx)
}

#[cfg(feature = "hydrate")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(feature = "hydrate")]
#[wasm_bindgen(start)]
pub fn start() {
  console_error_panic_hook::set_once();

  leptos::mount_to_body(move |cx| {
    view! { cx, <App/> }
  });
}
