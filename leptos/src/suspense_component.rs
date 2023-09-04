use leptos_dom::{DynChild, HydrationCtx, IntoView};
use leptos_macro::component;
#[cfg(any(feature = "csr", feature = "hydrate"))]
use leptos_reactive::SignalGet;
use leptos_reactive::{
    create_memo, provide_context, SignalGetUntracked, SuspenseContext,
};
#[cfg(not(any(feature = "csr", feature = "hydrate")))]
use leptos_reactive::{with_owner, Owner, SharedContext};
use std::rc::Rc;

/// If any [`Resource`](leptos_reactive::Resource) is read in the `children` of this
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
/// # let runtime = create_runtime();
/// async fn fetch_cats(how_many: u32) -> Option<Vec<String>> { Some(vec![]) }
///
/// let (cat_count, set_cat_count) = create_signal::<u32>(1);
///
/// let cats = create_resource(move || cat_count.get(), |count| fetch_cats(count));
///
/// view! {
///   <div>
///     <Suspense fallback=move || view! { <p>"Loading (Suspense Fallback)..."</p> }>
///       {move || {
///           cats.read().map(|data| match data {
///             None => view! {  <pre>"Error"</pre> }.into_view(),
///             Some(cats) => cats
///                 .iter()
///                 .map(|src| {
///                     view! {
///                       <img src={src}/>
///                     }
///                 })
///                 .collect_view(),
///           })
///         }
///       }
///     </Suspense>
///   </div>
/// };
/// # runtime.dispose();
/// # }
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "info", skip_all)
)]
#[component]
pub fn Suspense<F, E, V>(
    /// Returns a fallback UI that will be shown while `async` [`Resource`](leptos_reactive::Resource)s are still loading.
    fallback: F,
    /// Children will be displayed once all `async` [`Resource`](leptos_reactive::Resource)s have resolved.
    children: Box<dyn Fn() -> V>,
) -> impl IntoView
where
    F: Fn() -> E + 'static,
    E: IntoView,
    V: IntoView + 'static,
{
    let orig_children = Rc::new(children);
    let context = SuspenseContext::new();

    #[cfg(not(any(feature = "csr", feature = "hydrate")))]
    let owner =
        Owner::current().expect("<Suspense/> created with no reactive owner");

    // provide this SuspenseContext to any resources below it
    // run in a memo so the children are children of this parent
    let children = create_memo({
        let orig_children = Rc::clone(&orig_children);
        move |_| {
            provide_context(context);
            orig_children().into_view()
        }
    });

    // likewise for the fallback
    let fallback = create_memo({
        move |_| {
            provide_context(context);
            fallback().into_view()
        }
    });

    let current_id = HydrationCtx::next_component();

    #[cfg(any(feature = "csr", feature = "hydrate"))]
    let ready = context.ready();

    let child = DynChild::new({
        move || {
            // pull lazy memo before checking if context is ready
            let children_rendered = children.get_untracked();

            #[cfg(any(feature = "csr", feature = "hydrate"))]
            {
                if ready.get() {
                    children_rendered
                } else {
                    fallback.get_untracked()
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
                        with_owner(owner, move || {
                            //HydrationCtx::continue_from(current_id);
                            DynChild::new(move || children_rendered.clone())
                                .into_view()
                        })
                    }
                    // show the fallback, but also prepare to stream HTML
                    else {
                        HydrationCtx::continue_from(current_id);
                        let runtime = leptos_reactive::current_runtime();

                        SharedContext::register_suspense(
                            context,
                            &current_id.to_string(),
                            // out-of-order streaming
                            {
                                let orig_children = Rc::clone(&orig_children);
                                move || {
                                    leptos_reactive::set_current_runtime(
                                        runtime,
                                    );
                                    with_owner(owner, {
                                        move || {
                                            HydrationCtx::continue_from(
                                                current_id,
                                            );
                                            DynChild::new({
                                                move || {
                                                    orig_children().into_view()
                                                }
                                            })
                                            .into_view()
                                            .render_to_string()
                                            .to_string()
                                        }
                                    })
                                }
                            },
                            // in-order streaming
                            {
                                let orig_children = Rc::clone(&orig_children);
                                move || {
                                    leptos_reactive::set_current_runtime(
                                        runtime,
                                    );
                                    with_owner(owner, {
                                        move || {
                                            HydrationCtx::continue_from(
                                                current_id,
                                            );
                                            DynChild::new({
                                                move || {
                                                    orig_children().into_view()
                                                }
                                            })
                                            .into_view()
                                            .into_stream_chunks()
                                        }
                                    })
                                }
                            },
                        );

                        // return the fallback for now, wrapped in fragment identifier
                        fallback.get_untracked()
                    }
                }
            }
        }
    })
    .into_view();
    let core_component = match child {
        leptos_dom::View::CoreComponent(repr) => repr,
        _ => unreachable!(),
    };

    HydrationCtx::continue_from(current_id);
    HydrationCtx::next_component();

    leptos_dom::View::Suspense(current_id, core_component)
}
