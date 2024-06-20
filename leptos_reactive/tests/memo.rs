use leptos_reactive::*;

#[test]
fn basic_memo() {
    let runtime = create_runtime();

    let a = create_memo(|_| 5);
    assert_eq!(a.get(), 5);

    runtime.dispose();
}

#[test]
fn signal_with_untracked() {
    use leptos_reactive::SignalWithUntracked;

    let runtime = create_runtime();

    let m = create_memo(move |_| 5);
    let copied_out = m.with_untracked(|value| *value);
    assert_eq!(copied_out, 5);

    runtime.dispose();
}

#[test]
fn signal_get_untracked() {
    use leptos_reactive::SignalGetUntracked;

    let runtime = create_runtime();

    let m = create_memo(move |_| "memo".to_owned());
    let cloned_out = m.get_untracked();
    assert_eq!(cloned_out, "memo".to_owned());

    runtime.dispose();
}

#[test]
fn memo_with_computed_value() {
    let runtime = create_runtime();

    let (a, set_a) = create_signal(0);
    let (b, set_b) = create_signal(0);
    let c = create_memo(move |_| a.get() + b.get());
    assert_eq!(c.get(), 0);
    set_a.set(5);
    assert_eq!(c.get(), 5);
    set_b.set(1);
    assert_eq!(c.get(), 6);

    runtime.dispose();
}

#[test]
fn nested_memos() {
    let runtime = create_runtime();

    let (a, set_a) = create_signal(0); // 1
    let (b, set_b) = create_signal(0); // 2
    let c = create_memo(move |_| a.get() + b.get()); // 3
    let d = create_memo(move |_| c.get() * 2); // 4
    let e = create_memo(move |_| d.get() + 1); // 5
    assert_eq!(d.get(), 0);
    set_a.set(5);
    assert_eq!(e.get(), 11);
    assert_eq!(d.get(), 10);
    assert_eq!(c.get(), 5);
    set_b.set(1);
    assert_eq!(e.get(), 13);
    assert_eq!(d.get(), 12);
    assert_eq!(c.get(), 6);

    runtime.dispose();
}

#[test]
fn memo_runs_only_when_inputs_change() {
    use std::{cell::Cell, rc::Rc};

    let runtime = create_runtime();

    let call_count = Rc::new(Cell::new(0));
    let (a, set_a) = create_signal(0);
    let (b, _) = create_signal(0);
    let (c, _) = create_signal(0);

    // pretend that this is some kind of expensive computation and we need to access its its value often
    // we could do this with a derived signal, but that would re-run the computation
    // memos should only run when their inputs actually change: this is the only point
    let c = create_memo({
        let call_count = call_count.clone();
        move |_| {
            call_count.set(call_count.get() + 1);
            a.get() + b.get() + c.get()
        }
    });

    // initially the memo has not been called at all, because it's lazy
    assert_eq!(call_count.get(), 0);

    // here we access the value a bunch of times
    assert_eq!(c.get(), 0);
    assert_eq!(c.get(), 0);
    assert_eq!(c.get(), 0);
    assert_eq!(c.get(), 0);
    assert_eq!(c.get(), 0);

    // we've still only called the memo calculation once
    assert_eq!(call_count.get(), 1);

    // and we only call it again when an input changes
    set_a.set(1);
    assert_eq!(c.get(), 1);
    assert_eq!(call_count.get(), 2);

    runtime.dispose();
}

#[test]
fn diamond_problem() {
    use std::{cell::Cell, rc::Rc};

    let runtime = create_runtime();

    let (name, set_name) = create_signal("Greg Johnston".to_string());
    let first = create_memo(move |_| {
        name.get().split_whitespace().next().unwrap().to_string()
    });
    let last = create_memo(move |_| {
        name.get().split_whitespace().nth(1).unwrap().to_string()
    });

    let combined_count = Rc::new(Cell::new(0));
    let combined = create_memo({
        let combined_count = Rc::clone(&combined_count);
        move |_| {
            combined_count.set(combined_count.get() + 1);
            format!("{} {}", first.get(), last.get())
        }
    });

    assert_eq!(first.get(), "Greg");
    assert_eq!(last.get(), "Johnston");

    set_name.set("Will Smith".to_string());
    assert_eq!(first.get(), "Will");
    assert_eq!(last.get(), "Smith");
    assert_eq!(combined.get(), "Will Smith");
    // should not have run the memo logic twice, even
    // though both paths have been updated
    assert_eq!(combined_count.get(), 1);

    runtime.dispose();
}

#[test]
fn dynamic_dependencies() {
    use leptos_reactive::create_isomorphic_effect;
    use std::{cell::Cell, rc::Rc};

    let runtime = create_runtime();

    let (first, set_first) = create_signal("Greg");
    let (last, set_last) = create_signal("Johnston");
    let (use_last, set_use_last) = create_signal(true);
    let name = create_memo(move |_| {
        if use_last.get() {
            format!("{} {}", first.get(), last.get())
        } else {
            first.get().to_string()
        }
    });

    let combined_count = Rc::new(Cell::new(0));

    create_isomorphic_effect({
        let combined_count = Rc::clone(&combined_count);
        move |_| {
            _ = name.get();
            combined_count.set(combined_count.get() + 1);
        }
    });

    assert_eq!(combined_count.get(), 1);

    set_first.set("Bob");
    assert_eq!(name.get(), "Bob Johnston");

    assert_eq!(combined_count.get(), 2);

    set_last.set("Thompson");

    assert_eq!(combined_count.get(), 3);

    set_use_last.set(false);

    assert_eq!(name.get(), "Bob");
    assert_eq!(combined_count.get(), 4);

    assert_eq!(combined_count.get(), 4);
    set_last.set("Jones");
    assert_eq!(combined_count.get(), 4);
    set_last.set("Smith");
    assert_eq!(combined_count.get(), 4);
    set_last.set("Stevens");
    assert_eq!(combined_count.get(), 4);

    set_use_last.set(true);
    assert_eq!(name.get(), "Bob Stevens");
    assert_eq!(combined_count.get(), 5);

    runtime.dispose();
}

#[test]
fn owning_memo_slice() {
    use std::rc::Rc;
    let runtime = create_runtime();

    // this could be serialized to and from localstorage with miniserde
    pub struct State {
        name: String,
        token: String,
    }

    let state = create_rw_signal(State {
        name: "Alice".to_owned(),
        token: "is this a token????".to_owned(),
    });

    // We can allocate only when `state.name` changes
    let name = create_owning_memo(move |old_name| {
        state.with(move |state| {
            if let Some(name) =
                old_name.filter(|old_name| old_name == &state.name)
            {
                (name, false)
            } else {
                (state.name.clone(), true)
            }
        })
    });
    let set_name = move |name| state.update(|state| state.name = name);

    // We can also re-use the last token allocation, which may be even better if the tokens are
    // always of the same length
    let token = create_owning_memo(move |old_token| {
        state.with(move |state| {
            let is_different = old_token.as_ref() != Some(&state.token);
            let mut token = old_token.unwrap_or_default();

            if is_different {
                token.clone_from(&state.token);
            }
            (token, is_different)
        })
    });
    let set_token =
        move |new_token| state.update(|state| state.token = new_token);

    let count_name_updates = Rc::new(std::cell::Cell::new(0));
    assert_eq!(count_name_updates.get(), 0);
    create_isomorphic_effect({
        let count_name_updates = Rc::clone(&count_name_updates);
        move |_| {
            name.track();
            count_name_updates.set(count_name_updates.get() + 1);
        }
    });
    assert_eq!(count_name_updates.get(), 1);

    let count_token_updates = Rc::new(std::cell::Cell::new(0));
    assert_eq!(count_token_updates.get(), 0);
    create_isomorphic_effect({
        let count_token_updates = Rc::clone(&count_token_updates);
        move |_| {
            token.track();
            count_token_updates.set(count_token_updates.get() + 1);
        }
    });
    assert_eq!(count_token_updates.get(), 1);

    set_name("Bob".to_owned());
    name.with(|name| assert_eq!(name, "Bob"));
    assert_eq!(count_name_updates.get(), 2);
    assert_eq!(count_token_updates.get(), 1);

    set_token("this is not a token!".to_owned());
    token.with(|token| assert_eq!(token, "this is not a token!"));
    assert_eq!(count_name_updates.get(), 2);
    assert_eq!(count_token_updates.get(), 2);

    runtime.dispose();
}

#[test]
fn leak_on_dispose() {
    use std::rc::Rc;

    let runtime = create_runtime();

    let trigger = create_trigger();

    let value = Rc::new(());
    let weak = Rc::downgrade(&value);

    let memo = create_memo(move |_| {
        trigger.track();

        create_rw_signal(value.clone());
    });

    memo.get_untracked();

    memo.dispose();

    assert!(weak.upgrade().is_none()); // Should have been dropped.

    runtime.dispose();
}
