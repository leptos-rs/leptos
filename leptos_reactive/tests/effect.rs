use leptos_reactive::{create_effect, create_memo, create_scope, create_signal};

#[test]
fn effect_runs() {
    use std::cell::RefCell;
    use std::rc::Rc;

    create_scope(|cx| {
        let (a, set_a) = create_signal(cx, -1);

        // simulate an arbitrary side effect
        let b = Rc::new(RefCell::new(String::new()));

        create_effect(cx, {
            let b = b.clone();
            move |_| {
                let formatted = format!("Value is {}", a());
                *b.borrow_mut() = formatted;
            }
        });

        assert_eq!(b.borrow().as_str(), "Value is -1");

        set_a(1);

        assert_eq!(b.borrow().as_str(), "Value is 1");
    })
    .dispose()
}

#[test]
fn effect_tracks_memo() {
    use std::cell::RefCell;
    use std::rc::Rc;

    create_scope(|cx| {
        let (a, set_a) = create_signal(cx, -1);
        let b = create_memo(cx, move |_| format!("Value is {}", a()));

        // simulate an arbitrary side effect
        let c = Rc::new(RefCell::new(String::new()));

        create_effect(cx, {
            let c = c.clone();
            move |_| {
                *c.borrow_mut() = b();
            }
        });

        assert_eq!(b().as_str(), "Value is -1");
        assert_eq!(c.borrow().as_str(), "Value is -1");

        set_a(1);

        assert_eq!(b().as_str(), "Value is 1");
        assert_eq!(c.borrow().as_str(), "Value is 1");
    })
    .dispose()
}

#[test]
fn untrack_mutes_effect() {
    use std::cell::RefCell;
    use std::rc::Rc;

    create_scope(|cx| {
        let (a, set_a) = create_signal(cx, -1);

        // simulate an arbitrary side effect
        let b = Rc::new(RefCell::new(String::new()));

        create_effect(cx, {
            let b = b.clone();
            move |_| {
                let formatted = format!("Value is {}", cx.untrack(a));
                *b.borrow_mut() = formatted;
            }
        });

        assert_eq!(a(), -1);
        assert_eq!(b.borrow().as_str(), "Value is -1");

        set_a(1);

        assert_eq!(a(), 1);
        assert_eq!(b.borrow().as_str(), "Value is -1");
    })
    .dispose()
}
