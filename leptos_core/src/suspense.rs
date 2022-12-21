use std::rc::Rc;

use leptos_dom::{Component, Fragment, HydrationCtx, IntoView};
use leptos_reactive::{provide_context, Scope, SuspenseContext};
use typed_builder::TypedBuilder;

/// Props for the [Suspense](crate::Suspense) component, which shows a fallback
/// while [Resource](leptos_reactive::Resource)s are being read.
#[derive(TypedBuilder)]
pub struct SuspenseProps<F, E>
where
    F: Fn() -> E + 'static,
    E: IntoView,
{
    /// Will be displayed while resources are pending.
    pub fallback: F,
    /// Will be displayed once all resources have resolved.
    pub children: Box<dyn Fn(Scope) -> Fragment>,
}

/// If any [Resource](leptos_reactive::Resource)s are read in the `children` of this
/// component, it will show the `fallback` while they are loading. Once all are resolved,
/// it will render the `children`.
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
///
/// let cats = create_resource(cx, cat_count, |count| fetch_cats(count));
///
/// view! { cx,
///   <div>
///     <Suspense fallback=move || view! { cx, <p>"Loading (Suspense Fallback)..."</p> }>
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
///     </Suspense>
///   </div>
/// };
/// # }
/// # });
/// ```
#[allow(non_snake_case)]
pub fn Suspense<F, E>(cx: Scope, props: SuspenseProps<F, E>) -> impl IntoView
where
    F: Fn() -> E + 'static,
    E: IntoView,
{
    let context = SuspenseContext::new(cx);

    // provide this SuspenseContext to any resources below it
    provide_context(cx, context);

    render_suspense(cx, context, props.fallback, Rc::new(move |cx| (props.children)(cx)))
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
fn render_suspense<F, E>(
    _cx: Scope,
    context: SuspenseContext,
    fallback: F,
    child: Rc<dyn Fn(Scope) -> Fragment>,
) -> impl IntoView
where
    F: Fn() -> E + 'static,
    E: IntoView,
{
    use leptos_dom::DynChild;

    Component::new("Suspense", move |cx| {
       let current_id = HydrationCtx::peek();
        if context.ready() {
            HydrationCtx::continue_from(current_id);
            child(cx).into_view(cx)
        } else {
            HydrationCtx::continue_from(current_id);
            fallback().into_view(cx)
        }
    })
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
fn render_suspense<'a, F, E>(
    cx: Scope,
    context: SuspenseContext,
    fallback: F,
    orig_child: Rc<dyn Fn(Scope) -> Fragment>,
) -> impl IntoView
where
    F: Fn() -> E + 'static,
    E: IntoView,
{
    use leptos_dom::DynChild;

    let orig_child = Rc::clone(&orig_child);

    Component::new("Suspense", move |cx| {
        let current_id = HydrationCtx::peek();

        // run the child; we'll probably throw this away, but it will register resource reads
        let child = orig_child(cx).into_view(cx);

        let initial = {    
            // no resources were read under this, so just return the child
            if context.pending_resources.get() == 0 {
                child.clone()
            }
            // show the fallback, but also prepare to stream HTML
            else {
                let orig_child = Rc::clone(&orig_child);
                cx.register_suspense(context, &current_id.to_string(), {
                    let current_id = current_id.clone();
                    move || {
                        HydrationCtx::continue_from(current_id);
                        orig_child(cx)
                            .into_view(cx)
                            .render_to_string(cx)
                            .to_string()
                    }
                });
    
                // return the fallback for now, wrapped in fragment identifer
                fallback().into_view(cx)
            }
        };

        HydrationCtx::continue_from(current_id);

        initial
    })
}
