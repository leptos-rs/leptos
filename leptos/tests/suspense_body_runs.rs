//! These test and record the current trade-off between:
//! - #4430 / commit 4f3a26c: re-running children so that conditional resource
//!   reads that depend on *other* resources are discovered before the
//!   Suspense resolves.
//! - #4688: side effects in the Suspense body running multiple times per SSR
//!   render.

#![cfg(feature = "ssr")]

use any_spawner::Executor;
use futures::StreamExt;
use leptos::prelude::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

async fn render(app: impl IntoView + Send + 'static) -> String {
    app.to_html_stream_in_order().collect::<String>().await
}

/// No resources read in the Suspense body at all. The "double-check" added
/// by 4f3a26c cannot possibly discover anything new, yet the body still runs
/// more than once.
#[tokio::test]
async fn body_runs_with_no_resources() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let count = Arc::new(AtomicUsize::new(0));
    let count_in = Arc::clone(&count);

    let app = view! {
        <Suspense>{move || {
            count_in.fetch_add(1, Ordering::SeqCst);
            "hi"
        }}</Suspense>
    };

    let html = render(app).await;
    assert!(html.contains("hi"), "rendered html was: {html:?}");

    // With no async resources, the double-check pass is skipped, so the
    // body only runs once for the initial `dry_resolve` and once for the
    // final `resolve`.
    let runs = count.load(Ordering::SeqCst);
    println!("no-resource case: body ran {runs} times");
    assert_eq!(runs, 2, "expected 2 runs in the no-resource case");
}

/// One top-level resource, read unconditionally. The framework only needs a
/// single tracking pass to discover it — the double-check cannot reveal
/// anything new.
#[tokio::test]
async fn body_runs_with_one_resource() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let count = Arc::new(AtomicUsize::new(0));
    let count_in = Arc::clone(&count);

    let res = Resource::new(
        || (),
        |_| async move {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            42
        },
    );

    let app = view! {
        <Suspense>{move || {
            count_in.fetch_add(1, Ordering::SeqCst);
            res.get().map(|v| v.to_string())
        }}</Suspense>
    };

    let html = render(app).await;
    assert!(html.contains("42"), "rendered html was: {html:?}");

    let runs = count.load(Ordering::SeqCst);
    println!("single-resource case: body ran {runs} times");
    assert_eq!(runs, 3, "expected 3 runs in the single-resource case");
}

/// The case the double-check exists for: a resource whose completion reveals
/// a nested resource read. Here the double-check is necessary for
/// correctness — the second resource must be discovered before the Suspense
/// resolves, otherwise we'd render stale/None content.
#[tokio::test]
async fn body_runs_with_conditional_nested_resource() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let count = Arc::new(AtomicUsize::new(0));
    let count_in = Arc::clone(&count);

    let outer = Resource::new(
        || (),
        |_| async move {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            true
        },
    );
    let inner = Resource::new(
        || (),
        |_| async move {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            "inner-data".to_string()
        },
    );

    let app = view! {
        <Suspense>{move || {
            count_in.fetch_add(1, Ordering::SeqCst);
            // `inner` is only read on runs where `outer` has resolved. The
            // double-check is what makes us notice that read and wait for
            // `inner` before resolving the Suspense.
            outer.get().and_then(|flag| {
                if flag { inner.get() } else { None }
            })
        }}</Suspense>
    };

    let html = render(app).await;
    assert!(
        html.contains("inner-data"),
        "conditional nested resource must resolve before Suspense settles; \
         got: {html:?}"
    );

    let runs = count.load(Ordering::SeqCst);
    println!("nested-resource case: body ran {runs} times");
    assert_eq!(runs, 3, "expected 3 runs in the nested-resource case");
}

/// `strict=true` disables the double-check pass, so a Suspense with one
/// top-level resource runs the body twice (initial `dry_resolve` + final
/// `resolve`) rather than three times.
#[tokio::test]
async fn body_runs_with_strict_suspense() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let count = Arc::new(AtomicUsize::new(0));
    let count_in = Arc::clone(&count);

    let res = Resource::new(
        || (),
        |_| async move {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            42
        },
    );

    let app = view! {
        <Suspense strict=true>{move || {
            count_in.fetch_add(1, Ordering::SeqCst);
            res.get().map(|v| v.to_string())
        }}</Suspense>
    };

    let html = render(app).await;
    assert!(html.contains("42"), "rendered html was: {html:?}");

    let runs = count.load(Ordering::SeqCst);
    println!("strict single-resource case: body ran {runs} times");
    assert_eq!(runs, 2, "expected 2 runs in strict mode");
}

/// Same behavior for `<Transition strict=true/>`.
#[tokio::test]
async fn body_runs_with_strict_transition() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let count = Arc::new(AtomicUsize::new(0));
    let count_in = Arc::clone(&count);

    let res = Resource::new(
        || (),
        |_| async move {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            42
        },
    );

    let app = view! {
        <Transition strict=true>{move || {
            count_in.fetch_add(1, Ordering::SeqCst);
            res.get().map(|v| v.to_string())
        }}</Transition>
    };

    let html = render(app).await;
    assert!(html.contains("42"), "rendered html was: {html:?}");

    let runs = count.load(Ordering::SeqCst);
    println!("strict transition case: body ran {runs} times");
    assert_eq!(runs, 2, "expected 2 runs in strict transition mode");
}
