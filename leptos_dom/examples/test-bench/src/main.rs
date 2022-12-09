#![allow(warnings)]

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

  // let disposer =
  //   leptos_reactive::create_scope(leptos_reactive::create_runtime(), |cx| {
  //     let view = view_fn(cx).into_view(cx);

  //     let render = view.render_to_string();

  //     println!("{render}");
  // });
}

fn view_fn(cx: Scope) -> impl IntoView {
  let (tick, set_tick) = create_signal(cx, 0);
  let (count, set_count) = create_signal(cx, 0);
  let (show, set_show) = create_signal(cx, true);
  let (iterable, set_iterable) = create_signal(cx, vec![]);
  let (disabled, set_disabled) = create_signal(cx, false);
  let (apply_default_class_set, set_apply_default_class_set) =
    create_signal(cx, false);

  // wasm_bindgen_futures::spawn_local(async move {
  //   loop {
  //     gloo::timers::future::sleep(std::time::Duration::from_secs(5)).await;

  //     set_tick.update(|t| *t += 1);
  //   }
  // });

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
        *i = vec![0, 1, 2, 3];
      } else {
        *i = vec![0, 1, 2, 3, 4, 5, 6];
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
    span(cx).into_view(cx),
    div(cx)
      .attr("t", || true)
      .child(span(cx).attr("t", true))
      .child(span(cx).attr("t", || true))
      .into_view(cx),
    h1(cx)
      .child(move || text(count().to_string()))
      .into_view(cx),
    button(cx)
      .on(ev::click, move |_| set_tick.update(|t| *t += 1))
      .child(text("Tick"))
      .into_view(cx),
    button(cx)
      .on(ev::click, move |_| set_count.update(|n| *n += 1))
      .child(text("Click me"))
      .into_view(cx),
    button(cx)
      .on(ev::Undelegated(ev::click), move |_| {
        set_count.update(|n| *n += 1)
      })
      .child(text("Click me (undelegated)"))
      .into_view(cx),
    pre(cx)
      .child(Each::new(
        iterable,
        |i| *i,
        move |cx, i| text(format!("{i}, ")),
      ))
      .into_view(cx),
    pre(cx)
      .child(text("0, 1, 2, 3, 4, 5, 6, 7, 8, 9"))
      .into_view(cx),
    input(cx)
      .class("input", true)
      .attr("disabled", move || disabled().then_some(""))
      .into_view(cx),
    MyComponent.into_view(cx),
    h3(cx)
      .child(move || show().then(|| text("Now you see me...")))
      .into_view(cx),
  ]
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
