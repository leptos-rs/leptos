use reactive_graph::{
    effect::{Effect, RenderEffect},
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

#[cfg(feature = "futures-executor")]
#[test]
fn render_effect_runs() {
    _ = Executor::init_futures_executor();

    Executor::spawn(async {
        let a = RwSignal::new(-1);

        // simulate an arbitrary side effect
        let b = Arc::new(RwLock::new(String::new()));

        // we forget it so it continues running
        // if it's dropped, it will stop listening
        mem::forget(RenderEffect::new({
            let b = b.clone();
            move |_| {
                let formatted = format!("Value is {}", a.get());
                *b.write().unwrap() = formatted;
            }
        }));

        tick().await;
        assert_eq!(b.read().unwrap().as_str(), "Value is -1");

        println!("setting to 1");
        a.set(1);

        tick().await;
        assert_eq!(b.read().unwrap().as_str(), "Value is 1");
    });
}

#[cfg(feature = "futures-executor")]
#[test]
fn effect_runs() {
    _ = Executor::init_futures_executor();

    Executor::spawn(async {
        let a = RwSignal::new(-1);

        // simulate an arbitrary side effect
        let b = Arc::new(RwLock::new(String::new()));

        Effect::new({
            let b = b.clone();
            move |_| {
                let formatted = format!("Value is {}", a.get());
                *b.write().unwrap() = formatted;
            }
        });

        tick().await;
        assert_eq!(b.read().unwrap().as_str(), "Value is -1");

        println!("setting to 1");
        a.set(1);

        tick().await;
        assert_eq!(b.read().unwrap().as_str(), "Value is 1");
    });
}
#[cfg(feature = "futures-executor")]
#[test]
fn dynamic_dependencies() {
    _ = Executor::init_futures_executor();

    Executor::spawn(async {
        let first = RwSignal::new("Greg");
        let last = RwSignal::new("Johnston");
        let use_last = RwSignal::new(true);

        let combined_count = Arc::new(RwLock::new(0));

        mem::forget(RenderEffect::new({
            let combined_count = Arc::clone(&combined_count);
            move |_| {
                *combined_count.write().unwrap() += 1;
                if use_last.get() {
                    println!("{} {}", first.get(), last.get());
                } else {
                    println!("{}", first.get());
                }
            }
        }));

        tick().await;
        assert_eq!(*combined_count.read().unwrap(), 1);

        println!("\nsetting `first` to Bob");
        first.set("Bob");
        tick().await;
        assert_eq!(*combined_count.read().unwrap(), 2);

        println!("\nsetting `last` to Bob");
        last.set("Thompson");
        tick().await;
        assert_eq!(*combined_count.read().unwrap(), 3);

        println!("\nsetting `use_last` to false");
        use_last.set(false);
        tick().await;
        assert_eq!(*combined_count.read().unwrap(), 4);

        println!("\nsetting `last` to Jones");
        last.set("Jones");
        tick().await;
        assert_eq!(*combined_count.read().unwrap(), 4);

        println!("\nsetting `last` to Jones");
        last.set("Smith");
        tick().await;
        assert_eq!(*combined_count.read().unwrap(), 4);

        println!("\nsetting `last` to Stevens");
        last.set("Stevens");
        tick().await;
        assert_eq!(*combined_count.read().unwrap(), 4);

        println!("\nsetting `use_last` to true");
        use_last.set(true);
        tick().await;
        assert_eq!(*combined_count.read().unwrap(), 5);
    });
}

/*
#[cfg(feature = "futures-executor")]
#[test]
fn effect_runs() {
    _ = Executor::init_futures_executor();

    Executor::spawn(async {});
}

#[cfg(feature = "futures-executor")]
#[test]
fn effect_runs() {
    _ = Executor::init_futures_executor();

    Executor::spawn(async {});
}*/
