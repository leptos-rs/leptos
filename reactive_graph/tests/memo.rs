use reactive_graph::{
    computed::{ArcMemo, Memo},
    owner::Owner,
    prelude::*,
    signal::RwSignal,
    wrappers::read::Signal,
};
use std::{
    rc::Rc,
    sync::{Arc, RwLock},
};

#[cfg(feature = "effects")]
pub mod imports {
    pub use any_spawner::Executor;
    pub use reactive_graph::{
        computed::{ArcMemo, Memo},
        effect::{Effect, RenderEffect},
        prelude::*,
        signal::RwSignal,
        wrappers::read::Signal,
    };
    pub use std::{
        mem,
        rc::Rc,
        sync::{Arc, RwLock},
    };
    pub use tokio::task;
}

#[test]
fn memo_calculates_value() {
    let owner = Owner::new();
    owner.set();

    let a = RwSignal::new(1);
    let b = RwSignal::new(2);
    let c = RwSignal::new(3);

    let d = Memo::new(move |_| a.get() + b.get() + c.get());
    assert_eq!(d.read(), 6);
    assert_eq!(d.with_untracked(|n| *n), 6);
    assert_eq!(d.with(|n| *n), 6);
    assert_eq!(d.get_untracked(), 6);
}

#[test]
fn arc_memo_readable() {
    let owner = Owner::new();
    owner.set();

    let a = RwSignal::new(1);
    let b = RwSignal::new(2);
    let c = RwSignal::new(3);

    let d = ArcMemo::new(move |_| a.get() + b.get() + c.get());
    assert_eq!(d.read(), 6);
}

#[test]
fn memo_doesnt_repeat_calculation_per_get() {
    let owner = Owner::new();
    owner.set();

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
    assert_eq!(d.get_untracked(), 6);
    assert_eq!(d.get_untracked(), 6);
    assert_eq!(d.get_untracked(), 6);
    assert_eq!(*calculations.read().unwrap(), 1);

    println!("\n\n**setting to 0**");
    a.set(0);
    assert_eq!(d.get_untracked(), 5);
    assert_eq!(*calculations.read().unwrap(), 2);
}

#[test]
fn nested_memos() {
    let owner = Owner::new();
    owner.set();

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
    assert_eq!(e.get_untracked(), 1);
    assert_eq!(d.get_untracked(), 0);
    assert_eq!(c.get_untracked(), 0);

    println!("\n\nFirst Set\n\n");
    a.set(5);
    assert_eq!(c.get_untracked(), 5);
    assert_eq!(d.get_untracked(), 10);
    assert_eq!(e.get_untracked(), 11);

    println!("\n\nSecond Set\n\n");
    b.set(1);
    assert_eq!(e.get_untracked(), 13);
    assert_eq!(d.get_untracked(), 12);
    assert_eq!(c.get_untracked(), 6);
}

#[test]
fn memo_runs_only_when_inputs_change() {
    let owner = Owner::new();
    owner.set();

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
    assert_eq!(c.get_untracked(), 0);
    assert_eq!(c.get_untracked(), 0);
    assert_eq!(c.get_untracked(), 0);
    assert_eq!(c.get_untracked(), 0);
    assert_eq!(c.get_untracked(), 0);

    // we've still only called the memo calculation once
    assert_eq!(*call_count.read().unwrap(), 1);

    // and we only call it again when an input changes
    a.set(1);
    assert_eq!(c.get_untracked(), 1);
    assert_eq!(*call_count.read().unwrap(), 2);
}

#[test]
fn diamond_problem() {
    let owner = Owner::new();
    owner.set();

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

    assert_eq!(first.get_untracked(), "Greg");
    assert_eq!(last.get_untracked(), "Johnston");

    name.set("Will Smith".to_string());
    assert_eq!(first.get_untracked(), "Will");
    assert_eq!(last.get_untracked(), "Smith");
    assert_eq!(combined.get_untracked(), "Will Smith");
    // should not have run the memo logic twice, even
    // though both paths have been updated
    assert_eq!(*combined_count.read().unwrap(), 1);
}

#[cfg(feature = "effects")]
#[tokio::test]
async fn dynamic_dependencies() {
    let owner = Owner::new();
    owner.set();

    use imports::*;

    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

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
    println!("[Initial]");
    Effect::new_sync({
        let combined_count = Arc::clone(&combined_count);
        move |_| {
            println!("Effect running.");
            _ = name.get();
            *combined_count.write().unwrap() += 1;
        }
    });
    Executor::tick().await;
    println!("[After 1 tick]");

    assert_eq!(*combined_count.read().unwrap(), 1);

    println!("[Set 'Bob']");
    first.set("Bob");
    Executor::tick().await;

    assert_eq!(name.get_untracked(), "Bob Johnston");

    assert_eq!(*combined_count.read().unwrap(), 2);

    println!("[Set 'Thompson']");
    last.set("Thompson");
    Executor::tick().await;

    assert_eq!(*combined_count.read().unwrap(), 3);

    use_last.set(false);
    Executor::tick().await;

    assert_eq!(name.get_untracked(), "Bob");
    assert_eq!(*combined_count.read().unwrap(), 4);

    assert_eq!(*combined_count.read().unwrap(), 4);
    last.set("Jones");
    Executor::tick().await;

    assert_eq!(*combined_count.read().unwrap(), 4);
    last.set("Smith");
    Executor::tick().await;

    assert_eq!(*combined_count.read().unwrap(), 4);
    last.set("Stevens");
    Executor::tick().await;

    assert_eq!(*combined_count.read().unwrap(), 4);

    use_last.set(true);
    Executor::tick().await;
    assert_eq!(name.get_untracked(), "Bob Stevens");

    assert_eq!(*combined_count.read().unwrap(), 5);
}

#[cfg(feature = "effects")]
#[tokio::test]
async fn render_effect_doesnt_rerun_if_memo_didnt_change() {
    let owner = Owner::new();
    owner.set();

    use imports::*;

    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    task::LocalSet::new()
        .run_until(async {
            let count = RwSignal::new(1);
            let even = Memo::new(move |_| *count.read() % 2 == 0);

            let combined_count = Arc::new(RwLock::new(0));

            println!("[Initial]");
            mem::forget(RenderEffect::new({
                let combined_count = Arc::clone(&combined_count);
                move |_| {
                    println!("INSIDE RENDEREFFECT");
                    *combined_count.write().unwrap() += 1;
                    println!("even = {}", even.get());
                }
            }));

            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 1);
            println!("[done]\n");

            println!("\n[Set Signal to 2]");
            count.set(2);
            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 2);
            println!("[done]\n");

            println!("\n[Set Signal to 4]");
            count.set(4);
            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 2);
            println!("[done]\n");
        })
        .await
}

#[cfg(feature = "effects")]
#[tokio::test]
async fn effect_doesnt_rerun_if_memo_didnt_change() {
    let owner = Owner::new();
    owner.set();

    use imports::*;

    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    task::LocalSet::new()
        .run_until(async {
            let count = RwSignal::new(1);
            let even = Memo::new(move |_| *count.read() % 2 == 0);

            let combined_count = Arc::new(RwLock::new(0));

            Effect::new({
                let combined_count = Arc::clone(&combined_count);
                move |_| {
                    *combined_count.write().unwrap() += 1;
                    println!("even = {}", even.get());
                }
            });

            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 1);

            count.set(2);
            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 2);

            count.set(4);
            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 2);
        })
        .await
}

#[cfg(feature = "effects")]
#[tokio::test]
async fn effect_depending_on_signal_and_memo_doesnt_rerun_unnecessarily() {
    let owner = Owner::new();
    owner.set();

    use imports::*;

    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    task::LocalSet::new()
        .run_until(async {
            let other_signal = RwSignal::new(false);
            let count = RwSignal::new(1);
            let even = Memo::new(move |_| *count.read() % 2 == 0);

            let combined_count = Arc::new(RwLock::new(0));

            Effect::new({
                let combined_count = Arc::clone(&combined_count);
                move |_| {
                    *combined_count.write().unwrap() += 1;
                    println!(
                        "even = {}\nother_signal = {}",
                        even.get(),
                        other_signal.get()
                    );
                }
            });

            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 1);

            count.set(2);
            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 2);

            count.set(4);
            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 2);
        })
        .await
}

#[test]
fn unsync_derived_signal_and_memo() {
    let owner = Owner::new();
    owner.set();

    let a = RwSignal::new_local(Rc::new(1));
    let b = RwSignal::new(2);
    let c = RwSignal::new(3);
    let d = Memo::new(move |_| *a.get() + b.get() + c.get());

    let e = Rc::new(0);
    let f = Signal::derive_local(move || d.get() + *e);

    assert_eq!(d.read(), 6);
    assert_eq!(d.with_untracked(|n| *n), 6);
    assert_eq!(d.with(|n| *n), 6);
    assert_eq!(d.get_untracked(), 6);

    // derived signal also works
    assert_eq!(f.with_untracked(|n| *n), 6);
    assert_eq!(f.with(|n| *n), 6);
    assert_eq!(f.get_untracked(), 6);
}

#[cfg(feature = "effects")]
#[tokio::test]
async fn test_memo_multiple_read_guards() {
    // regression test for https://github.com/leptos-rs/leptos/issues/3158
    let owner = Owner::new();
    owner.set();
    use imports::*;

    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();
    task::LocalSet::new()
        .run_until(async {
            let memo = Memo::<i32>::new_with_compare(|_| 42, |_, _| true);

            Effect::new(move |_| {
                let guard_a = memo.read();
                let guard_b = memo.read();
                assert_eq!(guard_a, 42);
                assert_eq!(guard_b, 42);
            });
            Executor::tick().await;
        })
        .await
}

#[cfg(feature = "effects")]
#[tokio::test]
async fn test_memo_read_guard_held() {
    // regression test for https://github.com/leptos-rs/leptos/issues/3252
    let owner = Owner::new();
    owner.set();
    use imports::*;

    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();
    task::LocalSet::new()
        .run_until(async {
            let source = RwSignal::new(0);

            let directly_derived =
                Memo::new_with_compare(move |_| source.get(), |_, _| true);
            let indirect = Memo::new_with_compare(
                move |_| directly_derived.get(),
                |_, _| true,
            );

            Effect::new(move |_| {
                let direct_value = directly_derived.read();
                let indirect_value = indirect.get();
                assert_eq!(direct_value, indirect_value);
            });

            Executor::tick().await;
            source.set(1);
            Executor::tick().await;
            source.set(2);
            Executor::tick().await;
        })
        .await
}

#[test]
fn memo_updates_even_if_not_read_until_later() {
    #![allow(clippy::bool_assert_comparison)]

    let owner = Owner::new();
    owner.set();

    // regression test for https://github.com/leptos-rs/leptos/issues/3339

    let input = RwSignal::new(0);
    let first_memo = Memo::new(move |_| input.get() == 1);
    let second_memo = Memo::new(move |_| first_memo.get());

    assert_eq!(input.get(), 0);
    assert_eq!(first_memo.get(), false);

    println!("update to 1");
    input.set(1);
    assert_eq!(input.get(), 1);
    println!("read memo 1");
    assert_eq!(first_memo.get(), true);
    println!("read memo 2");
    assert_eq!(second_memo.get(), true);

    // this time, we don't read the memo
    println!("\nupdate to 2");
    input.set(2);
    assert_eq!(input.get(), 2);
    println!("read memo 1");
    assert_eq!(first_memo.get(), false);

    println!("\nupdate to 3");
    input.set(3);
    assert_eq!(input.get(), 3);
    println!("read memo 1");
    assert_eq!(first_memo.get(), false);
    println!("read memo 2");
    assert_eq!(second_memo.get(), false);
}
