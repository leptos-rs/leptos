use cfg_if::cfg_if;
use leptos_dom::{DynChild, Fragment, HydrationCtx, IntoView};
use leptos_macro::component;
#[cfg(any(feature = "csr", feature = "hydrate"))]
use leptos_reactive::ScopeDisposer;
use leptos_reactive::{provide_context, Scope, SuspenseContext};
#[cfg(any(feature = "csr", feature = "hydrate"))]
use std::cell::RefCell;
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

    let orig_child = Rc::new(children);

    let before_me = HydrationCtx::peek();
    let current_id = HydrationCtx::next_component();
    #[cfg(any(feature = "csr", feature = "hydrate"))]
    let prev_disposer = Rc::new(RefCell::new(None::<ScopeDisposer>));

    let child = DynChild::new({
        #[cfg(not(any(feature = "csr", feature = "hydrate")))]
        let current_id = current_id.clone();
        move || {
            cfg_if! {
                if #[cfg(any(feature = "csr", feature = "hydrate"))] {
                    if let Some(disposer) = prev_disposer.take() {
                        disposer.dispose();
                    }
                    let (view, disposer) =
                        cx.run_child_scope(|cx| if context.ready() {
                        Fragment::lazy(Box::new(|| vec![orig_child(cx).into_view(cx)])).into_view(cx)
                    } else {
                        Fragment::lazy(Box::new(|| vec![fallback().into_view(cx)])).into_view(cx)
                    });
                    *prev_disposer.borrow_mut() = Some(disposer);
                    view

                } else {
                    use leptos_reactive::signal_prelude::*;

                    // run the child; we'll probably throw this away, but it will register resource reads
                    let child = orig_child(cx).into_view(cx);
                    let after_original_child = HydrationCtx::id();

                    let initial = {
                        // no resources were read under this, so just return the child
                        if context.pending_resources.get() == 0 {
                            child
                        }
                        // show the fallback, but also prepare to stream HTML
                        else {
                            let orig_child = Rc::clone(&orig_child);

                            cx.register_suspense(
                                context,
                                &current_id.to_string(),
                                // out-of-order streaming
                                {
                                    let current_id = current_id.clone();
                                    let orig_child = Rc::clone(&orig_child);
                                    move || {
                                        HydrationCtx::continue_from(current_id.clone());
                                        Fragment::lazy(Box::new(move || {
                                            vec![DynChild::new(move || orig_child(cx)).into_view(cx)]
                                        }))
                                        .into_view(cx)
                                        .render_to_string(cx)
                                        .to_string()
                                    }
                                },
                                // in-order streaming
                                {
                                    let current_id = current_id.clone();
                                    move || {
                                        HydrationCtx::continue_from(current_id.clone());
                                        Fragment::lazy(Box::new(move || {
                                            vec![DynChild::new(move || orig_child(cx)).into_view(cx)]
                                        }))
                                        .into_view(cx)
                                        .into_stream_chunks(cx)
                                    }
                                },
                            );

                            // return the fallback for now, wrapped in fragment identifier
                            fallback().into_view(cx)
                        }
                    };

                    HydrationCtx::continue_from(after_original_child);
                    initial
                }
            }
        }
    })
    .into_view(cx);
    let core_component = match child {
        leptos_dom::View::CoreComponent(repr) => repr,
        _ => unreachable!(),
    };

    HydrationCtx::continue_from(before_me);

    leptos_dom::View::Suspense(current_id, core_component)
}
