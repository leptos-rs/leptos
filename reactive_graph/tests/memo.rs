use reactive_graph::{
    computed::{ArcMemo, Memo},
    effect::Effect,
    executor::Executor,
    prelude::*,
    signal::RwSignal,
};
use std::{
    mem,
    sync::{Arc, RwLock},
};

pub async fn tick() {
    tokio::time::sleep(std::time::Duration::from_micros(1)).await;
}

#[test]
fn memo_calculates_value() {
    let a = RwSignal::new(1);
    let b = RwSignal::new(2);
    let c = RwSignal::new(3);

    let d = Memo::new(move |_| a.get() + b.get() + c.get());
    assert_eq!(d.get(), 6);
}

#[test]
fn arc_memo_readable() {
    let a = RwSignal::new(1);
    let b = RwSignal::new(2);
    let c = RwSignal::new(3);

    let d = ArcMemo::new(move |_| a.get() + b.get() + c.get());
    assert_eq!(*d.read(), 6);
}

#[test]
fn memo_doesnt_repeat_calculation_per_get() {
    let calculations = Arc::new(RwLock::new(0));

    let a = RwSignal::new(1);
    let b = RwSignal::new(2);
    let c = RwSignal::new(3);

    let d = Memo::new({
        let calculations = Arc::clone(&calculations);
        move |_| {
            *calculations.write().unwrap() += 1;
            a.get() + b.get() + c.get()
        }
    });
    assert_eq!(d.get(), 6);
    assert_eq!(d.get(), 6);
    assert_eq!(d.get(), 6);
    assert_eq!(*calculations.read().unwrap(), 1);

    println!("\n\n**setting to 0**");
    a.set(0);
    assert_eq!(d.get(), 5);
    assert_eq!(*calculations.read().unwrap(), 2);
}

#[test]
fn nested_memos() {
    let a = RwSignal::new(0); // 1
    let b = RwSignal::new(0); // 2
    let c = Memo::new(move |_| {
        println!("calculating C");
        a.get() + b.get()
    }); // 3
    let d = Memo::new(move |_| {
        println!("calculating D");
        c.get() * 2
    }); // 4
    let e = Memo::new(move |_| {
        println!("calculating E");
        d.get() + 1
    }); // 5
    assert_eq!(e.get(), 1);
    assert_eq!(d.get(), 0);
    assert_eq!(c.get(), 0);
    println!("\n\nFirst Set\n\n");
    a.set(5);
    assert_eq!(c.get(), 5);
    assert_eq!(d.get(), 10);
    assert_eq!(e.get(), 11);
    println!("\n\nSecond Set\n\n");
    b.set(1);
    assert_eq!(e.get(), 13);
    assert_eq!(d.get(), 12);
    assert_eq!(c.get(), 6);
}

#[test]
fn memo_runs_only_when_inputs_change() {
    let call_count = Arc::new(RwLock::new(0));
    let a = RwSignal::new(0);
    let b = RwSignal::new(0);
    let c = RwSignal::new(0);

    // pretend that this is some kind of expensive computation and we need to access its its value often
    // we could do this with a derived signal, but that would re-run the computation
    // memos should only run when their inputs actually change: this is the only point
    let c = Memo::new({
        let call_count = call_count.clone();
        move |_| {
            let mut call_count = call_count.write().unwrap();
            *call_count += 1;

            a.get() + b.get() + c.get()
        }
    });

    // initially the memo has not been called at all, because it's lazy
    assert_eq!(*call_count.read().unwrap(), 0);

    // here we access the value a bunch of times
    assert_eq!(c.get(), 0);
    assert_eq!(c.get(), 0);
    assert_eq!(c.get(), 0);
    assert_eq!(c.get(), 0);
    assert_eq!(c.get(), 0);

    // we've still only called the memo calculation once
    assert_eq!(*call_count.read().unwrap(), 1);

    // and we only call it again when an input changes
    a.set(1);
    assert_eq!(c.get(), 1);
    assert_eq!(*call_count.read().unwrap(), 2);
}

#[test]
fn diamond_problem() {
    let name = RwSignal::new("Greg Johnston".to_string());
    let first = Memo::new(move |_| {
        println!("calculating first");
        name.get().split_whitespace().next().unwrap().to_string()
    });
    let last = Memo::new(move |_| {
        println!("calculating last");
        name.get().split_whitespace().nth(1).unwrap().to_string()
    });

    let combined_count = Arc::new(RwLock::new(0));
    let combined = Memo::new({
        let combined_count = Arc::clone(&combined_count);
        move |_| {
            println!("calculating combined");
            let mut combined_count = combined_count.write().unwrap();
            *combined_count += 1;

            format!("{} {}", first.get(), last.get())
        }
    });

    assert_eq!(first.get(), "Greg");
    assert_eq!(last.get(), "Johnston");

    name.set("Will Smith".to_string());
    assert_eq!(first.get(), "Will");
    assert_eq!(last.get(), "Smith");
    assert_eq!(combined.get(), "Will Smith");
    // should not have run the memo logic twice, even
    // though both paths have been updated
    assert_eq!(*combined_count.read().unwrap(), 1);
}

#[cfg(feature = "tokio")]
#[tokio::test]
async fn dynamic_dependencies() {
    _ = Executor::init_futures_executor();

    let first = RwSignal::new("Greg");
    let last = RwSignal::new("Johnston");
    let use_last = RwSignal::new(true);
    let name = Memo::new(move |_| {
        if use_last.get() {
            format!("{} {}", first.get(), last.get())
        } else {
            first.get().to_string()
        }
    });

    let combined_count = Arc::new(RwLock::new(0));

    // we forget it so it continues running
    // if it's dropped, it will stop listening
    mem::forget(Effect::new_sync({
        let combined_count = Arc::clone(&combined_count);
        move |_| {
            _ = name.get();
            *combined_count.write().unwrap() += 1;
        }
    }));
    tick().await;

    assert_eq!(*combined_count.read().unwrap(), 1);

    first.set("Bob");
    tick().await;

    assert_eq!(name.get(), "Bob Johnston");

    assert_eq!(*combined_count.read().unwrap(), 2);

    last.set("Thompson");
    tick().await;

    assert_eq!(*combined_count.read().unwrap(), 3);

    use_last.set(false);
    tick().await;

    assert_eq!(name.get(), "Bob");
    assert_eq!(*combined_count.read().unwrap(), 4);

    assert_eq!(*combined_count.read().unwrap(), 4);
    last.set("Jones");
    tick().await;

    assert_eq!(*combined_count.read().unwrap(), 4);
    last.set("Smith");
    tick().await;

    assert_eq!(*combined_count.read().unwrap(), 4);
    last.set("Stevens");
    tick().await;

    assert_eq!(*combined_count.read().unwrap(), 4);

    use_last.set(true);
    tick().await;
    assert_eq!(name.get(), "Bob Stevens");

    assert_eq!(*combined_count.read().unwrap(), 5);
}
