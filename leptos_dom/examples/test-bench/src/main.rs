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

  // let disposer =
  //   leptos_reactive::create_scope(leptos_reactive::create_runtime(), |cx| {
  //     let view = view_fn(cx).into_view(cx);

  //     let render = view.render_to_string();

  //     println!("{render}");
  // });
}

fn view_fn(cx: Scope) -> impl IntoView {
  let my_in = input(cx).attr("type", "text");
  let val = my_in.value();
}

struct MyComponent;

impl IntoView for MyComponent {
  fn into_view(self, cx: Scope) -> View {
    let component = Component::new("MyComponent", |cx| {
      h2(cx).child(text("MyComponent")).into_view(cx)
    });

    component.into_view(cx)
  }
}
