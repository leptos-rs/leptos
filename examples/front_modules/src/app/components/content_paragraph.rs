use crate::app::*;

#[component]
pub fn content_paragraph(cx: Scope, cont: String) -> impl IntoView {
    view! {cx,
        <p>{cont}</p>
    }
}
