use crate::Children;
use leptos_dom::{Errors, HydrationCtx, IntoView};
use leptos_macro::{component, view};
use leptos_reactive::{provide_context, run_as_child, signal_prelude::*};

/// When you render a `Result<_, _>` in your view, in the `Err` case it will
/// render nothing, and search up through the view tree for an `<ErrorBoundary/>`.
/// This component lets you define a fallback that should be rendered in that
/// error case, allowing you to handle errors within a section of the interface.
///
/// ```
/// # use leptos_reactive::*;
/// # use leptos_macro::*;
/// # use leptos_dom::*; use leptos::*;
/// # let runtime = create_runtime();
/// # if false {
/// let (value, set_value) = create_signal(Ok(0));
/// let on_input =
///     move |ev| set_value.set(event_target_value(&ev).parse::<i32>());
///
/// view! {
///   <input type="text" on:input=on_input/>
///   <ErrorBoundary
///     fallback=move |_| view! { <p class="error">"Enter a valid number."</p>}
///   >
///     <p>"Value is: " {move || value.get()}</p>
///   </ErrorBoundary>
/// }
/// # ;
/// # }
/// # runtime.dispose();
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
/// view! {
///   <Suspense fallback=move || view! { <p>"Loading..."</p> }>
///     <ErrorBoundary fallback=|errors| view! { <ErrorTemplate errors=errors/>}>
///       {move || {
///   /* etc. */
/// ```
///
/// ## Beginner's Tip: ErrorBoundary Requires Your Error To Implement std::error::Error.
/// `ErrorBoundary` requires your `Result<T,E>` to implement [IntoView](https://docs.rs/leptos/latest/leptos/trait.IntoView.html).
/// `Result<T,E>` only implements `IntoView` if `E` implements [std::error::Error](https://doc.rust-lang.org/std/error/trait.Error.html).
/// So, for instance, if you pass a `Result<T,String>` where `T` implements [IntoView](https://docs.rs/leptos/latest/leptos/trait.IntoView.html)
/// and attempt to render the error for the purposes of `ErrorBoundary` you'll get a compiler error like this.
///
/// ```rust,ignore
/// error[E0599]: the method `into_view` exists for enum `Result<ViewableLoginFlow, String>`, but its trait bounds were not satisfied
///    --> src/login.rs:229:32
///     |
/// 229 |                     err => err.into_view(),
///     |                                ^^^^^^^^^ method cannot be called on `Result<ViewableLoginFlow, String>` due to unsatisfied trait bounds
///     |
///     = note: the following trait bounds were not satisfied:
///             `<&Result<ViewableLoginFlow, std::string::String> as FnOnce<()>>::Output = _`
///             which is required by `&Result<ViewableLoginFlow, std::string::String>: leptos::IntoView`
///    ... more notes here ...
/// ```
///
/// For more information about how to easily implement `Error` see
/// [thiserror](https://docs.rs/thiserror/latest/thiserror/)
#[component]
pub fn ErrorBoundary<F, IV>(
    /// The components inside the tag which will get rendered
    children: Children,
    /// A fallback that will be shown if an error occurs.
    fallback: F,
) -> impl IntoView
where
    F: Fn(RwSignal<Errors>) -> IV + 'static,
    IV: IntoView,
{
    run_as_child(move || {
        let before_children = HydrationCtx::next_error();

        let errors: RwSignal<Errors> = create_rw_signal(Errors::default());

        provide_context(errors);

        // Run children so that they render and execute resources
        _ = HydrationCtx::next_error();
        let children = children();
        HydrationCtx::continue_from(before_children);

        #[cfg(all(debug_assertions, feature = "hydrate"))]
        {
            use leptos_dom::View;
            if children.nodes.iter().any(|child| {
            matches!(child, View::Suspense(_, _))
            || matches!(child, View::Component(repr) if repr.name() == "Transition")
        }) {
            leptos_dom::logging::console_warn("You are using a <Suspense/> or \
            <Transition/> as the direct child of an <ErrorBoundary/>. To ensure correct \
            hydration, these should be reorganized so that the <ErrorBoundary/> is a child \
            of the <Suspense/> or <Transition/> instead: \n\
            \nview! {{ \
            \n  <Suspense fallback=todo!()>\n    <ErrorBoundary fallback=todo!()>\n      {{move || {{ /* etc. */")
        }
        }

        let children = children.into_view();
        let errors_empty = create_memo(move |_| errors.with(Errors::is_empty));

        move || {
            if errors_empty.get() {
                children.clone().into_view()
            } else {
                view! {
                {fallback(errors)}
                <leptos-error-boundary style="display: none">{children.clone()}</leptos-error-boundary>
            }
            .into_view()
            }
        }
    })
}
