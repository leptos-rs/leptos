use leptos_reactive::{with_root_scope, Scope};

#[test]
fn effect_runs() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let d = with_root_scope(|cx| {
        let (a, set_a) = cx.create_signal(-1);

        // simulate an arbitrary side effect
        let b = Rc::new(RefCell::new(String::new()));

        cx.create_effect({
            let b = b.clone();
            move || {
                let formatted = format!("Value is {}", a());
                *b.borrow_mut() = formatted;
            }
        });

        assert_eq!(b.borrow().as_str(), "Value is -1");

        set_a(|a| *a = 1);

        assert_eq!(b.borrow().as_str(), "Value is 1");
    });
    unsafe { d.dispose() }
}

#[test]
fn effect_tracks_memo() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let d = with_root_scope(|cx| {
        let (a, set_a) = cx.create_signal(-1);
        let b = cx.create_memo(move || format!("Value is {}", a()));

        // simulate an arbitrary side effect
        let c = Rc::new(RefCell::new(String::new()));

        cx.create_effect({
            let c = c.clone();
            move || {
                *c.borrow_mut() = b();
            }
        });

        assert_eq!(b().as_str(), "Value is -1");
        assert_eq!(c.borrow().as_str(), "Value is -1");

        set_a(|a| *a = 1);

        assert_eq!(b().as_str(), "Value is 1");
        assert_eq!(c.borrow().as_str(), "Value is 1");
    });
    unsafe { d.dispose() }
}

#[test]
fn untrack_mutes_effect() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let d = with_root_scope(|cx| {
        let (a, set_a) = cx.create_signal(-1);

        // simulate an arbitrary side effect
        let b = Rc::new(RefCell::new(String::new()));

        cx.create_effect({
            let b = b.clone();
            move || {
                let formatted = format!("Value is {}", cx.untrack(a));
                *b.borrow_mut() = formatted;
            }
        });

        assert_eq!(a(), -1);
        assert_eq!(b.borrow().as_str(), "Value is -1");

        set_a(|a| *a = 1);

        assert_eq!(a(), 1);
        assert_eq!(b.borrow().as_str(), "Value is -1");
    });
    unsafe { d.dispose() }
}
