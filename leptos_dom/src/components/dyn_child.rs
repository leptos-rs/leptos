#[cfg(all(target_arch = "wasm32", feature = "web"))]
use crate::{mount_child, MountKind};
use crate::{Comment, IntoNode, Mountable, Node};
use leptos_reactive::{create_effect, Scope};
use std::{borrow::Cow, cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;

/// The internal representation of the [`DynChild`] core-component.
#[derive(Debug)]
pub struct DynChildRepr {
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  document_fragment: web_sys::DocumentFragment,
  #[cfg(debug_assertions)]
  opening: Comment,
  child: Rc<RefCell<Box<Option<Node>>>>,
  closing: Comment,
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
    let markers = (
      Comment::new(Cow::Borrowed("</DynChild>")),
      #[cfg(debug_assertions)]
      Comment::new(Cow::Borrowed("<DynChild>")),
    );

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let document_fragment = {
      let fragment = crate::document().create_document_fragment();

      // Insert the comments into the document fragment
      // so they can serve as our references when inserting
      // future nodes
      #[cfg(debug_assertions)]
      fragment
        .append_with_node_2(&markers.1.node, &markers.0.node)
        .unwrap();
      #[cfg(not(debug_assertions))]
      fragment.append_with_node_1(&markers.0.node).unwrap();

      fragment
    };

    Self {
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      document_fragment,
      #[cfg(debug_assertions)]
      opening: markers.1,
      child: Default::default(),
      closing: markers.0,
    }
  }
}

/// Represents any [`Node`] that can change over time.
pub struct DynChild<CF, N>
where
  CF: Fn() -> N + 'static,
  N: IntoNode,
{
  child_fn: CF,
}

impl<CF, N> DynChild<CF, N>
where
  CF: Fn() -> N + 'static,
  N: IntoNode,
{
  /// Creates a new dynamic child which will re-render whenever it's
  /// signal dependencies change.
  pub fn new(child_fn: CF) -> Self {
    Self { child_fn }
  }
}

impl<CF, N> IntoNode for DynChild<CF, N>
where
  CF: Fn() -> N + 'static,
  N: IntoNode,
{
  #[cfg_attr(
    debug_assertions,
    instrument(level = "trace", name = "<DynChild />", skip_all)
  )]
  fn into_node(self, cx: Scope) -> crate::Node {
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

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let prev_text_node = RefCell::new(None::<web_sys::Node>);

    let span = tracing::Span::current();

    create_effect(cx, move |prev_run| {
      let _guard = span.enter();
      let _guard = trace_span!("DynChild reactive").entered();

      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      if prev_run.is_some() {
        let opening =
          child.borrow().as_ref().as_ref().unwrap().get_opening_node();

        let mut sibling = opening;

        while sibling != closing {
          let next_sibling = sibling.next_sibling().unwrap();

          sibling.unchecked_ref::<web_sys::Element>().remove();

          sibling = next_sibling;
        }
      }

      let new_child = child_fn().into_node(cx);

      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      if let Some(t) = new_child.get_text() {
        let mut prev_text_node_borrow = prev_text_node.borrow_mut();

        if let Some(prev_t) = &*prev_text_node_borrow {
          prev_t.set_text_content(Some(&t.content));

          t.node.set(prev_t.clone()).unwrap();
        } else {
          t.fill_node();

          *prev_text_node_borrow = Some(t.node.get().unwrap().clone());
        }
      } else {
        *prev_text_node.borrow_mut() = None;
      }

      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      mount_child(MountKind::Before(&closing), &new_child);

      **child.borrow_mut() = Some(new_child);
    });

    Node::CoreComponent(crate::CoreComponent::DynChild(component))
  }
}
