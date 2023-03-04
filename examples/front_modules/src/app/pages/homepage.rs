use crate::app::components::content_paragraph::*;
use crate::app::*;
/// Renders the home page of your application.
#[component]
pub fn HomePage(cx: Scope) -> impl IntoView {
    // Creates a reactive value to update the button
    let (count, set_count) = create_signal(cx, 0);
    let on_click = move |_| set_count.update(|count| *count += 1);
    let content: String = "hello front end".to_string();

    view! { cx,
        <h1>"Welcome to Leptos!"</h1>
        <button on:click=on_click>"Click Me: " {count}</button>
        <ContentParagraph cont=content/>
    }
}
