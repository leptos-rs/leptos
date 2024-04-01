use leptos_reactive::{
    create_isomorphic_effect, create_runtime, signal_prelude::*,
};

#[test]
fn untracked_set_doesnt_trigger_effect() {
    use std::{cell::RefCell, rc::Rc};

    let runtime = create_runtime();

    let (a, set_a) = create_signal(-1);

    // simulate an arbitrary side effect
    let b = Rc::new(RefCell::new(String::new()));

    create_isomorphic_effect({
        let b = b.clone();
        move |_| {
            let formatted = format!("Value is {}", a.get());
            *b.borrow_mut() = formatted;
        }
    });

    assert_eq!(b.borrow().as_str(), "Value is -1");

    set_a.set(1);

    assert_eq!(b.borrow().as_str(), "Value is 1");

    set_a.set_untracked(-1);

    assert_eq!(b.borrow().as_str(), "Value is 1");

    runtime.dispose();
}

#[test]
fn untracked_get_doesnt_trigger_effect() {
    use std::{cell::RefCell, rc::Rc};

    let runtime = create_runtime();

    let (a, set_a) = create_signal(-1);
    let (a2, set_a2) = create_signal(1);

    // simulate an arbitrary side effect
    let b = Rc::new(RefCell::new(String::new()));

    create_isomorphic_effect({
        let b = b.clone();
        move |_| {
            let formatted =
                format!("Values are {} and {}", a.get(), a2.get_untracked());
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

    runtime.dispose();
}
