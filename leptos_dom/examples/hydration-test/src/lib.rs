use leptos::*;

#[component]
pub fn App(cx: Scope) -> View {
  view! {
    cx,
    <>
      "This is some text"
      <ComponentA/>
    </>
  }
}

#[component]
pub fn ComponentA(cx: Scope) -> View {
	let (value, set_value) = create_signal(cx, "".to_string());
	div(cx)
		.child(
      input(cx)
        .attr("type", "text")
        .prop("value", (cx, value))
    )
    .child(
      p(cx)
        .child("Value is: ")
        .child((cx, value))
        .child("!")
    )
    .into_view(cx)
}

cfg_if::cfg_if! {
  if #[cfg(feature = "hydrate")] {
      use wasm_bindgen::prelude::wasm_bindgen;

      #[wasm_bindgen]
      pub fn hydrate() {
          console_error_panic_hook::set_once();
          leptos::hydrate(body().unwrap(), move |cx| {
              view! { cx, <App/> }
          });
      }
  }
}
