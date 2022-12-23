use cfg_if::cfg_if;
use leptos_macro::component;
use std::rc::Rc;
use leptos_dom::{Fragment, HydrationCtx, IntoView, Component};
use leptos_reactive::{provide_context, Scope, SuspenseContext};

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
    let context = SuspenseContext::new(cx);

    // provide this SuspenseContext to any resources below it
    provide_context(cx, context);

    Component::new("Suspense", move |cx| {
        let current_id = HydrationCtx::peek();

        let orig_child = Rc::new(children);
    
        cfg_if! {
            if #[cfg(any(feature = "csr", feature = "hydrate"))] {
                if context.ready() {
                    HydrationCtx::continue_from(current_id);
                    orig_child(cx).into_view(cx)
                } else {
                    HydrationCtx::continue_from(current_id);
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
            }
        }
    })
}