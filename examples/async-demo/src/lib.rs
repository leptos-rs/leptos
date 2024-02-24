use gloo_timers::future::TimeoutFuture;
use leptos::{
    prelude::*,
    reactive_graph::{computed::AsyncDerived, signal::RwSignal},
    tachys::log,
    view, IntoView,
};
use send_wrapper::SendWrapper;
use std::future::{Future, IntoFuture};

fn wait(
    id: char,
    seconds: u8,
    value: u32,
) -> impl Future<Output = u32> + Send + Sync {
    SendWrapper::new(async move {
        log(&format!("loading data for {id}"));
        TimeoutFuture::new(seconds as u32 * 1000).await;
        value + 1
    })
}

fn sleep(seconds: u32) -> impl Future<Output = ()> + Send + Sync {
    SendWrapper::new(async move {
        TimeoutFuture::new(seconds * 1000).await;
    })
}

pub fn async_example() -> impl IntoView {
    let a = RwSignal::new(0);
    let b = RwSignal::new(1);

    let a2 = create_resource(a, |a| wait('A', 1, a));
    let b2 = create_resource(b, |a| wait('A', 1, b));

    view! {
        <button on:click=move |_| {
            a.update(|n| *n += 1);
        }>
            {a}
        </button>
        <button on:click=move |_| {
            b.update(|n| *n += 1);
        }>
            {b}
        </button>
        <Suspense>
            {move || a2().map(move |a2| {
                b2().map(move |b2| {
                    view! {
                        <p>{a2} + {b2}</p>
                    }
                })
            })}
        </Suspense>
        <p>
            //{times}
        </p>
    }
}

/*
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 */
pub fn notes() -> impl IntoView {
    let a = RwSignal::new(0);
    let b = RwSignal::new(1);

    let a2 = AsyncDerived::new(move || wait('A', 1, a.get()));
    let b2 = AsyncDerived::new(move || wait('B', 3, b.get()));
    let c = AsyncDerived::new(move || async move {
        sleep(1).await;
        a2.await + b2.await
    });

    let a_and_b = move || {
        async move { (a2.await, " + ", b2.await) }
            .suspend()
            .with_fallback("Loading A and B...")
            .track()
    };

    let c = move || {
        c.into_future()
            .suspend()
            .with_fallback("Loading C...")
            .track()
    };

    view! {
        <button on:click=move |_| {
            a.update(|n| *n += 1);
        }>
            {a}
        </button>
        <button on:click=move |_| {
            b.update(|n| *n += 1);
        }>
            {b}
        </button>
        <p> {a_and_b} </p>
        <p> {c} </p>
    }
}
