use crate as leptos;
use leptos_dom::{Child, IntoAttribute, IntoChild};
use leptos_macro::Props;
use leptos_reactive::{debug_warn, provide_context, Scope, SuspenseContext};

#[derive(Props)]
pub struct SuspenseProps<F, E, G, H>
where
    F: IntoChild + Clone,
    E: IntoChild,
    G: Fn() -> E,
    H: Fn() -> G,
{
    fallback: F,
    children: H,
}

#[allow(non_snake_case)]
pub fn Suspense<F, E, G, H>(cx: Scope, props: SuspenseProps<F, E, G, H>) -> impl Fn() -> Child
where
    F: IntoChild + Clone,
    E: IntoChild,
    G: Fn() -> E + 'static,
    H: Fn() -> G,
{
    let context = SuspenseContext::new(cx);

    // provide this SuspenseContext to any resources below it
    provide_context(cx, context.clone());

    let child = (props.children)();

    render_suspense(cx, context, props.fallback.clone(), child)
}

#[cfg(not(feature = "ssr"))]
fn render_suspense<'a, F, E, G>(
    cx: Scope,
    context: SuspenseContext,
    fallback: F,
    child: G,
) -> impl Fn() -> Child
where
    F: IntoChild + Clone,
    E: IntoChild,
    G: Fn() -> E,
{
    move || {
        if context.ready() || cx.transition_pending() {
            (child)().into_child(cx)
        } else {
            fallback.clone().into_child(cx)
        }
    }
}

#[cfg(feature = "ssr")]
fn render_suspense<'a, F, E, G>(
    cx: Scope,
    context: SuspenseContext,
    fallback: F,
    orig_child: G,
) -> impl Fn() -> Child
where
    F: IntoChild + Clone,
    E: IntoChild,
    G: Fn() -> E + 'static,
{
    use leptos_macro::view;

    let initial = {
        // run the child; we'll probably throw this away, but it will register resource reads
        let mut child = orig_child().into_child(cx);
        while let Child::Fn(f) = child {
            child = (f.borrow_mut())();
        }

        // no resources were read under this, so just return the child
        if context.pending_resources.get() == 0 {
            child
        }
        // show the fallback, but also prepare to stream HTML
        else {
            let key = cx.current_fragment_key();
            cx.register_suspense(context, &key, move || {
                orig_child().into_child(cx).as_child_string()
            });

            // return the fallback for now, wrapped in fragment identifer
            Child::Node(view! { <div data-fragment-id={key}>{fallback.into_child(cx)}</div> })
        }
    };
    move || initial.clone()
}
