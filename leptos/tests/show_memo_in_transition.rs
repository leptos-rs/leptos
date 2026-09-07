//! A `<Show>` whose condition reads a resource, inside a boundary, must render
//! the resolved branch once the resource has loaded: the memo the condition
//! runs through has to see the resolved value on the post-wait pass.

#![cfg(feature = "ssr")]

use any_spawner::Executor;
use futures::StreamExt;
use leptos::prelude::*;

async fn sleep_ms(ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
}

fn app() -> impl IntoView {
    let has_image = Resource::new(
        || (),
        |_| async {
            sleep_ms(20).await;
            true
        },
    );
    view! {
        <Transition fallback=|| ()>
            <Show when=move || has_image.get().unwrap_or(false)>
                <button>"delete"</button>
            </Show>
        </Transition>
    }
}

#[tokio::test]
async fn in_order_show_renders_the_resolved_branch() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let html = app().to_html_stream_in_order().collect::<String>().await;
    assert!(
        html.contains("<button"),
        "the Show condition never saw the resolved resource; got: {html:?}"
    );
}

#[tokio::test]
async fn out_of_order_show_renders_the_resolved_branch() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let html = app().to_html_stream_out_of_order().collect::<String>().await;
    assert!(
        html.contains("<button"),
        "the Show condition never saw the resolved resource; got: {html:?}"
    );
}
