
#[test]
fn slice() {
    use leptos_reactive::*;
    let (cx, disposer) = raw_scope_and_disposer(create_runtime());
    // this could be serialized to and from localstorage with miniserde
    pub struct State {
        token: String,
        dark_mode: bool,
    }
    let state = create_rw_signal(
        cx,
        State {
            token: "".into(),
            // this would cause flickering on reload,
            // use a cookie for the initial value in real projects
            dark_mode: false,
        },
        );
    let (token, set_token) = create_slice(
        cx,
        state,
        |state| state.token.clone(),
        |state, value| state.token = value,
        );
    let (dark_mode, set_dark_mode) = create_slice(
        cx,
        state,
        |state| state.dark_mode,
        |state, value| state.dark_mode = value,
        );
    let count_token_updates = create_rw_signal(cx, 0);
    count_token_updates.with(|counter| assert_eq!(counter, &0));
    create_isomorphic_effect(cx, move |_| {
        _ = token.with(|_| {});
        count_token_updates.update(|counter| *counter += 1)
    });
    count_token_updates.with(|counter| assert_eq!(counter, &1));
    set_token.set("this is not a token!".into());
    // token was updated with the new token
    token.with(|token| assert_eq!(token, "this is not a token!"));
    count_token_updates.with(|counter| assert_eq!(counter, &2));
    set_dark_mode.set(true);
    // since token didn't change, there was also no update emitted
    count_token_updates.with(|counter| assert_eq!(counter, &2));
}
