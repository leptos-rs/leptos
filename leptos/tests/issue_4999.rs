//! regression test for https://github.com/leptos-rs/leptos/issues/4999

#![cfg(feature = "ssr")]

use futures::StreamExt;
use hydration_context::{SerializedDataId, SharedContext, SsrSharedContext};
use leptos::prelude::*;
use std::sync::Arc;

fn render(out_of_order: bool, transition: bool) -> (String, bool) {
    let sc = Arc::new(SsrSharedContext::new());
    let owner = Owner::new_root(Some(
        sc.clone() as Arc<dyn SharedContext + Send + Sync>
    ));
    let html = owner.with(|| {
        let r = LocalResource::new(|| async { 1u32 });
        let children = move || r.get().map(|v| view! { <p>{v}</p> });
        let fallback = || view! { <p>"FALLBACK"</p> };
        let app = if transition {
            view! { <main><Transition fallback>{children}</Transition></main> }
                .into_any()
        } else {
            view! { <main><Suspense fallback>{children}</Suspense></main> }
                .into_any()
        };
        let stream = if out_of_order {
            app.to_html_stream_out_of_order()
        } else {
            app.to_html_stream_in_order()
        };
        futures::executor::block_on(stream.collect::<String>())
    });
    (html, sc.get_incomplete_chunk(&SerializedDataId::new(0)))
}

#[test]
fn local_resource_keeps_fallback() {
    _ = any_spawner::Executor::init_futures_executor();
    for out_of_order in [true, false] {
        for transition in [false, true] {
            for _ in 0..100 {
                let (html, incomplete) = render(out_of_order, transition);
                assert!(
                    html.contains("FALLBACK"),
                    "ooo={out_of_order} transition={transition}: fallback \
                     missing (incomplete={incomplete}): {html}"
                );
                assert!(incomplete);
            }
        }
    }
}
