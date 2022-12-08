use crate::{hydration::HydrationCtx, Comment, IntoView, View};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use crate::{mount_child, MountKind, Mountable};
use leptos_reactive::{create_effect, Scope, ScopeDisposer};
use std::{borrow::Cow, cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;

/// The internal representation of the [`DynChild`] core-component.
#[derive(Debug)]
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
  fn into_view(self, cx: Scope) -> crate::View {
    let Self { child_fn } = self;

    let component = DynChildRepr::new();

    #[cfg(all(debug_assertions, target_arch = "wasm32", feature = "web"))]
    let (opening, closing) = (
      component.opening.node.clone(),
      component.closing.node.clone(),
    );
    #[cfg(all(not(debug_assertions), target_arch = "wasm32", feature = "web"))]
    let closing = component.closing.node.clone();

    let child = component.child.clone();

    let span = tracing::Span::current();

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    create_effect(
      cx,
      move |prev_run: Option<(Option<web_sys::Node>, ScopeDisposer)>| {
        let _guard = span.enter();
        let _guard = trace_span!("DynChild reactive").entered();

        let (new_child, disposer) =
          cx.run_child_scope(|cx| child_fn().into_view(cx));

        let mut child_borrow = child.borrow_mut();

        if let Some((prev_t, prev_disposer)) = prev_run {
          let child = child_borrow.take().unwrap();

          prev_disposer.dispose();

          if let Some(prev_t) = prev_t {
            if let Some(new_t) = new_child.get_text() {
              prev_t
                .unchecked_ref::<web_sys::Text>()
                .set_data(&new_t.content);

              **child_borrow = Some(new_child);

              (Some(prev_t), disposer)
            } else {
              // Remove the text
              closing
                .previous_sibling()
                .unwrap()
                .unchecked_into::<web_sys::Element>()
                .remove();

              mount_child(MountKind::Before(&closing), &new_child);

              **child_borrow = Some(new_child);

              (None, disposer)
            }
          } else {
            if !HydrationCtx::is_hydrating() {
              // Remove the child
              let child = child_borrow.take().unwrap();

              let start = child.get_opening_node();
              let end = &closing;

              let mut sibling = start.clone();

              while sibling != *end {
                let next_sibling = sibling.next_sibling().unwrap();

                sibling.unchecked_ref::<web_sys::Element>().remove();

                sibling = next_sibling;
              }

              mount_child(MountKind::Before(&closing), &child);
            }

            let t = child.get_text().map(|t| t.node.clone());

            **child_borrow = Some(child);

            (t, disposer)
          }
        } else {
          // We need to reuse the text created from SSR
          if HydrationCtx::is_hydrating() && new_child.get_text().is_some() {
            closing
              .previous_sibling()
              .unwrap()
              .unchecked_into::<web_sys::Element>()
              .remove();

            mount_child(MountKind::Before(&closing), &new_child);
          }

          if !HydrationCtx::is_hydrating() {
            mount_child(MountKind::Before(&closing), &new_child);
          }

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
