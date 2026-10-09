//! These test and record how many times a Suspense body runs during one SSR
//! render: once for the initial `dry_resolve` that registers resource reads,
//! once for `resolve`, plus one more run for every round of reads that
//! `resolve` finds still pending (#4430, #4688).

#![cfg(feature = "ssr")]

use any_spawner::Executor;
use futures::StreamExt;
use leptos::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

async fn render(app: impl IntoView + 'static) -> String {
    app.to_html_stream_in_order().collect::<String>().await
}

/// No resources read in the Suspense body at all.
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

    let runs = count.load(Ordering::SeqCst);
    println!("no-resource case: body ran {runs} times");
    assert_eq!(runs, 2, "expected 2 runs in the no-resource case");
}

/// One top-level resource, read unconditionally. It is registered by the
/// initial walk and has resolved by the time the body runs again.
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
    assert_eq!(runs, 2, "expected 2 runs in the single-resource case");
}

/// A resource whose completion reveals a nested resource read. The read is
/// discovered while resolving, waited for, and the body runs once more.
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
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            "inner-data".to_string()
        },
    );

    let app = view! {
        <Suspense>{move || {
            count_in.fetch_add(1, Ordering::SeqCst);
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

#[tokio::test]
async fn out_of_order_body_runs_with_conditional_nested_resource() {
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
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            "inner-data".to_string()
        },
    );

    let app = view! {
        <Suspense>{move || {
            count_in.fetch_add(1, Ordering::SeqCst);
            outer.get().and_then(|flag| {
                if flag { inner.get() } else { None }
            })
        }}</Suspense>
    };

    let html = app.to_html_stream_out_of_order().collect::<String>().await;
    assert!(
        html.contains("inner-data"),
        "conditional nested resource must resolve before the out-of-order \
         fragment renders; got: {html:?}"
    );

    let runs = count.load(Ordering::SeqCst);
    println!("out-of-order nested case: body ran {runs} times");
    assert_eq!(runs, 3, "expected 3 runs in the out-of-order nested case");
}

#[tokio::test]
async fn body_runs_with_two_levels_of_nesting() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let count = Arc::new(AtomicUsize::new(0));
    let count_in = Arc::clone(&count);

    let a = Resource::new(
        || (),
        |_| async move {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            true
        },
    );
    let b = Resource::new(
        || (),
        |_| async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            true
        },
    );
    let c = Resource::new(
        || (),
        |_| async move {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            "deep-data".to_string()
        },
    );

    let app = view! {
        <Suspense>{move || {
            count_in.fetch_add(1, Ordering::SeqCst);
            a.get().and_then(|a| if a { b.get() } else { None })
                   .and_then(|b| if b { c.get() } else { None })
        }}</Suspense>
    };

    let html = render(app).await;
    assert!(
        html.contains("deep-data"),
        "a read gated behind two rounds of resources must still be waited \
         for; got: {html:?}"
    );

    let runs = count.load(Ordering::SeqCst);
    println!("two-level nested case: body ran {runs} times");
    // the initial walk, then one run per level revealed, then the final run
    assert_eq!(runs, 4, "expected 4 runs in the two-level nested case");
}
