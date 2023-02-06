use leptos_reactive::Scope;

use crate::{
  hydration::HydrationKey, ComponentRepr, HydrationCtx, IntoView, View,
};

/// Trait for converting any iterable into a [`Fragment`].
pub trait IntoFragment {
  /// Consumes this type, returning [`Fragment`].
  fn into_fragment(self, cx: Scope) -> Fragment;
}

impl<I, V> IntoFragment for I
where
  I: IntoIterator<Item = V>,
  V: IntoView,
{
  fn into_fragment(self, cx: Scope) -> Fragment {
    self.into_iter().map(|v| v.into_view(cx)).collect()
  }
}

/// Represents a group of [`views`](View).
#[derive(Debug, Clone)]
pub struct Fragment {
  id: HydrationKey,
  /// The nodes contained in the fragment.
  pub nodes: Vec<View>,
}

impl FromIterator<View> for Fragment {
  fn from_iter<T: IntoIterator<Item = View>>(iter: T) -> Self {
    Fragment::new(iter.into_iter().collect())
  }
}

impl From<View> for Fragment {
  fn from(view: View) -> Self {
    Fragment::new(vec![view])
  }
}

impl Fragment {
  /// Creates a new [`Fragment`] from a [`Vec<Node>`].
  pub fn new(nodes: Vec<View>) -> Self {
    Self::new_with_id(HydrationCtx::id(), nodes)
  }

  /// Creates a new [`Fragment`] from a function that returns [`Vec<Node>`].
  pub fn lazy(nodes: impl FnOnce() -> Vec<View>) -> Self {
    Self::new_with_id(HydrationCtx::id(), nodes())
  }

  /// Creates a new [`Fragment`] with the given hydration ID from a [`Vec<Node>`].
  pub fn new_with_id(id: HydrationKey, nodes: Vec<View>) -> Self {
    Self { id, nodes }
  }

  /// Gives access to the [View] children contained within the fragment.
  pub fn as_children(&self) -> &[View] {
    &self.nodes
  }

  /// Returns the fragment's hydration ID.
  pub fn id(&self) -> &HydrationKey {
    &self.id
  }
}

impl IntoView for Fragment {
  #[cfg_attr(debug_assertions, instrument(level = "trace", name = "</>", skip_all, fields(children = self.nodes.len())))]
  fn into_view(self, cx: leptos_reactive::Scope) -> View {
    let mut frag = ComponentRepr::new_with_id("", self.id.clone());

    frag.children = self.nodes;

    frag.into_view(cx)
  }
}
