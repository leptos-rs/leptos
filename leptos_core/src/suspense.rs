use crate as leptos;
use leptos_dom::{Child, IntoChild};
use leptos_macro::Props;
use leptos_reactive::{debug_warn, provide_context, Scope, SuspenseContext};

#[derive(Props)]
pub struct SuspenseProps<F, E, G>
where
    F: IntoChild + Clone,
    E: IntoChild,
    G: Fn() -> E,
{
    fallback: F,
    children: Vec<G>,
}

#[allow(non_snake_case)]
pub fn Suspense<F, E, G>(cx: Scope, mut props: SuspenseProps<F, E, G>) -> impl Fn() -> Child
where
    F: IntoChild + Clone,
    E: IntoChild,
    G: Fn() -> E,
{
    let context = SuspenseContext::new(cx);

    if props.children.len() > 1 {
        debug_warn!("[Suspense] Only pass one function as a child to <Suspense/>. Other children will be ignored.");
    }

    // guard against a zero-length Children; warn but don't panic
    let child = if props.children.is_empty() {
        debug_warn!("[Suspense] You need to pass a function as a child to <Suspense/>.");
        None
    } else {
        Some(props.children.swap_remove(0))
    };

    // provide this SuspenseContext to any resources below it
    provide_context(cx, context);

    move || {
        if context.ready() || cx.transition_pending() {
            child.as_ref().map(|child| (child)()).into_child(cx)
        } else {
            props.fallback.clone().into_child(cx)
        }
    }
}
