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
  let (tick, set_tick) = create_signal(cx, 0);
  let (count, set_count) = create_signal(cx, 0);
  let (show, set_show) = create_signal(cx, true);
  let (iterable, set_iterable) = create_signal(cx, vec![]);
  let (disabled, set_disabled) = create_signal(cx, false);
  let (apply_default_class_set, set_apply_default_class_set) =
    create_signal(cx, false);

  wasm_bindgen_futures::spawn_local(async move {
    loop {
      gloo::timers::future::sleep(std::time::Duration::from_secs(5)).await;

      set_tick.update(|t| *t += 1);
    }
  });

  create_effect(cx, move |_| {
    tick();

    set_count.update(|c| *c += 1);
  });

  create_effect(cx, move |_| {
    tick();

    set_show.update(|s| *s = !*s);
  });

  create_effect(cx, move |_| {
    tick();

    set_iterable.update(|i| {
      if tick() % 2 == 0 {
        *i = vec![1, 2, 3];
      } else {
        *i = vec![1, 2, 3, 4, 5, 6];
      }
    })
  });

  create_effect(cx, move |_| {
    tick();

    set_disabled.update(|d| *d = !*d);
  });

  create_effect(cx, move |_| {
    tick();

    set_apply_default_class_set.update(|cs| *cs = !*cs);
  });

  [
    h1()
      .dyn_child(move || text(count().to_string()))
      .into_node(cx),
    button()
      .on("click", move |_: web_sys::Event| {
        set_count.update(|n| *n += 1)
      })
      .child(text("Click me"))
      .into_node(cx),
    button()
      .on_delegated("click", move |_: web_sys::Event| {
        set_count.update(|n| *n += 1)
      })
      .child(text("Click me (delegated)"))
      .into_node(cx),
    p()
      .child(EachKey::new(iterable, |i| *i, |i| text(format!("{i}, "))))
      .into_node(cx),
    input()
      .class("input input-primary")
      .dyn_class(move || {
        if apply_default_class_set() {
          Some("a b")
        } else {
          Some("b c")
        }
      })
      .dyn_attr("disabled", move || disabled().then_some(""))
      .into_node(cx),
    MyComponent.into_node(cx),
    h3()
      .dyn_child(move || show().then(|| text("Now you see me...")))
      .into_node(cx),
  ]
}

struct MyComponent;

impl IntoNode for MyComponent {
  fn into_node(self, cx: Scope) -> Node {
    let component = Component::new("MyComponent", |cx| {
      vec![[h2().child(text("MyComponent"))].into_node(cx)]
    });

    component.into_node(cx)
  }
}
