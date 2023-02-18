#[macro_use]
extern crate tracing;

use leptos::*;
use tracing_subscriber::prelude::*;

fn main() {
    tracing_subscriber::fmt()
        .with_writer(tracing_subscriber_wasm::MakeConsoleWriter::default())
        .without_time()
        .with_max_level(tracing::Level::TRACE)
        .pretty()
        .with_target(false)
        .init();

    mount_to_body(app);
}

#[instrument]
fn app(cx: Scope) -> impl IntoView {
    let (data, set_data) = create_signal(cx, vec![1]);

    let handle_change = move |_| {
        set_data.update(|data| {
            if [1] == data[..] {
                *data = vec![0, 1, 2];
            } else {
                *data = vec![1];
            }
        })
    };

    view! { cx,
      <button on:click=handle_change>"Reverse"</button>

      <For
        each=data
        key=|item| *item
        view=|cx, i| view! { cx, <h3>{i}</h3> }
      />
    }
}
