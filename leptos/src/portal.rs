use crate::ChildrenFn;
use cfg_if::cfg_if;
use leptos_dom::IntoView;
use leptos_macro::component;
#[cfg(all(
    target_arch = "wasm32",
    any(feature = "hydrate", feature = "csr")
))]
use leptos_reactive::untrack;

/// Renders components somewhere else in the DOM.
///
/// Useful for inserting modals and tooltips outside of a cropping layout.
/// If no mount point is given, the portal is inserted in `document.body`;
/// it is wrapped in a `<div>` unless  `is_svg` is `true` in which case it's wrappend in a `<g>`.
/// Setting `use_shadow` to `true` places the element in a shadow root to isolate styles.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all)
)]
#[component]
pub fn Portal(
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
    children: ChildrenFn,
) -> impl IntoView {
    cfg_if! { if #[cfg(all(target_arch = "wasm32", any(feature = "hydrate", feature = "csr")))] {
        use leptos_dom::{document, Mountable};
        use leptos_reactive::{create_effect, on_cleanup};
        use wasm_bindgen::JsCast;

        let mount = mount
            .unwrap_or_else(|| document().body().expect("body to exist").unchecked_into());

        create_effect(move |_| {
            leptos::logging::log!("inside Portal effect");
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

            let children = untrack(|| children().into_view().get_mountable_node());
            let _ = render_root.append_child(&children);

            let _ = mount.append_child(&container);

            on_cleanup({
                let mount = mount.clone();

                move || {
                    let _ = mount.remove_child(&container);
                }
            })
        });
    } else {
        let _ = mount;
        let _ = use_shadow;
        let _ = is_svg;
        let _ = children;
    }}
}
