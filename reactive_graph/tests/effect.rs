#[cfg(feature = "effects")]
pub mod imports {
    pub use any_spawner::Executor;
    pub use reactive_graph::{
        effect::{Effect, RenderEffect},
        prelude::*,
        signal::RwSignal,
    };
    pub use std::{
        mem,
        sync::{Arc, RwLock},
    };
    pub use tokio::task;
}

#[cfg(feature = "effects")]
#[tokio::test]
async fn render_effect_runs() {
    use imports::*;

    _ = Executor::init_tokio();
    task::LocalSet::new()
        .run_until(async {
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

            Executor::tick().await;
            assert_eq!(b.read().unwrap().as_str(), "Value is -1");

            println!("setting to 1");
            a.set(1);

            Executor::tick().await;
            assert_eq!(b.read().unwrap().as_str(), "Value is 1");
        })
        .await;
}

#[cfg(feature = "effects")]
#[tokio::test]
async fn effect_runs() {
    use imports::*;

    _ = Executor::init_tokio();

    task::LocalSet::new()
        .run_until(async {
            let a = RwSignal::new(-1);

            // simulate an arbitrary side effect
            let b = Arc::new(RwLock::new(String::new()));

            Effect::new({
                let b = b.clone();
                move || {
                    let formatted = format!("Value is {}", a.get());
                    *b.write().unwrap() = formatted;
                }
            });

            Executor::tick().await;
            assert_eq!(b.read().unwrap().as_str(), "Value is -1");

            println!("setting to 1");
            a.set(1);

            Executor::tick().await;
            assert_eq!(b.read().unwrap().as_str(), "Value is 1");
        })
        .await
}

#[cfg(feature = "effects")]
#[tokio::test]
async fn dynamic_dependencies() {
    use imports::*;

    _ = Executor::init_tokio();

    task::LocalSet::new()
        .run_until(async {
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

            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 1);

            println!("\nsetting `first` to Bob");
            first.set("Bob");
            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 2);

            println!("\nsetting `last` to Bob");
            last.set("Thompson");
            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 3);

            println!("\nsetting `use_last` to false");
            use_last.set(false);
            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 4);

            println!("\nsetting `last` to Jones");
            last.set("Jones");
            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 4);

            println!("\nsetting `last` to Jones");
            last.set("Smith");
            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 4);

            println!("\nsetting `last` to Stevens");
            last.set("Stevens");
            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 4);

            println!("\nsetting `use_last` to true");
            use_last.set(true);
            Executor::tick().await;
            assert_eq!(*combined_count.read().unwrap(), 5);
        })
        .await
}
