#[cfg(not(feature = "stable"))]
use leptos_reactive::{
    create_isomorphic_effect, create_runtime, create_scope, create_signal,
    UntrackedGettableSignal, UntrackedSettableSignal,
};

#[cfg(not(feature = "stable"))]
#[test]
fn untracked_set_doesnt_trigger_effect() {
    use std::{cell::RefCell, rc::Rc};

    create_scope(create_runtime(), |cx| {
        let (a, set_a) = create_signal(cx, -1);

        // simulate an arbitrary side effect
        let b = Rc::new(RefCell::new(String::new()));

        create_isomorphic_effect(cx, {
            let b = b.clone();
            move |_| {
                let formatted = format!("Value is {}", a());
                *b.borrow_mut() = formatted;
            }
        });

        assert_eq!(b.borrow().as_str(), "Value is -1");

        set_a.set(1);

        assert_eq!(b.borrow().as_str(), "Value is 1");

        set_a.set_untracked(-1);

        assert_eq!(b.borrow().as_str(), "Value is 1");
    })
    .dispose()
}

#[cfg(not(feature = "stable"))]
#[test]
fn untracked_get_doesnt_trigger_effect() {
    use std::{cell::RefCell, rc::Rc};

    create_scope(create_runtime(), |cx| {
        let (a, set_a) = create_signal(cx, -1);
        let (a2, set_a2) = create_signal(cx, 1);

        // simulate an arbitrary side effect
        let b = Rc::new(RefCell::new(String::new()));

        create_isomorphic_effect(cx, {
            let b = b.clone();
            move |_| {
                let formatted =
                    format!("Values are {} and {}", a(), a2.get_untracked());
                *b.borrow_mut() = formatted;
            }
        });

        assert_eq!(b.borrow().as_str(), "Values are -1 and 1");

        set_a.set(1);

        assert_eq!(b.borrow().as_str(), "Values are 1 and 1");

        set_a.set_untracked(-1);

        assert_eq!(b.borrow().as_str(), "Values are 1 and 1");

        set_a2.set(-1);

        assert_eq!(b.borrow().as_str(), "Values are 1 and 1");

        set_a.set(-1);

        assert_eq!(b.borrow().as_str(), "Values are -1 and -1");
    })
    .dispose()
}
