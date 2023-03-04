use crate::app::*;

#[component]
pub fn paragraf(cx: Scope, cont: String) -> impl IntoView {
    view! {cx,
        <p>{cont}</p>
    }
}
