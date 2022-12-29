#![allow(warnings)]

#[macro_use]
extern crate tracing;

mod utils;

use leptos::*;
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
    if is_a() {
      set_b(a());

      set_is_a(false);
    } else {
      set_a(a());

      set_is_a(true);
    }
  };

  view! { cx,
    <>
      <div>
        <button on:click=handle_toggle>"Toggle"</button>
      </div>
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
  view! { cx,
    <h1>"Example"</h1>
  }
}
