use futures::StreamExt;
use gloo_timers::{
    callback::Interval,
    future::{IntervalStream, TimeoutFuture},
};
use leptos::{
    prelude::*,
    reactive_graph::{computed::AsyncDerived, owner::Stored, signal::RwSignal},
    view, Executor, IntoView,
};
use send_wrapper::SendWrapper;
use std::{cell::RefCell, future::Future};

fn wait(seconds: u8, output: char) -> impl Future<Output = char> + Send + Sync {
    SendWrapper::new(async move {
        TimeoutFuture::new(seconds as u32 * 1000).await;
        output
    })
}

pub fn async_example() -> impl IntoView {
    let trigger = RwSignal::new(());
    let count = RwSignal::new(0);

    let a = || wait(1, 'A');
    let b = || wait(2, 'B');

    let a2 = AsyncDerived::new(|| wait(1, 'A'));
    let b2 = AsyncDerived::new(|| wait(2, 'B'));

    let times = move || {
        trigger.track();
        async move { (b2.await, a2.await, " and ") }
            .suspend()
            .with_fallback("Loading...")
            .track()
    };

    let on_click = move |_| {
        let mut ticks = 0;
        let mut timer = IntervalStream::new(1000);
        Executor::spawn_local(async move {
            while timer.next().await.is_some() {
                ticks += 1;
                if ticks >= 3 {
                    break;
                }
                count.update(|n| *n += 1);
            }
        });
        count.set(1);
        trigger.set(());
    };

    view! {
        <button on:click=on_click>
            +1
        </button>
        <p>{move || count.get()}</p>
        <p>
            {times}
        </p>
    }
}
