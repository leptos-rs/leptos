use leptos_dom::{DynChild, HydrationCtx, IntoView};
use leptos_macro::component;
use leptos_reactive::{provide_context, Scope, SuspenseContext};
use std::rc::Rc;

/// If any [Resources](leptos_reactive::Resource) are read in the `children` of this
/// component, it will show the `fallback` while they are loading. Once all are resolved,
/// it will render the `children`.
///
/// Note that the `children` will be rendered initially (in order to capture the fact that
/// those resources are read under the suspense), so you cannot assume that resources have
/// `Some` value in `children`.
///
/// ```
/// # use leptos_reactive::*;
/// # use leptos_macro::*;
/// # use leptos_dom::*; use leptos::*;
/// # if false {
/// # run_scope(create_runtime(), |cx| {
/// async fn fetch_cats(how_many: u32) -> Option<Vec<String>> { Some(vec![]) }
///
/// let (cat_count, set_cat_count) = create_signal::<u32>(cx, 1);
///
/// let cats = create_resource(cx, move || cat_count.get(), |count| fetch_cats(count));
///
/// view! { cx,
///   <div>
///     <Suspense fallback=move || view! { cx, <p>"Loading (Suspense Fallback)..."</p> }>
///       {move || {
///           cats.read(cx).map(|data| match data {
///             None => view! { cx,  <pre>"Error"</pre> }.into_view(cx),
///             Some(cats) => cats
///                 .iter()
///                 .map(|src| {
///                     view! { cx,
///                       <img src={src}/>
///                     }
///                 })
///                 .collect_view(cx),
///           })
///         }
///       }
///     </Suspense>
///   </div>
/// };
/// # });
/// # }
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "info", skip_all)
)]
#[component(transparent)]
pub fn Suspense<F, E, V>(
    cx: Scope,
    /// Returns a fallback UI that will be shown while `async` [Resources](leptos_reactive::Resource) are still loading.
    fallback: F,
    /// Children will be displayed once all `async` [Resources](leptos_reactive::Resource) have resolved.
    children: Box<dyn Fn(Scope) -> V>,
) -> impl IntoView
where
    F: Fn() -> E + 'static,
    E: IntoView,
    V: IntoView + 'static,
{
    let orig_children = Rc::new(children);
    let context = SuspenseContext::new(cx);

    // provide this SuspenseContext to any resources below it
    provide_context(cx, context);

    let current_id = HydrationCtx::next_component();

    let child = DynChild::new({
        #[cfg(not(any(feature = "csr", feature = "hydrate")))]
        let current_id = current_id;

        let children = Rc::new(orig_children(cx).into_view(cx));
        #[cfg(not(any(feature = "csr", feature = "hydrate")))]
        let orig_children = Rc::clone(&orig_children);
        move || {
            #[cfg(any(feature = "csr", feature = "hydrate"))]
            {
                if context.ready() {
                    (*children).clone()
                } else {
                    fallback().into_view(cx)
                }
            }
            #[cfg(not(any(feature = "csr", feature = "hydrate")))]
            {
                use leptos_reactive::signal_prelude::*;

                // run the child; we'll probably throw this away, but it will register resource reads
                //let after_original_child = HydrationCtx::peek();

                {
                    // no resources were read under this, so just return the child
                    if context.pending_resources.get() == 0 {
                        HydrationCtx::continue_from(current_id);
                        DynChild::new({
                            let children = Rc::clone(&children);
                            move || (*children).clone()
                        })
                        .into_view(cx)
                    }
                    // show the fallback, but also prepare to stream HTML
                    else {
                        HydrationCtx::continue_from(current_id);

                        cx.register_suspense(
                            context,
                            &current_id.to_string(),
                            // out-of-order streaming
                            {
                                let orig_children = Rc::clone(&orig_children);
                                move || {
                                    HydrationCtx::continue_from(current_id);
                                    DynChild::new({
                                        let orig_children =
                                            orig_children(cx).into_view(cx);
                                        move || orig_children.clone()
                                    })
                                    .into_view(cx)
                                    .render_to_string(cx)
                                    .to_string()
                                }
                            },
                            // in-order streaming
                            {
                                let orig_children = Rc::clone(&orig_children);
                                move || {
                                    HydrationCtx::continue_from(current_id);
                                    DynChild::new({
                                        let orig_children =
                                            orig_children(cx).into_view(cx);
                                        move || orig_children.clone()
                                    })
                                    .into_view(cx)
                                    .into_stream_chunks(cx)
                                }
                            },
                        );

                        // return the fallback for now, wrapped in fragment identifier
                        fallback().into_view(cx)
                    }
                }
            }
        }
    })
    .into_view(cx);
    let core_component = match child {
        leptos_dom::View::CoreComponent(repr) => repr,
        _ => unreachable!(),
    };

    HydrationCtx::continue_from(current_id);
    HydrationCtx::next_component();

    leptos_dom::View::Suspense(current_id, core_component)
}
