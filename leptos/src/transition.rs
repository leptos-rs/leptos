use leptos::ViewFn;
use leptos_dom::{Fragment, HydrationCtx, IntoView, View};
use leptos_macro::component;
use leptos_reactive::{
    create_isomorphic_effect, create_rw_signal, use_context, RwSignal,
    SignalGet, SignalGetUntracked, SignalSet, SignalSetter, SuspenseContext,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// If any [`Resource`](leptos_reactive::Resource)s are read in the `children` of this
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
/// # let runtime = create_runtime();
/// async fn fetch_cats(how_many: u32) -> Option<Vec<String>> {
///     Some(vec![])
/// }
///
/// let (cat_count, set_cat_count) = create_signal::<u32>(1);
/// let (pending, set_pending) = create_signal(false);
///
/// let cats =
///     create_resource(move || cat_count.get(), |count| fetch_cats(count));
///
/// view! {
///   <div>
///     <Transition
///       fallback=move || view! {  <p>"Loading..."</p>}
///       set_pending
///     >
///       {move || {
///           cats.read().map(|data| match data {
///             None => view! { <pre>"Error"</pre> }.into_view(),
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
///     </Transition>
///   </div>
/// };
/// # runtime.dispose();
/// # }
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all)
)]
#[component(transparent)]
pub fn Transition(
    /// Will be displayed while resources are pending. By default this is the empty view.
    #[prop(optional, into)]
    fallback: ViewFn,
    /// A function that will be called when the component transitions into or out of
    /// the `pending` state, with its argument indicating whether it is pending (`true`)
    /// or not pending (`false`).
    #[prop(optional, into)]
    set_pending: Option<SignalSetter<bool>>,
    /// Will be displayed once all resources have resolved.
    children: Box<dyn Fn() -> Fragment>,
) -> impl IntoView {
    let prev_children = Rc::new(RefCell::new(None::<View>));

    let first_run = create_rw_signal(true);
    let child_runs = Cell::new(0);
    let held_suspense_context = Rc::new(RefCell::new(None::<SuspenseContext>));

    crate::Suspense(
        crate::SuspenseProps::builder()
            .fallback({
                let prev_child = Rc::clone(&prev_children);
                move || {
                    let suspense_context = use_context::<SuspenseContext>()
                        .expect("there to be a SuspenseContext");

                    let was_first_run =
                        cfg!(feature = "csr") && first_run.get();
                    let is_first_run =
                        is_first_run(first_run, &suspense_context);
                    if was_first_run {
                        first_run.set(false)
                    }

                    if let Some(prev_children) = &*prev_child.borrow() {
                        if is_first_run || was_first_run {
                            fallback.run()
                        } else {
                            prev_children.clone()
                        }
                    } else {
                        fallback.run()
                    }
                }
            })
            .children(Rc::new(move || {
                let frag = children().into_view();

                if let Some(suspense_context) = use_context::<SuspenseContext>()
                {
                    *held_suspense_context.borrow_mut() =
                        Some(suspense_context);
                }
                let suspense_context = held_suspense_context.borrow().unwrap();

                if cfg!(feature = "hydrate")
                    || !first_run.get_untracked()
                    || (cfg!(feature = "csr") && first_run.get())
                {
                    *prev_children.borrow_mut() = Some(frag.clone());
                }
                if is_first_run(first_run, &suspense_context) {
                    let has_local_only = suspense_context.has_local_only()
                        || cfg!(feature = "csr")
                        || !HydrationCtx::is_hydrating();
                    if (!has_local_only || child_runs.get() > 0)
                        && !cfg!(feature = "csr")
                    {
                        first_run.set(false);
                    }
                }
                child_runs.set(child_runs.get() + 1);

                create_isomorphic_effect(move |_| {
                    if let Some(set_pending) = set_pending {
                        set_pending.set(!suspense_context.none_pending())
                    }
                });
                frag
            }))
            .build(),
    )
}

fn is_first_run(
    first_run: RwSignal<bool>,
    suspense_context: &SuspenseContext,
) -> bool {
    if cfg!(feature = "csr")
        || (cfg!(feature = "hydrate") && !HydrationCtx::is_hydrating())
    {
        false
    } else {
        match (
            first_run.get_untracked(),
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
