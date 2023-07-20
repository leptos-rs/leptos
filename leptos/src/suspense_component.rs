use crate::ChildrenFn;
use leptos_dom::{DynChild, HydrationCtx, IntoView};
use leptos_macro::component;
use leptos_reactive::{
    create_memo, provide_context, with_owner, Owner, SharedContext,
    SignalGetUntracked, SuspenseContext,
};
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
/// # run_scope(create_runtime(), || {
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
/// # });
/// # }
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "info", skip_all)
)]
#[component]
pub fn Suspense<F, E, V>(
    /// Returns a fallback UI that will be shown while `async` [Resources](leptos_reactive::Resource) are still loading.
    fallback: F,
    /// Children will be displayed once all `async` [Resources](leptos_reactive::Resource) have resolved.
    children: Box<dyn Fn() -> V>,
) -> impl IntoView
where
    F: Fn() -> E + 'static,
    E: IntoView,
    V: IntoView + 'static,
{
    let orig_children = Rc::new(children);
    let context = SuspenseContext::new();
    let owner =
        Owner::current().expect("<Suspense/> created with no reactive owner");

    // provide this SuspenseContext to any resources below it
    let children = create_memo({
        let orig_children = Rc::clone(&orig_children);
        move |_| {
            eprintln!("<Suspense/> children with owner = {owner:?}");
            provide_context(context);
            orig_children().into_view()
        }
    });

    let current_id = HydrationCtx::next_component();
    eprintln!("\n\ninvoking <Suspense/> at {current_id}");

    let child = DynChild::new({
        #[cfg(not(any(feature = "csr", feature = "hydrate")))]
        let current_id = current_id;

        move || {
            // pull lazy memo before checking if context is ready
            let children_rendered = children.get_untracked();

            #[cfg(any(feature = "csr", feature = "hydrate"))]
            {
                if context.ready() {
                    children_rendered
                } else {
                    fallback().into_view()
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
                        eprintln!("no resources read under <Suspense/>");
                        with_owner(owner, move || {
                            //HydrationCtx::continue_from(current_id);
                            DynChild::new({ move || children_rendered.clone() })
                                .into_view()
                        })
                    }
                    // show the fallback, but also prepare to stream HTML
                    else {
                        HydrationCtx::continue_from(current_id);

                        with_owner(owner, {
                            let orig_children = Rc::clone(&orig_children);
                            move || {
                                SharedContext::register_suspense(
                                    context,
                                    &current_id.to_string(),
                                    // out-of-order streaming
                                    {
                                        let orig_children =
                                            Rc::clone(&orig_children);
                                        move || {
                                            HydrationCtx::continue_from(
                                                current_id,
                                            );
                                            DynChild::new({
                                                move || {
                                                    eprintln!(
                                                        "\n\n**calling \
                                                         orig_children again**"
                                                    );
                                                    orig_children().into_view()
                                                }
                                            })
                                            .into_view()
                                            .render_to_string()
                                            .to_string()
                                        }
                                    },
                                    // in-order streaming
                                    {
                                        let orig_children =
                                            Rc::clone(&orig_children);
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
                                    },
                                );
                            }
                        });

                        // return the fallback for now, wrapped in fragment identifier
                        fallback().into_view()
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

    //HydrationCtx::continue_from(current_id);
    HydrationCtx::next_component();

    leptos_dom::View::Suspense(current_id, core_component)
}
