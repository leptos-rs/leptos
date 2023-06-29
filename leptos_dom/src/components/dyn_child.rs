use crate::{
    hydration::{HydrationCtx, HydrationKey},
    Comment, IntoView, View,
};
use cfg_if::cfg_if;
use leptos_reactive::Scope;
use std::{borrow::Cow, cell::RefCell, fmt, ops::Deref, rc::Rc};
cfg_if! {
  if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
    use crate::{mount_child, prepare_to_move, unmount_child, MountKind, Mountable};
    use leptos_reactive::{create_effect, ScopeDisposer};
    use wasm_bindgen::JsCast;
  }
}

/// The internal representation of the [`DynChild`] core-component.
#[derive(Clone, PartialEq, Eq)]
pub struct DynChildRepr {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    document_fragment: web_sys::DocumentFragment,
    #[cfg(debug_assertions)]
    opening: Comment,
    pub(crate) child: Rc<RefCell<Box<Option<View>>>>,
    closing: Comment,
    #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
    pub(crate) id: HydrationKey,
}

impl fmt::Debug for DynChildRepr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write;

        f.write_str("<DynChild>\n")?;

        let mut pad_adapter = pad_adapter::PadAdapter::new(f);

        writeln!(
            pad_adapter,
            "{:#?}",
            self.child.borrow().deref().deref().as_ref().unwrap()
        )?;

        f.write_str("</DynChild>")
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Mountable for DynChildRepr {
    fn get_mountable_node(&self) -> web_sys::Node {
        if self.document_fragment.child_nodes().length() != 0 {
            self.document_fragment.clone().unchecked_into()
        } else {
            let opening = self.get_opening_node();

            prepare_to_move(
                &self.document_fragment,
                &opening,
                &self.closing.node,
            );

            self.document_fragment.clone().unchecked_into()
        }
    }

    fn get_opening_node(&self) -> web_sys::Node {
        #[cfg(debug_assertions)]
        return self.opening.node.clone();

        #[cfg(not(debug_assertions))]
        return self
            .child
            .borrow()
            .as_ref()
            .as_ref()
            .unwrap()
            .get_opening_node();
    }

    fn get_closing_node(&self) -> web_sys::Node {
        self.closing.node.clone()
    }
}

impl DynChildRepr {
    fn new_with_id(id: HydrationKey) -> Self {
        let markers = (
            Comment::new(Cow::Borrowed("</DynChild>"), &id, true),
            #[cfg(debug_assertions)]
            Comment::new(Cow::Borrowed("<DynChild>"), &id, false),
        );

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        let document_fragment = {
            let fragment = crate::document().create_document_fragment();

            // Insert the comments into the document fragment
            // so they can serve as our references when inserting
            // future nodes
            if !HydrationCtx::is_hydrating() {
                #[cfg(debug_assertions)]
                fragment
                    .append_with_node_2(&markers.1.node, &markers.0.node)
                    .unwrap();
                #[cfg(not(debug_assertions))]
                fragment.append_with_node_1(&markers.0.node).unwrap();
            }

            fragment
        };

        Self {
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            document_fragment,
            #[cfg(debug_assertions)]
            opening: markers.1,
            child: Default::default(),
            closing: markers.0,
            #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
            id,
        }
    }
}

/// Represents any [`View`] that can change over time.
pub struct DynChild<CF, N>
where
    CF: Fn() -> N + 'static,
    N: IntoView,
{
    id: crate::HydrationKey,
    child_fn: CF,
}

impl<CF, N> DynChild<CF, N>
where
    CF: Fn() -> N + 'static,
    N: IntoView,
{
    /// Creates a new dynamic child which will re-render whenever it's
    /// signal dependencies change.
    #[track_caller]
    #[inline(always)]
    pub fn new(child_fn: CF) -> Self {
        Self::new_with_id(HydrationCtx::id(), child_fn)
    }

    #[doc(hidden)]
    #[track_caller]
    #[inline(always)]
    pub const fn new_with_id(id: HydrationKey, child_fn: CF) -> Self {
        Self { id, child_fn }
    }
}

impl<CF, N> IntoView for DynChild<CF, N>
where
    CF: Fn() -> N + 'static,
    N: IntoView,
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", name = "<DynChild />", skip_all)
    )]
    #[inline]
    fn into_view(self, cx: Scope) -> View {
        // concrete inner function
        #[inline(never)]
        fn create_dyn_view(
            cx: Scope,
            component: DynChildRepr,
            child_fn: Box<dyn Fn() -> View>,
        ) -> DynChildRepr {
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            let closing = component.closing.node.clone();

            let child = component.child.clone();

            #[cfg(all(
                debug_assertions,
                target_arch = "wasm32",
                feature = "web"
            ))]
            let span = tracing::Span::current();

            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            create_effect(
                cx,
                move |prev_run: Option<(
                    Option<web_sys::Node>,
                    ScopeDisposer,
                )>| {
                    #[cfg(debug_assertions)]
                    let _guard = span.enter();

                    let (new_child, disposer) =
                        cx.run_child_scope(|cx| child_fn().into_view(cx));

                    let mut child_borrow = child.borrow_mut();

                    // Is this at least the second time we are loading a child?
                    if let Some((prev_t, prev_disposer)) = prev_run {
                        let child = child_borrow.take().unwrap();

                        // Dispose of the scope
                        prev_disposer.dispose();

                        // We need to know if our child wasn't moved elsewhere.
                        // If it was, `DynChild` no longer "owns" that child, and
                        // is therefore no longer sound to unmount it from the DOM
                        // or to reuse it in the case of a text node

                        // TODO check does this still detect moves correctly?
                        let was_child_moved = prev_t.is_none()
                            && child
                                .get_closing_node()
                                .next_non_view_marker_sibling()
                                .as_ref()
                                != Some(&closing);

                        // If the previous child was a text node, we would like to
                        // make use of it again if our current child is also a text
                        // node
                        let ret = if let Some(prev_t) = prev_t {
                            // Here, our child is also a text node
                            if let Some(new_t) = new_child.get_text() {
                                if !was_child_moved && child != new_child {
                                    prev_t
                                        .unchecked_ref::<web_sys::Text>()
                                        .set_data(&new_t.content);

                                    **child_borrow = Some(new_child);

                                    (Some(prev_t), disposer)
                                } else {
                                    mount_child(
                                        MountKind::Before(&closing),
                                        &new_child,
                                    );

                                    **child_borrow = Some(new_child.clone());

                                    (Some(new_t.node.clone()), disposer)
                                }
                            }
                            // Child is not a text node, so we can remove the previous
                            // text node
                            else {
                                if !was_child_moved && child != new_child {
                                    // Remove the text
                                    closing
                                        .previous_non_view_marker_sibling()
                                        .unwrap()
                                        .unchecked_into::<web_sys::Element>()
                                        .remove();
                                }

                                // Mount the new child, and we're done
                                mount_child(
                                    MountKind::Before(&closing),
                                    &new_child,
                                );

                                **child_borrow = Some(new_child);

                                (None, disposer)
                            }
                        }
                        // Otherwise, the new child can still be a text node,
                        // but we know the previous child was not, so no special
                        // treatment here
                        else {
                            // Technically, I think this check shouldn't be necessary, but
                            // I can imagine some edge case that the child changes while
                            // hydration is ongoing
                            if !HydrationCtx::is_hydrating() {
                                let same_child = child == new_child;
                                if !was_child_moved && !same_child {
                                    // Remove the child
                                    let start = child.get_opening_node();
                                    let end = &closing;

                                    match child {
                                        View::CoreComponent(
                                            crate::CoreComponent::DynChild(
                                                child,
                                            ),
                                        ) => {
                                            let start =
                                                child.get_opening_node();
                                            let end = child.closing.node;
                                            prepare_to_move(
                                                &child.document_fragment,
                                                &start,
                                                &end,
                                            );
                                        }
                                        View::Component(child) => {
                                            let start =
                                                child.get_opening_node();
                                            let end = child.closing.node;
                                            prepare_to_move(
                                                &child.document_fragment,
                                                &start,
                                                &end,
                                            );
                                        }
                                        _ => unmount_child(&start, end),
                                    }
                                }

                                // Mount the new child
                                // If it's the same child, don't re-mount
                                if !same_child {
                                    mount_child(
                                        MountKind::Before(&closing),
                                        &new_child,
                                    );
                                }
                            }

                            // We want to reuse text nodes, so hold onto it if
                            // our child is one
                            let t =
                                new_child.get_text().map(|t| t.node.clone());

                            **child_borrow = Some(new_child);

                            (t, disposer)
                        };

                        ret
                    }
                    // Otherwise, we know for sure this is our first time
                    else {
                        // We need to remove the text created from SSR
                        if HydrationCtx::is_hydrating()
                            && new_child.get_text().is_some()
                        {
                            let t = closing
                                .previous_non_view_marker_sibling()
                                .unwrap()
                                .unchecked_into::<web_sys::Element>();

                            // See note on ssr.rs when matching on `DynChild`
                            // for more details on why we need to do this for
                            // release
                            if !cfg!(debug_assertions) {
                                t.previous_sibling()
                                    .unwrap()
                                    .unchecked_into::<web_sys::Element>()
                                    .remove();
                            }

                            t.remove();

                            mount_child(
                                MountKind::Before(&closing),
                                &new_child,
                            );
                        }

                        // If we are not hydrating, we simply mount the child
                        if !HydrationCtx::is_hydrating() {
                            mount_child(
                                MountKind::Before(&closing),
                                &new_child,
                            );
                        }

                        // We want to update text nodes, rather than replace them, so
                        // make sure to hold onto the text node
                        let t = new_child.get_text().map(|t| t.node.clone());

                        **child_borrow = Some(new_child);

                        (t, disposer)
                    }
                },
            );

            #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
            {
                let new_child = child_fn().into_view(cx);

                **child.borrow_mut() = Some(new_child);
            }

            component
        }

        // monomorphized outer function
        let Self { id, child_fn } = self;

        let component = DynChildRepr::new_with_id(id);
        let component = create_dyn_view(
            cx,
            component,
            Box::new(move || child_fn().into_view(cx)),
        );

        View::CoreComponent(crate::CoreComponent::DynChild(component))
    }
}

cfg_if! {
    if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
        use web_sys::Node;

        pub(crate) trait NonViewMarkerSibling {
            fn next_non_view_marker_sibling(&self) -> Option<Node>;

            fn previous_non_view_marker_sibling(&self) -> Option<Node>;
        }

        impl NonViewMarkerSibling for web_sys::Node {
            #[cfg_attr(not(debug_assertions), inline(always))]
            fn next_non_view_marker_sibling(&self) -> Option<Node> {
                cfg_if! {
                    if #[cfg(debug_assertions)] {
                        self.next_sibling().and_then(|node| {
                            if node.text_content().unwrap_or_default().trim().starts_with("leptos-view") {
                                node.next_sibling()
                            } else {
                                Some(node)
                            }
                        })
                    } else {
                        self.next_sibling()
                    }
                }
            }

            #[cfg_attr(not(debug_assertions), inline(always))]
            fn previous_non_view_marker_sibling(&self) -> Option<Node> {
                cfg_if! {
                    if #[cfg(debug_assertions)] {
                        self.previous_sibling().and_then(|node| {
                            if node.text_content().unwrap_or_default().trim().starts_with("leptos-view") {
                                node.previous_sibling()
                            } else {
                                Some(node)
                            }
                        })
                    } else {
                        self.previous_sibling()
                    }
                }
            }
        }
    }
}
