use futures::StreamExt;
use gloo_timers::{
    callback::Interval,
    future::{IntervalStream, TimeoutFuture},
};
use leptos::{
    prelude::*,
    reactive_graph::{computed::AsyncDerived, owner::Stored, signal::RwSignal},
    tachys::log,
    view, Executor, IntoView,
};
use send_wrapper::SendWrapper;
use std::future::Future;

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

pub fn async_example() -> impl IntoView {
    let a = RwSignal::new(0);
    let b = RwSignal::new(1);

    let a2 = AsyncDerived::new(move || wait('A', 1, a.get()));
    let b2 = AsyncDerived::new(move || wait('B', 3, b.get()));
    let c = AsyncDerived::new(move || async move { a2.await + b2.await });

    let times = move || {
        //let a2 = wait('A', 1, a.get());
        //let b2 = wait('B', 3, b.get());
        async move { (a2.await, " + ", b2.await) }
            //async move { (a2.await, " + ", b2.await, " = ", c.await) }
            .suspend()
            .with_fallback("Loading...")
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
        <p>
            {times}
        </p>
    }
}
