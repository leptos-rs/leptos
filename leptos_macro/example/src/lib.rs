use leptos::*;

#[component]
fn TestComponent(
    _cx: Scope,
    /// ```
    /// assert_eq!("hello", stringify!(hello));
    /// ```
    #[prop(into)]
    key: String,
) -> impl IntoView {
    _ = key;
    todo!()
}

