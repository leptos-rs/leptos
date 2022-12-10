use crate::{hydration::HydrationCtx, Comment, IntoView, View};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use crate::{mount_child, MountKind, Mountable};
use leptos_reactive::Scope;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use leptos_reactive::{create_effect, ScopeDisposer};
use std::{borrow::Cow, cell::RefCell, rc::Rc};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use wasm_bindgen::JsCast;

/// The internal representation of the [`DynChild`] core-component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynChildRepr {
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  document_fragment: web_sys::DocumentFragment,
  #[cfg(debug_assertions)]
  opening: Comment,
  pub(crate) child: Rc<RefCell<Box<Option<View>>>>,
  closing: Comment,
  #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
  pub(crate) id: usize,
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Mountable for DynChildRepr {
  fn get_mountable_node(&self) -> web_sys::Node {
    self.document_fragment.clone().unchecked_into()
  }

  fn get_opening_node(&self) -> web_sys::Node {
    self
      .child
      .borrow()
      .as_ref()
      .as_ref()
      .unwrap()
      .get_opening_node()
  }
}

impl DynChildRepr {
  fn new() -> Self {
    let id = HydrationCtx::id();

    let markers = (
      Comment::new(Cow::Borrowed("</DynChild>"), id, true),
      #[cfg(debug_assertions)]
      Comment::new(Cow::Borrowed("<DynChild>"), id, false),
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
  child_fn: CF,
}

impl<CF, N> DynChild<CF, N>
where
  CF: Fn() -> N + 'static,
  N: IntoView,
{
  /// Creates a new dynamic child which will re-render whenever it's
  /// signal dependencies change.
  pub fn new(child_fn: CF) -> Self {
    Self { child_fn }
  }
}

impl<CF, N> IntoView for DynChild<CF, N>
where
  CF: Fn() -> N + 'static,
  N: IntoView,
{
  #[cfg_attr(
    debug_assertions,
    instrument(level = "trace", name = "<DynChild />", skip_all)
  )]
  fn into_view(self, cx: Scope) -> View {
    let Self { child_fn } = self;

    let component = DynChildRepr::new();

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let closing = component.closing.node.clone();

    let child = component.child.clone();

    #[cfg(all(debug_assertions, target_arch = "wasm32", feature = "web"))]
    let span = tracing::Span::current();

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    create_effect(
      cx,
      move |prev_run: Option<(Option<web_sys::Node>, ScopeDisposer)>| {
        #[cfg(debug_assertions)]
        let _guard = span.enter();
        #[cfg(debug_assertions)]
        let _guard = trace_span!("DynChild reactive").entered();

        let (new_child, disposer) =
          cx.run_child_scope(|cx| child_fn().into_view(cx));

        let mut child_borrow = child.borrow_mut();

        // Is this at least the second time we are loading a child?
        if let Some((prev_t, prev_disposer)) = prev_run {
          let child = child_borrow.take().unwrap();

          // Dispose of the scope
          prev_disposer.dispose();

          // If the previous child was a text node, we would like to
          // make use of it again if our current child is also a text
          // node
          if let Some(prev_t) = prev_t {
            // Here, our child is also a text node
            if let Some(new_t) = new_child.get_text() {
              prev_t
                .unchecked_ref::<web_sys::Text>()
                .set_data(&new_t.content);

              **child_borrow = Some(new_child);

              (Some(prev_t), disposer)
            }
            // Child is not a text node, so we can remove the previous
            // text node
            else {
              // Remove the text
              closing
                .previous_sibling()
                .unwrap()
                .unchecked_into::<web_sys::Element>()
                .remove();

              // Mount the new child, and we're done
              mount_child(MountKind::Before(&closing), &new_child);

              **child_borrow = Some(new_child);

              (None, disposer)
            }
          }
          // Otherwise, our child can still be a text node,
          // but we know the previous child was not, so no special
          // treatment here
          else {
            // Technically, I think this check shouldn't be necessary, but
            // I can imagine some edge case that the child changes while
            // hydration is ongoing
            if !HydrationCtx::is_hydrating() {
              // Remove the child
              let start = child.get_opening_node();
              let end = &closing;

              let mut sibling = start;

              while sibling != *end {
                let next_sibling = sibling.next_sibling().unwrap();

                sibling.unchecked_ref::<web_sys::Element>().remove();

                sibling = next_sibling;
              }

              // Mount the new child
              mount_child(MountKind::Before(&closing), &new_child);
            }

            // We want to reuse text nodes, so hold onto it if
            // our child is one
            let t = child.get_text().map(|t| t.node.clone());

            **child_borrow = Some(new_child);

            (t, disposer)
          }
        }
        // Otherwise, we know for sure this is our first time
        else {
          // We need to remove the text created from SSR
          if HydrationCtx::is_hydrating() && new_child.get_text().is_some() {
            let t = closing
              .previous_sibling()
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

            mount_child(MountKind::Before(&closing), &new_child);
          }

          // If we are not hydrating, we simply mount the child
          if !HydrationCtx::is_hydrating() {
            mount_child(MountKind::Before(&closing), &new_child);
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

    View::CoreComponent(crate::CoreComponent::DynChild(component))
  }
}
