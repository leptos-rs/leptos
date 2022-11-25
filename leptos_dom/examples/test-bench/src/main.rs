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

    wasm_bindgen_futures::spawn_local(async move { set_count.update(|c| *c += 1) });

    vec![
        h1().dyn_child(move || text(count().to_string()))
            .into_node(cx),
        ().into_node(cx),
    ]
}
