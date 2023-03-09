use std::rc::Rc;

#[test]
fn slice() {
    use leptos_reactive::*;
    let (cx, disposer) = raw_scope_and_disposer(create_runtime());
    // this could be serialized to and from localstorage with miniserde
    pub struct State {
        token: String,
        dark_mode: bool,
    }
    // 1
    let state = create_rw_signal(
        cx,
        State {
            token: "".into(),
            // this would cause flickering on reload,
            // use a cookie for the initial value in real projects
            dark_mode: false,
        },
    );

    // 2 (3)
    let (token, set_token) = create_slice(
        cx,
        state,
        |state| state.token.clone(),
        |state, value| state.token = value,
    );

    // 4 (5)
    let (dark_mode, set_dark_mode) = create_slice(
        cx,
        state,
        |state| state.dark_mode,
        |state, value| state.dark_mode = value,
    );

    let count_token_updates = Rc::new(std::cell::Cell::new(0)); //create_rw_signal(cx, 0);
    assert_eq!(count_token_updates.get(), 0);
    create_isomorphic_effect(cx, {
        let count_token_updates = Rc::clone(&count_token_updates);
        move |_| {
            _ = token.with(|_| {});
            count_token_updates.set(count_token_updates.get() + 1);
        }
    });
    assert_eq!(count_token_updates.get(), 1);
    set_token.set("this is not a token!".into());
    // token was updated with the new token
    token.with(|token| assert_eq!(token, "this is not a token!"));
    assert_eq!(count_token_updates.get(), 2);
    set_dark_mode.set(true);
    // since token didn't change, there was also no update emitted
    assert_eq!(count_token_updates.get(), 2);
}
