use crate::{children::TypedChildrenFn, mount, IntoView};
use leptos_dom::helpers::document;
use leptos_macro::component;
use reactive_graph::{effect::Effect, graph::untrack, owner::Owner};
use std::sync::Arc;

/// Renders components somewhere else in the DOM.
///
/// Useful for inserting modals and tooltips outside of a cropping layout.
/// If no mount point is given, the portal is inserted in `document.body`;
/// it is wrapped in a `<div>` unless  `is_svg` is `true` in which case it's wrappend in a `<g>`.
/// Setting `use_shadow` to `true` places the element in a shadow root to isolate styles.
#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip_all))]
#[component]
pub fn Portal<V>(
    /// Target element where the children will be appended
    #[prop(into, optional)]
    mount: Option<web_sys::Element>,
    /// Whether to use a shadow DOM inside `mount`. Defaults to `false`.
    #[prop(optional)]
    use_shadow: bool,
    /// When using SVG this has to be set to `true`. Defaults to `false`.
    #[prop(optional)]
    is_svg: bool,
    /// The children to teleport into the `mount` element
    children: TypedChildrenFn<V>,
) -> impl IntoView
where
    V: IntoView + 'static,
{
    if cfg!(target_arch = "wasm32")
        && Owner::current_shared_context()
            .map(|sc| sc.is_browser())
            .unwrap_or(true)
    {
        use send_wrapper::SendWrapper;
        use wasm_bindgen::JsCast;

        let mount = mount.unwrap_or_else(|| {
            document().body().expect("body to exist").unchecked_into()
        });
        let children = children.into_inner();

        Effect::new(move |_| {
            let tag = if is_svg { "g" } else { "div" };

            let container = document()
                .create_element(tag)
                .expect("element creation to work");

            let render_root = if use_shadow {
                container
                    .attach_shadow(&web_sys::ShadowRootInit::new(
                        web_sys::ShadowRootMode::Open,
                    ))
                    .map(|root| root.unchecked_into())
                    .unwrap_or(container.clone())
            } else {
                container.clone()
            };

            let _ = mount.append_child(&container);
            let handle = SendWrapper::new((
                mount::mount_to(render_root.unchecked_into(), {
                    let children = Arc::clone(&children);
                    move || untrack(|| children())
                }),
                mount.clone(),
                container,
            ));

            Owner::on_cleanup({
                move || {
                    let (handle, mount, container) = handle.take();
                    drop(handle);
                    let _ = mount.remove_child(&container);
                }
            })
        });
    }
}
