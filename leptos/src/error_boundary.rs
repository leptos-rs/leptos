use crate::Children;
use leptos_dom::{Errors, HydrationCtx, IntoView};
use leptos_macro::{component, view};
use leptos_reactive::{provide_context, run_as_child, signal_prelude::*};

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
    /// The elements that will be rendered, which may include one or more `Result<_>` types.
    children: Children,
    /// A fallback that will be shown if an error occurs.
    fallback: F,
) -> impl IntoView
where
    F: Fn(Error) -> IV + 'static,
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
