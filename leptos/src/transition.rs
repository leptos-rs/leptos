use leptos_dom::{Fragment, IntoView, View};
use leptos_macro::component;
use leptos_reactive::{Scope, SignalSetter};
use std::{cell::RefCell, rc::Rc};

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
/// async fn fetch_cats(how_many: u32) -> Option<Vec<String>> { Some(vec![]) }
///
/// let (cat_count, set_cat_count) = create_signal::<u32>(cx, 1);
/// let (pending, set_pending) = create_signal(cx, false);
///
/// let cats = create_resource(cx, cat_count, |count| fetch_cats(count));
///
/// view! { cx,
///   <div>
///     <Transition
///       fallback=move || view! { cx, <p>"Loading..."</p>}
///       set_pending=set_pending.into()
///     >
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
///     </Transition>
///   </div>
/// };
/// # });
/// # }
/// ```
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
    let prev_children = std::rc::Rc::new(RefCell::new(None::<Vec<View>>));
    crate::Suspense(
        cx,
        crate::SuspenseProps::builder()
            .fallback({
                let prev_child = Rc::clone(&prev_children);
                move || {
                    if let Some(set_pending) = &set_pending {
                        set_pending.set(true);
                    }
                    if let Some(prev_children) = &*prev_child.borrow() {
                        prev_children.clone().into_view(cx)
                    } else {
                        fallback().into_view(cx)
                    }
                }
            })
            .children(Box::new(move |cx| {
                let frag = children(cx);
                *prev_children.borrow_mut() = Some(frag.nodes.clone());
                if let Some(set_pending) = &set_pending {
                    set_pending.set(false);
                }
                frag
            }))
            .build(),
    )
}
