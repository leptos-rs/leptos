//! Same shape as `show_memo_in_transition`, on the futures executor. Its own
//! binary because the executor is set once per process.

#![cfg(feature = "ssr")]

use any_spawner::Executor;
use futures::StreamExt;
use leptos::prelude::*;

/// The boundary sits inside a component, so that with `erase_components`
/// the whole view is an erased `AnyView`, as it is in an application.
#[component]
fn DeleteButton(has_image: Resource<bool>) -> impl IntoView {
    view! {
        <Transition fallback=|| ()>
            <Show when=move || has_image.get().unwrap_or(false)>
                <button>"delete"</button>
            </Show>
        </Transition>
    }
}

/// The same shape on the futures executor, with the resource released only
/// after the stream has been created: the condition's first read happens
/// before the value exists, and the post-wait pass must still see it.
#[test]
fn in_order_show_with_a_gated_resource_on_the_futures_executor() {
    use futures::{FutureExt, channel::oneshot};

    _ = Executor::init_futures_executor();
    let html = Owner::new().with(|| {
        let (release, pending) = oneshot::channel::<()>();
        let pending = pending.shared();
        let has_image = Resource::new(
            || (),
            move |_| {
                let pending = pending.clone();
                async move {
                    assert!(pending.await.is_ok());
                    true
                }
            },
        );
        let rendered = view! { <DeleteButton has_image /> }.to_html_stream_in_order();

        assert!(has_image.get_untracked().is_none());
        assert!(release.send(()).is_ok());
        futures::executor::block_on(rendered.collect::<String>())
    });
    assert!(
        html.contains("<button"),
        "the Show condition never saw the resolved resource; got: {html:?}"
    );
}
