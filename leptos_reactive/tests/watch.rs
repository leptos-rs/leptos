use leptos_reactive::{
    create_runtime, create_scope, create_signal, watch, SignalGet, SignalSet,
};
use std::{cell::RefCell, rc::Rc};

#[test]
fn watch_runs() {
    create_scope(create_runtime(), |cx| {
        let (a, set_a) = create_signal(cx, -1);

        // simulate an arbitrary side effect
        let b = Rc::new(RefCell::new(String::new()));

        let stop = watch(
            cx,
            move || a.get(),
            {
                let b = b.clone();

                move |a, prev_a, prev_ret| {
                    let formatted = format!(
                        "Value is {}; Prev is {:?}; Prev return is {:?}",
                        a, prev_a, prev_ret
                    );
                    *b.borrow_mut() = formatted;

                    a + 10
                }
            },
            false,
        );

        assert_eq!(b.borrow().as_str(), "");

        set_a.set(1);

        assert_eq!(
            b.borrow().as_str(),
            "Value is 1; Prev is Some(-1); Prev return is None"
        );

        set_a.set(2);

        assert_eq!(
            b.borrow().as_str(),
            "Value is 2; Prev is Some(1); Prev return is Some(11)"
        );

        stop();

        *b.borrow_mut() = "nothing happened".to_string();
        set_a.set(3);

        assert_eq!(b.borrow().as_str(), "nothing happened");
    })
    .dispose()
}

#[test]
fn watch_runs_immediately() {
    create_scope(create_runtime(), |cx| {
        let (a, set_a) = create_signal(cx, -1);

        // simulate an arbitrary side effect
        let b = Rc::new(RefCell::new(String::new()));

        let _ = watch(
            cx,
            move || a.get(),
            {
                let b = b.clone();

                move |a, prev_a, prev_ret| {
                    let formatted = format!(
                        "Value is {}; Prev is {:?}; Prev return is {:?}",
                        a, prev_a, prev_ret
                    );
                    *b.borrow_mut() = formatted;

                    a + 10
                }
            },
            true,
        );

        assert_eq!(
            b.borrow().as_str(),
            "Value is -1; Prev is None; Prev return is None"
        );

        set_a.set(1);

        assert_eq!(
            b.borrow().as_str(),
            "Value is 1; Prev is Some(-1); Prev return is Some(9)"
        );
    })
    .dispose()
}

#[test]
fn watch_ignores_callback() {
    create_scope(create_runtime(), |cx| {
        let (a, set_a) = create_signal(cx, -1);
        let (b, set_b) = create_signal(cx, 0);

        // simulate an arbitrary side effect
        let s = Rc::new(RefCell::new(String::new()));

        let _ = watch(
            cx,
            move || a.get(),
            {
                let s = s.clone();

                move |a, _, _| {
                    let formatted =
                        format!("Value a is {}; Value b is {}", a, b.get());
                    *s.borrow_mut() = formatted;
                }
            },
            false,
        );

        set_a.set(1);

        assert_eq!(s.borrow().as_str(), "Value a is 1; Value b is 0");

        *s.borrow_mut() = "nothing happened".to_string();

        set_b.set(10);

        assert_eq!(s.borrow().as_str(), "nothing happened");

        set_a.set(2);

        assert_eq!(s.borrow().as_str(), "Value a is 2; Value b is 10");
    })
    .dispose()
}
