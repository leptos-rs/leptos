#[test]
fn cleanup() {
    use leptos_reactive::{
        create_isomorphic_effect, create_runtime, create_signal, on_cleanup,
        SignalSet, SignalWith,
    };
    use std::{cell::Cell, rc::Rc};

    let runtime = create_runtime();

    let runs = Rc::new(Cell::new(0));
    let cleanups = Rc::new(Cell::new(0));

    let (a, set_a) = create_signal(-1);

    create_isomorphic_effect({
        let cleanups = Rc::clone(&cleanups);
        let runs = Rc::clone(&runs);
        move |_| {
            a.track();
            runs.set(runs.get() + 1);
            on_cleanup({
                let cleanups = Rc::clone(&cleanups);
                move || {
                    cleanups.set(cleanups.get() + 1);
                }
            });
        }
    });

    assert_eq!(cleanups.get(), 0);
    assert_eq!(runs.get(), 1);

    set_a.set(1);

    assert_eq!(runs.get(), 2);
    assert_eq!(cleanups.get(), 1);

    set_a.set(2);

    assert_eq!(runs.get(), 3);
    assert_eq!(cleanups.get(), 2);

    runtime.dispose();
}

#[test]
fn cleanup_on_dispose() {
    use leptos_reactive::{
        create_memo, create_runtime, create_trigger, on_cleanup, SignalDispose,
        SignalGetUntracked,
    };

    struct ExecuteOnDrop(Option<Box<dyn FnOnce()>>);
    impl ExecuteOnDrop {
        fn new(f: impl FnOnce() + 'static) -> Self {
            Self(Some(Box::new(f)))
        }
    }
    impl Drop for ExecuteOnDrop {
        fn drop(&mut self) {
            self.0.take().unwrap()();
        }
    }

    let runtime = create_runtime();

    let trigger = create_trigger();

    println!("STARTING");

    let memo = create_memo(move |_| {
        trigger.track();

        // An example of why you might want to do this is that
        // when something goes out of reactive scope you want it to be cleaned up.
        // The cleaning up might have side effects, and those side effects might cause
        // re-renders where new `on_cleanup` are registered.
        let on_drop = ExecuteOnDrop::new(|| {
            on_cleanup(|| println!("Nested cleanup in progress."))
        });

        on_cleanup(move || {
            println!("Cleanup in progress.");
            drop(on_drop)
        });
    });
    println!("Memo 1: {:?}", memo);
    let _ = memo.get_untracked(); // First cleanup registered.

    memo.dispose(); // Cleanup not run here.

    println!("Cleanup should have been executed.");

    let memo = create_memo(move |_| {
        // New cleanup registered. It'll panic here.
        on_cleanup(move || println!("Test passed."));
    });
    println!("Memo 2: {:?}", memo);
    println!("^ Note how the memos have the same key (different versions).");
    let _ = memo.get_untracked(); // First cleanup registered.

    println!("Test passed.");

    memo.dispose();

    runtime.dispose();
}
