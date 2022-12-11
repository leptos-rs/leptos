use leptos_dom::{Fragment, IntoView, View};
use leptos_reactive::{provide_context, Scope, SignalSetter, SuspenseContext};
use typed_builder::TypedBuilder;

/// Props for the [Suspense](crate::Suspense) component, which shows a fallback
/// while [Resource](leptos_reactive::Resource)s are being read.
#[derive(TypedBuilder)]
pub struct TransitionProps<F, E>
where
    F: Fn() -> E + 'static,
    E: IntoView,
{
    /// Will be displayed while resources are pending.
    pub fallback: F,
    /// A function that will be called when the component transitions into or out of
    /// the `pending` state, with its argument indicating whether it is pending (`true`)
    /// or not pending (`false`).
    #[builder(default, setter(strip_option, into))]
    pub set_pending: Option<SignalSetter<bool>>,
    /// Will be displayed once all resources have resolved.
    pub children: Box<dyn Fn() -> Fragment>,
}

/// If any [Resource](leptos_reactive::Resource)s are read in the `children` of this
/// component, it will show the `fallback` while they are loading. Once all are resolved,
/// it will render the `children`. Unlike [`Suspense`](crate::Suspense), this will not fall
/// back to the `fallback` state if there are further changes after the initial load.
///
/// Note that the `children` will be rendered initially (in order to capture the fact that
/// those resources are read under the suspense), so you cannot assume that resources have
/// `Some` value in `children`.
///
/// ```
/// # use leptos_reactive::*;
/// # use leptos_core::*;
/// # use leptos_macro::*;
/// # use leptos_dom::*; use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// # if cfg!(not(any(feature = "csr", feature = "hydrate", feature = "ssr"))) {
/// async fn fetch_cats(how_many: u32) -> Result<Vec<String>, ()> { Ok(vec![]) }
///
/// let (cat_count, set_cat_count) = create_signal::<u32>(cx, 1);
/// let (pending, set_pending) = create_signal(cx, false);
///
/// let cats = create_resource(cx, cat_count, |count| fetch_cats(count));
///
/// view! { cx,
///   <div>
///     <Transition
///       fallback=move || view! { cx, <p>"Loading..."</p>}
///       set_pending=set_pending
///     >
///       {move || {
///           cats.read().map(|data| match data {
///             Err(_) => view! { cx,  <pre>"Error"</pre> },
///             Ok(cats) => view! { cx,
///               <div>{
///                 cats.iter()
///                   .map(|src| {
///                     view! { cx,
///                       <img src={src}/>
///                     }
///                   })
///                   .collect::<Vec<_>>()
///               }</div>
///             },
///           })
///         }
///       }
///     </Transition>
///   </div>
/// };
/// # }
/// # });
/// ```
#[allow(non_snake_case)]
pub fn Transition<F, E>(cx: Scope, props: TransitionProps<F, E>) -> impl IntoView
where
    F: Fn() -> E + 'static,
    E: IntoView,
{
    let context = SuspenseContext::new(cx);

    // provide this SuspenseContext to any resources below it
    provide_context(cx, context);

    render_transition(
        cx,
        context,
        props.fallback,
        props.children,
        props.set_pending,
    )
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
fn render_transition<'a, F, E>(
    cx: Scope,
    context: SuspenseContext,
    fallback: F,
    child: Box<dyn Fn() -> Fragment>,
    set_pending: Option<SignalSetter<bool>>,
) -> impl IntoView
where
    F: Fn() -> E + 'static,
    E: IntoView,
{
    use std::cell::RefCell;

    let prev_child = RefCell::new(None);

    (move || {
        if context.ready() {
            // TODO should be unnecessary: seems to be a bug if DynChild
            // receives a Fragment as its new child
            let current_child = leptos_dom::div(cx).child(child()).into_view(cx);
            //let current_child = child().into_view(cx);
            *prev_child.borrow_mut() = Some(current_child.clone());
            if let Some(pending) = &set_pending {
                pending.set(false);
            }
            current_child
        } else if let Some(prev_child) = &*prev_child.borrow() {
            if let Some(pending) = &set_pending {
                pending.set(true);
            }
            prev_child.clone()
        } else {
            if let Some(pending) = &set_pending {
                pending.set(true);
            }
            let fallback = fallback().into_view(cx);
            *prev_child.borrow_mut() = Some(fallback.clone());
            fallback
        }
    })
    .into_view(cx)
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
fn render_transition<'a, F, E, H, G>(
    cx: Scope,
    context: SuspenseContext,
    fallback: F,
    orig_child: G,
    set_pending: Option<SignalSetter<bool>>,
) -> View
where
    F: Fn() -> E + 'static,
    E: IntoView,
    G: Fn() -> H + 'static,
    H: IntoView,
{
    use leptos_dom::*;

    let initial = {
        // run the child; we'll probably throw this away, but it will register resource reads
        let child = orig_child().into_view(cx);

        // no resources were read under this, so just return the child
        if context.pending_resources.get() == 0 {
            child
        }
        // show the fallback, but also prepare to stream HTML
        else {
            let key = cx.current_fragment_key();
            cx.register_suspense(context, &key, move || {
                render_to_string(move |cx| orig_child())
            });

            // return the fallback for now, wrapped in fragment identifer
            div(cx)
                .attr("data-fragment", key)
                .child(move || fallback())
                .into_view(cx)
        }
    };
    initial
}
