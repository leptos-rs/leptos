use crate as leptos;
use leptos_dom::{Child, IntoChild};
use leptos_macro::Props;
use leptos_reactive::{provide_context, Scope, SuspenseContext};

/// Props for the [Suspense](crate::Suspense) component, which shows a fallback
/// while [Resource](leptos_reactive::Resource)s are being read.
#[derive(Props)]
pub struct SuspenseProps<F, E, G>
where
    F: IntoChild + Clone,
    E: IntoChild,
    G: Fn() -> E,
{
    /// Will be displayed while resources are pending.
    pub fallback: F,
    /// Will be displayed once all resources have resolved.
    pub children: Box<dyn Fn() -> Vec<G>>,
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
/// # run_scope(|cx| {
/// # if cfg!(not(any(feature = "csr", feature = "hydrate", feature = "ssr"))) {
/// async fn fetch_cats(how_many: u32) -> Result<Vec<String>, ()> { Ok(vec![]) }
///
/// let (cat_count, set_cat_count) = create_signal::<u32>(cx, 1);
///
/// let cats = create_resource(cx, cat_count, |count| fetch_cats(count));
///
/// view! { cx,
///   <div>
///     <Suspense fallback={"Loading (Suspense Fallback)...".to_string()}>
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
pub fn Suspense<F, E, G>(cx: Scope, props: SuspenseProps<F, E, G>) -> impl Fn() -> Child
where
    F: IntoChild + Clone,
    E: IntoChild,
    G: Fn() -> E + 'static,
{
    let context = SuspenseContext::new(cx);

    // provide this SuspenseContext to any resources below it
    provide_context(cx, context.clone());

    let child = (props.children)().swap_remove(0);

    render_suspense(cx, context, props.fallback.clone(), child)
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
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
        #[cfg(feature = "transition")]
        let transition_pending = cx.transition_pending();

        #[cfg(not(feature = "transition"))]
        let transition_pending = false;

        if context.ready() || transition_pending {
            (child)().into_child(cx)
        } else {
            fallback.clone().into_child(cx)
        }
    }
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
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
    use leptos_dom::IntoAttribute;
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
            Child::Node(view! { cx, <div data-fragment-id=key>{fallback.into_child(cx)}</div> })
        }
    };
    move || initial.clone()
}
