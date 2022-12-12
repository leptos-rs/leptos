use leptos_reactive::Scope;

use crate::{ComponentRepr, IntoView, View};

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
pub struct Fragment(Vec<View>);

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
    Self(nodes)
  }

  /// Gives access to the [View] children contained within the fragment.
  pub fn as_children(&self) -> &[View] {
    &self.0
  }
}

impl IntoView for Fragment {
  #[cfg_attr(debug_assertions, instrument(level = "trace", name = "</>", skip_all, fields(children = self.0.len())))]
  fn into_view(self, cx: leptos_reactive::Scope) -> View {
    let mut frag = ComponentRepr::new("");

    frag.children = self.0;

    frag.into_view(cx)
  }
}
