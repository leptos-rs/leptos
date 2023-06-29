use crate::{
    hydration::HydrationKey, ComponentRepr, HydrationCtx, IntoView, View,
};
use leptos_reactive::Scope;

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
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", skip_all,)
    )]
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
    #[cfg(debug_assertions)]
    pub(crate) view_marker: Option<String>,
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

impl From<Fragment> for View {
    fn from(value: Fragment) -> Self {
        let mut frag = ComponentRepr::new_with_id("", value.id);

        #[cfg(debug_assertions)]
        {
            frag.view_marker = value.view_marker;
        }

        frag.children = value.nodes;

        frag.into()
    }
}

impl Fragment {
    /// Creates a new [`Fragment`] from a [`Vec<Node>`].
    #[inline(always)]
    pub fn new(nodes: Vec<View>) -> Self {
        Self::new_with_id(HydrationCtx::id(), nodes)
    }

    /// Creates a new [`Fragment`] from a function that returns [`Vec<Node>`].
    #[inline(always)]
    pub fn lazy(nodes: impl FnOnce() -> Vec<View>) -> Self {
        Self::new_with_id(HydrationCtx::id(), nodes())
    }

    /// Creates a new [`Fragment`] with the given hydration ID from a [`Vec<Node>`].
    #[inline(always)]
    pub const fn new_with_id(id: HydrationKey, nodes: Vec<View>) -> Self {
        Self {
            id,
            nodes,
            #[cfg(debug_assertions)]
            view_marker: None,
        }
    }

    /// Gives access to the [View] children contained within the fragment.
    #[inline(always)]
    pub fn as_children(&self) -> &[View] {
        &self.nodes
    }

    /// Returns the fragment's hydration ID.
    #[inline(always)]
    pub fn id(&self) -> &HydrationKey {
        &self.id
    }

    #[cfg(debug_assertions)]
    /// Adds an optional marker indicating the view macro source.
    pub fn with_view_marker(mut self, marker: impl Into<String>) -> Self {
        self.view_marker = Some(marker.into());
        self
    }
}

impl IntoView for Fragment {
    #[cfg_attr(debug_assertions, instrument(level = "info", name = "</>", skip_all, fields(children = self.nodes.len())))]
    fn into_view(self, _: leptos_reactive::Scope) -> View {
        self.into()
    }
}
