use crate::{ComponentRepr, IntoView, View};

/// Represents a group of [`views`](View).
#[derive(Debug, Clone)]
pub struct Fragment(Vec<View>);

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
