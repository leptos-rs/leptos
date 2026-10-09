//! Components rendered conditionally on a parent's resource, each creating
//! their own resource, must render once each and never stall the boundary.

#![cfg(feature = "ssr")]

use any_spawner::Executor;
use futures::StreamExt;
use leptos::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Default)]
struct Runs {
    gramps_closure: Arc<AtomicUsize>,
    parent_body: Arc<AtomicUsize>,
    child_body: Arc<AtomicUsize>,
}

impl Runs {
    fn snapshot(&self) -> (usize, usize, usize) {
        (
            self.gramps_closure.load(Ordering::SeqCst),
            self.parent_body.load(Ordering::SeqCst),
            self.child_body.load(Ordering::SeqCst),
        )
    }
}

async fn load() -> String {
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    "boom".to_string()
}

#[component]
fn Gramps(runs: Runs) -> impl IntoView {
    let gramps = Resource::new(|| (), |_| load());
    view! {
        <Suspense>
            {move || {
                runs.gramps_closure.fetch_add(1, Ordering::SeqCst);
                let runs = runs.clone();
                gramps.get().map(|_| view! { <Parent runs /> })
            }}
        </Suspense>
    }
}

#[component]
fn Parent(runs: Runs) -> impl IntoView {
    runs.parent_body.fetch_add(1, Ordering::SeqCst);
    let parent = Resource::new(|| (), |_| load());
    view! {
        {move || {
            let runs = runs.clone();
            parent.get().map(|_| view! { <Child runs /> })
        }}
    }
}

#[component]
fn Child(runs: Runs) -> impl IntoView {
    runs.child_body.fetch_add(1, Ordering::SeqCst);
    let child = Resource::new(|| (), |_| load());
    view! {
        <p id="child">{move || child.get()}</p>
    }
}

async fn assert_renders(html: impl Future<Output = String>, runs: &Runs) {
    let html = tokio::time::timeout(std::time::Duration::from_secs(5), html)
        .await
        .unwrap_or_else(|_| panic!("render hung; runs: {:?}", runs.snapshot()));
    assert!(html.contains("boom"), "got: {html:?}");
    assert_eq!(
        runs.snapshot(),
        (2, 1, 1),
        "(gramps closure, parent bodies, child bodies)"
    );
}

#[tokio::test]
async fn nested_components_out_of_order() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let runs = Runs::default();
    let html = view! { <Gramps runs=runs.clone() /> }
        .to_html_stream_out_of_order()
        .collect::<String>();
    assert_renders(html, &runs).await;
}

#[tokio::test]
async fn nested_components_in_order() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let runs = Runs::default();
    let html = view! { <Gramps runs=runs.clone() /> }
        .to_html_stream_in_order()
        .collect::<String>();
    assert_renders(html, &runs).await;
}
