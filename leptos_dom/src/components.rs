mod dyn_child;
mod each;
mod errors;
mod fragment;
mod unit;

use crate::{
    hydration::{HydrationCtx, HydrationKey},
    Comment, IntoView, View,
};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use crate::{mount_child, prepare_to_move, MountKind, Mountable};
pub use dyn_child::*;
pub use each::*;
pub use errors::*;
pub use fragment::*;
use leptos_reactive::Scope;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use once_cell::unsync::OnceCell;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use std::rc::Rc;
use std::{borrow::Cow, fmt};
pub use unit::*;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use wasm_bindgen::JsCast;

/// The core foundational leptos components.
#[derive(educe::Educe)]
#[educe(Default, Clone, PartialEq, Eq)]
pub enum CoreComponent {
    /// The [Unit] component.
    #[educe(Default)]
    Unit(UnitRepr),
    /// The [DynChild] component.
    DynChild(DynChildRepr),
    /// The [Each] component.
    Each(EachRepr),
}

impl fmt::Debug for CoreComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit(u) => u.fmt(f),
            Self::DynChild(dc) => dc.fmt(f),
            Self::Each(e) => e.fmt(f),
        }
    }
}

/// Custom leptos component.
#[derive(Clone, PartialEq, Eq)]
pub struct ComponentRepr {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    pub(crate) document_fragment: web_sys::DocumentFragment,
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    mounted: Rc<OnceCell<()>>,
    #[cfg(any(debug_assertions, feature = "ssr"))]
    pub(crate) name: Cow<'static, str>,
    #[cfg(debug_assertions)]
    _opening: Comment,
    /// The children of the component.
    pub children: Vec<View>,
    closing: Comment,
    #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
    pub(crate) id: HydrationKey,
    #[cfg(debug_assertions)]
    pub(crate) view_marker: Option<String>,
}

impl fmt::Debug for ComponentRepr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write;

        if self.children.is_empty() {
            #[cfg(debug_assertions)]
            return write!(f, "<{} />", self.name);

            #[cfg(not(debug_assertions))]
            return f.write_str("<Component />");
        } else {
            #[cfg(debug_assertions)]
            writeln!(f, "<{}>", self.name)?;
            #[cfg(not(debug_assertions))]
            f.write_str("<Component>")?;

            let mut pad_adapter = pad_adapter::PadAdapter::new(f);

            for child in &self.children {
                writeln!(pad_adapter, "{child:#?}")?;
            }

            #[cfg(debug_assertions)]
            write!(f, "</{}>", self.name)?;
            #[cfg(not(debug_assertions))]
            f.write_str("</Component>")?;

            Ok(())
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Mountable for ComponentRepr {
    fn get_mountable_node(&self) -> web_sys::Node {
        if self.mounted.get().is_none() {
            self.mounted.set(()).unwrap();

            self.document_fragment
                .unchecked_ref::<web_sys::Node>()
                .to_owned()
        }
        // We need to prepare all children to move
        else {
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
        return self._opening.node.clone();

        #[cfg(not(debug_assertions))]
        return if let Some(child) = self.children.get(0) {
            child.get_opening_node()
        } else {
            self.closing.node.clone()
        };
    }

    #[inline]
    fn get_closing_node(&self) -> web_sys::Node {
        self.closing.node.clone()
    }
}
impl From<ComponentRepr> for View {
    fn from(value: ComponentRepr) -> Self {
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        if !HydrationCtx::is_hydrating() {
            for child in &value.children {
                mount_child(MountKind::Before(&value.closing.node), child);
            }
        }

        View::Component(value)
    }
}

impl IntoView for ComponentRepr {
    #[cfg_attr(any(debug_assertions, feature = "ssr"), instrument(level = "info", name = "<Component />", skip_all, fields(name = %self.name)))]
    fn into_view(self, _: Scope) -> View {
        self.into()
    }
}

impl ComponentRepr {
    /// Creates a new [`Component`].
    #[inline(always)]
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self::new_with_id_concrete(name.into(), HydrationCtx::id())
    }

    /// Creates a new [`Component`] with the given hydration ID.
    #[inline(always)]
    pub fn new_with_id(
        name: impl Into<Cow<'static, str>>,
        id: HydrationKey,
    ) -> Self {
        Self::new_with_id_concrete(name.into(), id)
    }

    fn new_with_id_concrete(name: Cow<'static, str>, id: HydrationKey) -> Self {
        let markers = (
            Comment::new(Cow::Owned(format!("</{name}>")), &id, true),
            #[cfg(debug_assertions)]
            Comment::new(Cow::Owned(format!("<{name}>")), &id, false),
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
                    .expect("append to not err");
                #[cfg(not(debug_assertions))]
                fragment
                    .append_with_node_1(&markers.0.node)
                    .expect("append to not err");
            }

            fragment
        };

        Self {
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            document_fragment,
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            mounted: Default::default(),
            #[cfg(debug_assertions)]
            _opening: markers.1,
            closing: markers.0,
            #[cfg(any(debug_assertions, feature = "ssr"))]
            name,
            children: Vec::with_capacity(1),
            #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
            id,
            #[cfg(debug_assertions)]
            view_marker: None,
        }
    }

    #[cfg(any(debug_assertions, feature = "ssr"))]
    /// Returns the name of the component.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A user-defined `leptos` component.
pub struct Component<F, V>
where
    F: FnOnce(Scope) -> V,
    V: IntoView,
{
    id: HydrationKey,
    name: Cow<'static, str>,
    children_fn: F,
}

impl<F, V> Component<F, V>
where
    F: FnOnce(Scope) -> V,
    V: IntoView,
{
    /// Creates a new component.
    pub fn new(name: impl Into<Cow<'static, str>>, f: F) -> Self {
        Self {
            id: HydrationCtx::id(),
            name: name.into(),
            children_fn: f,
        }
    }
}

impl<F, V> IntoView for Component<F, V>
where
    F: FnOnce(Scope) -> V,
    V: IntoView,
{
    #[track_caller]
    fn into_view(self, cx: Scope) -> View {
        let Self {
            id,
            name,
            children_fn,
        } = self;

        let mut repr = ComponentRepr::new_with_id(name, id);

        // disposed automatically when the parent scope is disposed
        let (child, _) = cx.run_child_scope(|cx| {
            cx.untrack_with_diagnostics(|| children_fn(cx).into_view(cx))
        });

        repr.children.push(child);

        repr.into_view(cx)
    }
}
