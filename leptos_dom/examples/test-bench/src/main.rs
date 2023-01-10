#![allow(warnings)]

#[macro_use]
extern crate tracing;

mod utils;

use leptos::*;
use tracing::field::debug;
use tracing_subscriber::util::SubscriberInitExt;

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

fn view_fn(cx: Scope) -> impl IntoView {
  let view = view! { cx,
   <For
     each=|| vec![0, 1, 2, 3, 4, 5, 6, 7]
     key=|i| *i
     view=|i| view! { cx, {i} }
     />
  }
  .into_view(cx);

  let (a, set_a) = create_signal(cx, view.clone());
  let (b, set_b) = create_signal(cx, view);

  let (is_a, set_is_a) = create_signal(cx, true);

  let handle_toggle = move |_| {
    trace!("toggling");
    if is_a() {
      set_b(a());

      set_is_a(false);
    } else {
      set_a(a());

      set_is_a(true);
    }
  };

  let a_tag = view! { cx, <svg::a/> };

  view! { cx,
    <>
      <div>
        <button on:click=handle_toggle>"Toggle"</button>
      </div>
      <svg>{a_tag}</svg>
      <Example/>
      <A child=Signal::from(a) />
      <A child=Signal::from(b) />
    </>
  }
}

#[component]
fn A(cx: Scope, child: Signal<View>) -> impl IntoView {
  move || child()
}

#[component]
fn Example(cx: Scope) -> impl IntoView {
  trace!("rendering <Example/>");

  let (value, set_value) = create_signal(cx, 10);

  let memo = create_memo(cx, move |_| value() * 2);
  let derived = Signal::derive(cx, move || value() * 3);

  create_effect(cx, move |_| {
    trace!("logging value of derived..., {}", derived.get());
  });

  set_timeout(
    move || set_value.update(|v| *v += 1),
    std::time::Duration::from_millis(50),
  );

  view! { cx,
    <h1>"Example"</h1>
    <button on:click=move |_| set_value.update(|value| *value += 1)>
      "Click me"
    </button>
  }
}
