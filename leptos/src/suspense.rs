use cfg_if::cfg_if;
use leptos_dom::{Component, DynChild, Fragment, IntoView};
#[cfg(not(any(feature = "csr", feature = "hydrate")))]
use leptos_dom::{HydrationCtx, HydrationKey};
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
/// let cats = create_resource(cx, cat_count, |count| fetch_cats(count));
///
/// view! { cx,
///   <div>
///     <Suspense fallback=move || view! { cx, <p>"Loading (Suspense Fallback)..."</p> }>
///       {move || {
///           cats.read().map(|data| match data {
///             None => view! { cx,  <pre>"Error"</pre> }.into_any(),
///             Some(cats) => view! { cx,
///               <div>{
///                 cats.iter()
///                   .map(|src| {
///                     view! { cx,
///                       <img src={src}/>
///                     }
///                   })
///                   .collect::<Vec<_>>()
///               }</div>
///             }.into_any(),
///           })
///         }
///       }
///     </Suspense>
///   </div>
/// };
/// # });
/// # }
/// ```
#[component(transparent)]
pub fn Suspense<F, E>(
    cx: Scope,
    /// Returns a fallback UI that will be shown while `async` [Resources](leptos_reactive::Resource) are still loading.
    fallback: F,
    /// Children will be displayed once all `async` [Resources](leptos_reactive::Resource) have resolved.
    children: Box<dyn Fn(Scope) -> Fragment>,
) -> impl IntoView
where
    F: Fn() -> E + 'static,
    E: IntoView,
{
    #[cfg(not(any(feature = "csr", feature = "hydrate")))]
    let id_before_suspense = HydrationCtx::peek();
    let context = SuspenseContext::new(cx);

    // provide this SuspenseContext to any resources below it
    provide_context(cx, context);

    let orig_child = Rc::new(children);

    Component::new("Suspense", move |cx| {
        #[cfg(not(any(feature = "csr", feature = "hydrate")))]
        let current_id = HydrationCtx::peek();

        DynChild::new(move || {
            cfg_if! {
                if #[cfg(any(feature = "csr", feature = "hydrate"))] {
                    if context.ready() {
                        orig_child(cx).into_view(cx)
                    } else {
                        fallback().into_view(cx)
                    }
                } else {
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

                            cx.register_suspense(
                                context,
                                &id_before_suspense.to_string(),
                                &current_id.to_string(),
                                {
                                    let current_id = current_id.clone();
                                    let fragment_id = HydrationKey {
                                        previous: current_id.previous,
                                        offset: current_id.offset + 1
                                    };
                                    move || {
                                        HydrationCtx::continue_from(fragment_id);
                                        orig_child(cx)
                                            .into_view(cx)
                                            .render_to_string(cx)
                                            .to_string()
                                    }
                                }
                            );

                            // return the fallback for now, wrapped in fragment identifer
                            fallback().into_view(cx)
                        }
                    };

                    HydrationCtx::continue_from(current_id.clone());

                    initial
                }
            }
        })
    })
}
