#[macro_use]
extern crate tracing;

mod utils;

use leptos_dom::*;
use leptos_reactive::*;
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

fn view_fn(cx: Scope) -> impl IntoNode {
  let (items, set_items) = create_signal(cx, vec![]);
}

struct MyComponent;

impl IntoNode for MyComponent {
  fn into_node(self, cx: Scope) -> Node {
    let mut component = Component::new("MyComponent", |cx| {
      vec![[h2().child(text("MyComponent"))].into_node(cx)]
    });

    component.into_node(cx)
  }
}
