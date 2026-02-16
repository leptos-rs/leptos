//! Regression test for SSR livelock when multiple Suspense boundaries share
//! Resources via `.get()`.
//!
//! Root cause: The Suspense Effect's state machine (in suspense_component.rs)
//! returned `false` as a catch-all, resetting `double_checking` from `Some(true)`
//! to `Some(false)` when the Effect saw non-empty tasks between individual monitor
//! completions. This caused `dry_resolve()` to be called again when tasks drained,
//! spawning new monitors in an infinite cycle.
//!
//! The fix: replace the catch-all `false` with `double_checking.unwrap_or(false)`,
//! preserving the `Some(true)` state through intermediate Effect wakeups.

#[cfg(feature = "ssr")]
mod imports {
    pub use any_spawner::Executor;
    pub use futures::StreamExt;
    pub use leptos::prelude::*;
}

/// Two Suspense boundaries sharing two Resources via `.get()` must resolve
/// within a reasonable time. Previously this livelocked with worker_threads=2-3
/// due to the Suspense Effect's dry_resolve re-registration cycle.
#[cfg(feature = "ssr")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shared_resources_across_suspense_boundaries_no_livelock() {
    use imports::*;

    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let r_a = Resource::new(|| (), |_| async { "a".to_string() });
    let r_b = Resource::new(|| (), |_| async { "b".to_string() });

    let app = view! {
        <Transition fallback=|| "loading 1">
            {move || {
                let a = r_a.get()?;
                let b = r_b.get()?;
                Some(view! { <div>{a} " " {b}</div> })
            }}
        </Transition>
        <Suspense fallback=|| "loading 2">
            {move || {
                let a = r_a.get()?;
                let b = r_b.get()?;
                Some(view! { <div>{a} " " {b}</div> })
            }}
        </Suspense>
    };

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        app.to_html_stream_in_order().collect::<String>(),
    )
    .await;

    assert!(
        result.is_ok(),
        "SSR timed out with worker_threads=2 — Suspense Effect livelock"
    );
}
