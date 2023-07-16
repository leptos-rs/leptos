use leptos_dom::{Fragment, HydrationCtx, IntoView, View};
use leptos_macro::component;
use leptos_reactive::{
    create_isomorphic_effect, use_context, Scope, SignalGet, SignalSetter,
    SuspenseContext,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

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
/// # use leptos_macro::*;
/// # use leptos_dom::*;
/// # use leptos::*;
/// # if false {
/// # run_scope(create_runtime(), |cx| {
/// async fn fetch_cats(how_many: u32) -> Option<Vec<String>> {
///     Some(vec![])
/// }
///
/// let (cat_count, set_cat_count) = create_signal::<u32>(cx, 1);
/// let (pending, set_pending) = create_signal(cx, false);
///
/// let cats =
///     create_resource(cx, move || cat_count.get(), |count| fetch_cats(count));
///
/// view! { cx,
///   <div>
///     <Transition
///       fallback=move || view! { cx, <p>"Loading..."</p>}
///       set_pending=set_pending.into()
///     >
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
///     </Transition>
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
pub fn Transition<F, E>(
    cx: Scope,
    /// Will be displayed while resources are pending.
    fallback: F,
    /// A function that will be called when the component transitions into or out of
    /// the `pending` state, with its argument indicating whether it is pending (`true`)
    /// or not pending (`false`).
    #[prop(optional)]
    set_pending: Option<SignalSetter<bool>>,
    /// Will be displayed once all resources have resolved.
    children: Box<dyn Fn(Scope) -> Fragment>,
) -> impl IntoView
where
    F: Fn() -> E + 'static,
    E: IntoView,
{
    let prev_children = Rc::new(RefCell::new(None::<View>));

    let first_run = Rc::new(std::cell::Cell::new(true));
    let child_runs = Cell::new(0);

    crate::Suspense(
        cx,
        crate::SuspenseProps::builder()
            .fallback({
                let prev_child = Rc::clone(&prev_children);
                let first_run = Rc::clone(&first_run);
                move || {
                    let suspense_context = use_context::<SuspenseContext>(cx)
                        .expect("there to be a SuspenseContext");

                    let is_first_run =
                        is_first_run(&first_run, &suspense_context);
                    first_run.set(false);

                    if let Some(prev_children) = &*prev_child.borrow() {
                        if is_first_run {
                            fallback().into_view(cx)
                        } else {
                            prev_children.clone()
                        }
                    } else {
                        fallback().into_view(cx)
                    }
                }
            })
            .children(Box::new(move |cx| {
                let frag = children(cx).into_view(cx);

                let suspense_context = use_context::<SuspenseContext>(cx)
                    .expect("there to be a SuspenseContext");

                if cfg!(feature = "hydrate")
                    || !first_run.get()
                    || (cfg!(feature = "csr") && first_run.get())
                {
                    *prev_children.borrow_mut() = Some(frag.clone());
                }
                if is_first_run(&first_run, &suspense_context) {
                    let has_local_only = suspense_context.has_local_only()
                        || cfg!(feature = "csr");
                    if (!has_local_only || child_runs.get() > 0)
                        && (cfg!(feature = "csr")
                            || HydrationCtx::is_hydrating())
                    {
                        first_run.set(false);
                    }
                }
                child_runs.set(child_runs.get() + 1);

                let pending = suspense_context.pending_resources;
                create_isomorphic_effect(cx, move |_| {
                    if let Some(set_pending) = set_pending {
                        set_pending.set(pending.get() > 0)
                    }
                });
                frag
            }))
            .build(),
    )
}

fn is_first_run(
    first_run: &Rc<Cell<bool>>,
    suspense_context: &SuspenseContext,
) -> bool {
    if cfg!(feature = "csr") {
        false
    } else {
        match (
            first_run.get(),
            cfg!(feature = "hydrate"),
            suspense_context.has_local_only(),
        ) {
            (false, _, _) => false,
            // SSR and has non-local resources (so, has streamed)
            (_, false, false) => false,
            // SSR but with only local resources (so, has not streamed)
            (_, false, true) => true,
            // hydrate: it's the first run
            (first_run, true, _) => HydrationCtx::is_hydrating() || first_run,
        }
    }
}
