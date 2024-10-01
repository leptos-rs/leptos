#[cfg(feature = "effects")]
use any_spawner::Executor;
#[cfg(feature = "effects")]
use reactive_graph::owner::Owner;
#[cfg(feature = "effects")]
use reactive_graph::{effect::Effect, prelude::*, signal::RwSignal};
#[cfg(feature = "effects")]
use std::sync::{Arc, RwLock};
#[cfg(feature = "effects")]
use tokio::task;

#[cfg(feature = "effects")]
#[tokio::test]
async fn watch_runs() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    task::LocalSet::new()
        .run_until(async {
            let a = RwSignal::new(-1);

            // simulate an arbitrary side effect
            let b = Arc::new(RwLock::new(String::new()));

            let effect = Effect::watch(
                move || a.get(),
                {
                    let b = b.clone();

                    move |a, prev_a, prev_ret| {
                        let formatted = format!(
                            "Value is {a}; Prev is {prev_a:?}; Prev return is \
                             {prev_ret:?}"
                        );
                        *b.write().unwrap() = formatted;

                        a + 10
                    }
                },
                false,
            );

            Executor::tick().await;
            assert_eq!(b.read().unwrap().as_str(), "");

            a.set(1);

            Executor::tick().await;
            assert_eq!(
                b.read().unwrap().as_str(),
                "Value is 1; Prev is Some(-1); Prev return is None"
            );

            a.set(2);

            Executor::tick().await;
            assert_eq!(
                b.read().unwrap().as_str(),
                "Value is 2; Prev is Some(1); Prev return is Some(11)"
            );

            effect.stop();

            *b.write().unwrap() = "nothing happened".to_string();
            a.set(3);

            Executor::tick().await;
            assert_eq!(b.read().unwrap().as_str(), "nothing happened");
        })
        .await
}

#[cfg(feature = "effects")]
#[tokio::test]
async fn watch_runs_immediately() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    task::LocalSet::new()
        .run_until(async {
            let a = RwSignal::new(-1);

            // simulate an arbitrary side effect
            let b = Arc::new(RwLock::new(String::new()));

            Effect::watch(
                move || a.get(),
                {
                    let b = b.clone();

                    move |a, prev_a, prev_ret| {
                        let formatted = format!(
                            "Value is {a}; Prev is {prev_a:?}; Prev return is \
                             {prev_ret:?}"
                        );
                        *b.write().unwrap() = formatted;

                        a + 10
                    }
                },
                true,
            );

            Executor::tick().await;
            assert_eq!(
                b.read().unwrap().as_str(),
                "Value is -1; Prev is None; Prev return is None"
            );

            a.set(1);

            Executor::tick().await;
            assert_eq!(
                b.read().unwrap().as_str(),
                "Value is 1; Prev is Some(-1); Prev return is Some(9)"
            );
        })
        .await
}

#[cfg(feature = "effects")]
#[tokio::test]
async fn watch_ignores_callback() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    task::LocalSet::new()
        .run_until(async {
            let a = RwSignal::new(-1);
            let b = RwSignal::new(0);

            // simulate an arbitrary side effect
            let s = Arc::new(RwLock::new(String::new()));

            Effect::watch(
                move || a.get(),
                {
                    let s = s.clone();

                    move |a, _, _| {
                        let formatted =
                            format!("Value a is {a}; Value b is {}", b.get());
                        *s.write().unwrap() = formatted;

                        a + 10
                    }
                },
                false,
            );

            Executor::tick().await;

            a.set(1);

            Executor::tick().await;
            assert_eq!(
                s.read().unwrap().as_str(),
                "Value a is 1; Value b is 0"
            );

            *s.write().unwrap() = "nothing happened".to_string();
            b.set(10);

            Executor::tick().await;
            assert_eq!(s.read().unwrap().as_str(), "nothing happened");

            a.set(2);

            Executor::tick().await;
            assert_eq!(
                s.read().unwrap().as_str(),
                "Value a is 2; Value b is 10"
            );
        })
        .await
}

#[cfg(feature = "effects")]
#[tokio::test]
async fn deprecated_watch_runs() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    task::LocalSet::new()
        .run_until(async {
            let a = RwSignal::new(-1);

            // simulate an arbitrary side effect
            let b = Arc::new(RwLock::new(String::new()));

            #[allow(deprecated)]
            let effect = reactive_graph::effect::watch(
                move || a.get(),
                {
                    let b = b.clone();

                    move |a, prev_a, prev_ret| {
                        let formatted = format!(
                            "Value is {a}; Prev is {prev_a:?}; Prev return is \
                             {prev_ret:?}"
                        );
                        *b.write().unwrap() = formatted;

                        a + 10
                    }
                },
                false,
            );

            Executor::tick().await;
            assert_eq!(b.read().unwrap().as_str(), "");

            a.set(1);

            Executor::tick().await;
            assert_eq!(
                b.read().unwrap().as_str(),
                "Value is 1; Prev is Some(-1); Prev return is None"
            );

            a.set(2);

            Executor::tick().await;
            assert_eq!(
                b.read().unwrap().as_str(),
                "Value is 2; Prev is Some(1); Prev return is Some(11)"
            );

            effect();

            *b.write().unwrap() = "nothing happened".to_string();
            a.set(3);

            Executor::tick().await;
            assert_eq!(b.read().unwrap().as_str(), "nothing happened");
        })
        .await
}
