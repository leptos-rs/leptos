use crate::{hydration::HydrationCtx, ComponentRepr, IntoView, View};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use crate::{mount_child, MountKind};

/// Represents a group of [`views`](View).
#[derive(Debug)]
pub struct Fragment(Vec<View>);

impl Fragment {
  /// Creates a new [`Fragment`] from a [`Vec<Node>`].
  pub fn new(nodes: Vec<View>) -> Self {
    Self(nodes)
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
