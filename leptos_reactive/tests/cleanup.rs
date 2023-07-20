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
