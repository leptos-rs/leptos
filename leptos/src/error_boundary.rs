use crate::Children;
use leptos_dom::{Errors, HydrationCtx, IntoView};
use leptos_macro::{component, view};
use leptos_reactive::{
    create_rw_signal, provide_context, signal_prelude::*, RwSignal, Scope,
};

/// When you render a `Result<_, _>` in your view, in the `Err` case it will
/// render nothing, and search up through the view tree for an `<ErrorBoundary/>`.
/// This component lets you define a fallback that should be rendered in that
/// error case, allowing you to handle errors within a section of the interface.
///
/// ```
/// # use leptos_reactive::*;
/// # use leptos_macro::*;
/// # use leptos_dom::*; use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// let (value, set_value) = create_signal(cx, Ok(0));
/// let on_input = move |ev| set_value.set(event_target_value(&ev).parse::<i32>());
///
/// view! { cx,
///   <input type="text" on:input=on_input/>
///   <ErrorBoundary
///     fallback=move |_, _| view! { cx, <p class="error">"Enter a valid number."</p>}
///   >
///     <p>"Value is: " {move || value.get()}</p>
///   </ErrorBoundary>
/// }
/// # });
/// ```
///
/// ## Interaction with `<Suspense/>`
/// If you use this with a `<Suspense/>` or `<Transition/>` component, note that the
/// `<ErrorBoundary/>` should go inside the `<Suspense/>`, not the other way around,
/// if there’s a chance that the `<ErrorBoundary/>` will begin in the error state.
/// This is a limitation of the current design of the two components and the way they
/// hydrate. Placing the `<ErrorBoundary/>` outside the `<Suspense/>` means that
/// it is rendered on the server without any knowledge of the suspended view, so it
/// will always be rendered on the server as if there were no errors, but might need
/// to be hydrated with errors, depending on the actual result.
///
/// ```rust,ignore
/// view! { cx,
///   <Suspense fallback=move || view! {cx, <p>"Loading..."</p> }>
///     <ErrorBoundary fallback=|cx, errors| view!{ cx, <ErrorTemplate errors=errors/>}>
///       {move || {
///   /* etc. */
/// ```
#[component]
pub fn ErrorBoundary<F, IV>(
    cx: Scope,
    /// The components inside the tag which will get rendered
    children: Children,
    /// A fallback that will be shown if an error occurs.
    fallback: F,
) -> impl IntoView
where
    F: Fn(Scope, RwSignal<Errors>) -> IV + 'static,
    IV: IntoView,
{
    let before_children = HydrationCtx::next_component();

    let errors: RwSignal<Errors> = create_rw_signal(cx, Errors::default());

    provide_context(cx, errors);

    // Run children so that they render and execute resources
    _ = HydrationCtx::next_component();
    let children = children(cx);
    HydrationCtx::continue_from(before_children);

    #[cfg(all(debug_assertions, feature = "hydrate"))]
    {
        use leptos_dom::View;
        if children.nodes.iter().any(|child| {
            matches!(child, View::Suspense(_, _))
            || matches!(child, View::Component(repr) if repr.name() == "Transition")
        }) {
            crate::debug_warn!("You are using a <Suspense/> or \
            <Transition/> as the direct child of an <ErrorBoundary/>. To ensure correct \
            hydration, these should be reorganized so that the <ErrorBoundary/> is a child \
            of the <Suspense/> or <Transition/> instead: \n\
            \nview! {{ cx,\
            \n  <Suspense fallback=todo!()>\n    <ErrorBoundary fallback=todo!()>\n      {{move || {{ /* etc. */")
        }
    }

    let children = children.into_view(cx);
    let errors_empty = create_memo(cx, move |_| errors.with(Errors::is_empty));

    move || {
        if errors_empty.get() {
            children.clone().into_view(cx)
        } else {
            view! { cx,
                <>
                    {fallback(cx, errors)}
                    <leptos-error-boundary style="display: none">{children.clone()}</leptos-error-boundary>
                </>
            }
            .into_view(cx)
        }
    }
}
