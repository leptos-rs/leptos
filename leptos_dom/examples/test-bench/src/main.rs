#[macro_use]
extern crate tracing;

mod utils;

use leptos_dom::*;
use leptos_reactive::*;
use tracing_subscriber::util::SubscriberInitExt;

#[instrument]
fn main() {
  console_error_panic_hook::set_once();

  tracing_subscriber::fmt()
    .with_max_level(tracing::Level::TRACE)
    .without_time()
    .with_file(true)
    .with_line_number(true)
    .with_target(false)
    .with_writer(utils::MakeConsoleWriter)
    .with_ansi(false)
    .pretty()
    .finish()
    .init();

  mount_to_body(view_fn);
}

fn view_fn(cx: Scope) -> impl IntoNode {
  let (count, set_count) = create_signal(cx, 0);
  let (show, set_show) = create_signal(cx, true);
  let (iterable, set_iterable) =
    create_signal(cx, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);
  let (disabled, set_disabled) = create_signal(cx, false);

  wasm_bindgen_futures::spawn_local(async move {
    loop {
      gloo::timers::future::sleep(std::time::Duration::from_secs(1)).await;

      set_count.update(|c| *c += 1);
    }
  });

  wasm_bindgen_futures::spawn_local(async move {
    loop {
      gloo::timers::future::sleep(std::time::Duration::from_secs(1)).await;

      set_show.update(|s| *s = !*s);
    }
  });

  wasm_bindgen_futures::spawn_local(async move {
    loop {
      gloo::timers::future::sleep(std::time::Duration::from_secs(1)).await;

      set_iterable.update(|i| i.reverse())
    }
  });

  wasm_bindgen_futures::spawn_local(async move {
    loop {
      gloo::timers::future::sleep(std::time::Duration::from_secs(1)).await;

      set_disabled.update(|d| *d = !*d);
    }
  });

  vec![
    h1()
      .dyn_child(move || text(count().to_string()))
      .into_node(cx),
    p()
      .child(EachKey::new(iterable, |i| *i, |i| text(format!("{i}, "))))
      .into_node(cx),
    input()
      .dyn_attr("disabled", move || disabled().then_some(""))
      .into_node(cx),
    h3()
      .dyn_child(move || show().then(|| text("Now you see me...")))
      .into_node(cx),
  ]
}
