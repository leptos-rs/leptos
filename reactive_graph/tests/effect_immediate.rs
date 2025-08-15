#[cfg(feature = "effects")]
pub mod imports {
    pub use any_spawner::Executor;
    pub use reactive_graph::{
        effect::ImmediateEffect, owner::Owner, prelude::*, signal::RwSignal,
    };
    pub use std::sync::{Arc, RwLock};
    pub use tokio::task;
}

#[cfg(feature = "effects")]
#[test]
fn effect_runs() {
    use imports::*;

    let owner = Owner::new();
    owner.set();

    let a = RwSignal::new(-1);

    // simulate an arbitrary side effect
    let b = Arc::new(RwLock::new(String::new()));

    let _guard = ImmediateEffect::new({
        let b = b.clone();
        move || {
            let formatted = format!("Value is {}", a.get());
            *b.write().unwrap() = formatted;
        }
    });
    assert_eq!(b.read().unwrap().as_str(), "Value is -1");

    println!("setting to 1");
    a.set(1);
    assert_eq!(b.read().unwrap().as_str(), "Value is 1");
}

#[cfg(feature = "effects")]
#[test]
fn dynamic_dependencies() {
    use imports::*;

    let owner = Owner::new();
    owner.set();

    let first = RwSignal::new("Greg");
    let last = RwSignal::new("Johnston");
    let use_last = RwSignal::new(true);

    let combined_count = Arc::new(RwLock::new(0));

    let _guard = ImmediateEffect::new({
        let combined_count = Arc::clone(&combined_count);
        move || {
            *combined_count.write().unwrap() += 1;
            if use_last.get() {
                println!("{} {}", first.get(), last.get());
            } else {
                println!("{}", first.get());
            }
        }
    });

    assert_eq!(*combined_count.read().unwrap(), 1);

    println!("\nsetting `first` to Bob");
    first.set("Bob");
    assert_eq!(*combined_count.read().unwrap(), 2);

    println!("\nsetting `last` to Bob");
    last.set("Thompson");
    assert_eq!(*combined_count.read().unwrap(), 3);

    println!("\nsetting `use_last` to false");
    use_last.set(false);
    assert_eq!(*combined_count.read().unwrap(), 4);

    println!("\nsetting `last` to Jones");
    last.set("Jones");
    assert_eq!(*combined_count.read().unwrap(), 4);

    println!("\nsetting `last` to Jones");
    last.set("Smith");
    assert_eq!(*combined_count.read().unwrap(), 4);

    println!("\nsetting `last` to Stevens");
    last.set("Stevens");
    assert_eq!(*combined_count.read().unwrap(), 4);

    println!("\nsetting `use_last` to true");
    use_last.set(true);
    assert_eq!(*combined_count.read().unwrap(), 5);
}

#[cfg(feature = "effects")]
#[test]
fn recursive_effect_runs_recursively() {
    use imports::*;

    let owner = Owner::new();
    owner.set();

    let s = RwSignal::new(0);

    let logged_values = Arc::new(RwLock::new(Vec::new()));

    let _guard = ImmediateEffect::new({
        let logged_values = Arc::clone(&logged_values);
        move || {
            let a = s.get();
            println!("a = {a}");
            logged_values.write().unwrap().push(a);
            if a == 0 {
                return;
            }
            s.set(0);
        }
    });

    s.set(1);
    s.set(2);
    s.set(3);

    assert_eq!(0, s.get_untracked());
    assert_eq!(&*logged_values.read().unwrap(), &[0, 1, 0, 2, 0, 3, 0]);
}

#[cfg(feature = "effects")]
#[test]
fn paused_effect_pauses() {
    use imports::*;
    use reactive_graph::owner::StoredValue;

    let owner = Owner::new();
    owner.set();

    let a = RwSignal::new(-1);

    // simulate an arbitrary side effect
    let runs = StoredValue::new(0);

    let owner = StoredValue::new(None);

    let _guard = ImmediateEffect::new({
        move || {
            *owner.write_value() = Owner::current();

            let _ = a.get();
            *runs.write_value() += 1;
        }
    });

    assert_eq!(runs.get_value(), 1);

    println!("setting to 1");
    a.set(1);

    assert_eq!(runs.get_value(), 2);

    println!("pausing");
    owner.get_value().unwrap().pause();

    println!("setting to 2");
    a.set(2);

    assert_eq!(runs.get_value(), 2);

    println!("resuming");
    owner.get_value().unwrap().resume();

    println!("setting to 3");
    a.set(3);

    println!("checking value");
    assert_eq!(runs.get_value(), 3);
}

#[cfg(feature = "effects")]
#[test]
#[ignore = "Parallel signal access can panic."]
fn threaded_chaos_effect() {
    use imports::*;
    use reactive_graph::owner::StoredValue;

    const SIGNAL_COUNT: usize = 5;
    const THREAD_COUNT: usize = 10;

    let owner = Owner::new();
    owner.set();

    let signals = vec![RwSignal::new(0); SIGNAL_COUNT];

    let runs = StoredValue::new(0);

    let _guard = ImmediateEffect::new({
        let signals = signals.clone();
        move || {
            *runs.write_value() += 1;

            let mut values = vec![];
            for s in &signals {
                let v = s.get();
                values.push(v);
                if v != 0 {
                    s.set(v - 1);
                }
            }
            println!("{values:?}");
        }
    });

    std::thread::scope(|s| {
        for _ in 0..THREAD_COUNT {
            let signals = signals.clone();
            s.spawn(move || {
                for s in &signals {
                    s.set(1);
                }
            });
        }
    });

    assert_eq!(runs.get_value(), 1 + THREAD_COUNT * SIGNAL_COUNT);

    let values: Vec<_> = signals.iter().map(|s| s.get_untracked()).collect();
    println!("FINAL: {values:?}");
}
