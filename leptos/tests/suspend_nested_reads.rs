//! Resources that are first read inside the view returned by a `Suspend`
//! future, with no inner `<Suspense>`, must still be awaited by the enclosing
//! boundary before its chunk is streamed.

#![cfg(feature = "ssr")]

use any_spawner::Executor;
use futures::StreamExt;
use leptos::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

async fn sleep_ms(ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
}

#[component]
fn Detail(runs: Arc<AtomicUsize>) -> impl IntoView {
    runs.fetch_add(1, Ordering::SeqCst);
    let first = Resource::new(|| (), |_| sleep_ms(20));
    let second = Resource::new(|| (), |_| sleep_ms(40));
    view! {
        <p>{move || first.get().map(|_| "first")}</p>
        <p>{move || second.get().map(|_| "second")}</p>
    }
}

#[component]
fn DetailWithBoundary(runs: Arc<AtomicUsize>) -> impl IntoView {
    runs.fetch_add(1, Ordering::SeqCst);
    let first = Resource::new(|| (), |_| sleep_ms(20));
    let second = Resource::new(|| (), |_| sleep_ms(40));
    view! {
        <Suspense>
            <p>{move || first.get().map(|_| "first")}</p>
            <p>{move || second.get().map(|_| "second")}</p>
        </Suspense>
    }
}

fn app(runs: Arc<AtomicUsize>) -> impl IntoView {
    let gate = Resource::new(|| (), |_| sleep_ms(20));
    view! {
        <Transition fallback=|| "loading">
            {move || {
                let runs = Arc::clone(&runs);
                Suspend::new(async move {
                    gate.await;
                    view! { <Detail runs /> }
                })
            }}
        </Transition>
    }
}

fn assert_rendered(html: &str) {
    assert!(
        html.contains("first") && html.contains("second"),
        "resources read inside the resolved Suspend view were not awaited; \
         got: {html:?}"
    );
}

#[tokio::test]
async fn out_of_order_waits_for_resources_read_in_suspend_output() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let runs = Arc::new(AtomicUsize::new(0));
    let html = app(Arc::clone(&runs))
        .to_html_stream_out_of_order()
        .collect::<String>()
        .await;

    assert_rendered(&html);
    assert_eq!(
        runs.load(Ordering::SeqCst),
        1,
        "Detail body should run once"
    );
}

#[tokio::test]
async fn in_order_waits_for_resources_read_in_suspend_output() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let runs = Arc::new(AtomicUsize::new(0));
    let html = app(Arc::clone(&runs))
        .to_html_stream_in_order()
        .collect::<String>()
        .await;

    assert_rendered(&html);
    assert_eq!(
        runs.load(Ordering::SeqCst),
        1,
        "Detail body should run once"
    );
}

/// The same shape with an inner `<Suspense>` around the reads already worked
/// before the post-resolve wait was added.
#[tokio::test]
async fn out_of_order_with_inner_suspense() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let runs = Arc::new(AtomicUsize::new(0));
    let gate = Resource::new(|| (), |_| sleep_ms(20));
    let runs_in = Arc::clone(&runs);
    let app = view! {
        <Transition fallback=|| "loading">
            {move || {
                let runs = Arc::clone(&runs_in);
                Suspend::new(async move {
                    gate.await;
                    view! { <DetailWithBoundary runs /> }
                })
            }}
        </Transition>
    };
    let html = app.to_html_stream_out_of_order().collect::<String>().await;

    assert_rendered(&html);
    assert_eq!(
        runs.load(Ordering::SeqCst),
        1,
        "Detail body should run once"
    );
}
