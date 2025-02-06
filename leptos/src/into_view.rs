use std::borrow::Cow;
use tachys::{
    html::attribute::{any_attribute::AnyAttribute, Attribute},
    hydration::Cursor,
    ssr::StreamBuilder,
    view::{
        add_attr::AddAnyAttr, any_view::ExtraAttrsMut, Position, PositionState,
        Render, RenderHtml, ToTemplate,
    },
};

/// A wrapper for any kind of view.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct View<T>
where
    T: Sized,
{
    inner: T,
    #[cfg(debug_assertions)]
    view_marker: Option<Cow<'static, str>>,
}

impl<T> View<T> {
    /// Wraps the view.
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            #[cfg(debug_assertions)]
            view_marker: None,
        }
    }

    /// Unwraps the view, returning the inner type.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Adds a view marker, which is used for hot-reloading and debug purposes.
    #[inline(always)]
    pub fn with_view_marker(
        #[allow(unused_mut)] // used in debug
        mut self,
        #[allow(unused_variables)] // used in debug
        view_marker: impl Into<Cow<'static, str>>,
    ) -> Self {
        #[cfg(debug_assertions)]
        {
            self.view_marker = Some(view_marker.into());
        }
        self
    }
}

/// A trait that is implemented for types that can be rendered.
pub trait IntoView
where
    Self: Sized + Render + RenderHtml + Send,
{
    /// Wraps the inner type.
    fn into_view(self) -> View<Self>;
}

impl<T> IntoView for T
where
    T: Sized + Render + RenderHtml + Send, //+ AddAnyAttr,
{
    fn into_view(self) -> View<Self> {
        View {
            inner: self,
            #[cfg(debug_assertions)]
            view_marker: None,
        }
    }
}

impl<T: Render> Render for View<T> {
    type State = T::State;

    fn build(self, extra_attrs: Option<Vec<AnyAttribute>>) -> Self::State {
        self.inner.build(extra_attrs)
    }

    fn rebuild(
        self,
        state: &mut Self::State,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) {
        self.inner.rebuild(state, extra_attrs)
    }
}

impl<T: RenderHtml> RenderHtml for View<T> {
    type AsyncOutput = T::AsyncOutput;
    type Owned = View<T::Owned>;

    const MIN_LENGTH: usize = <T as RenderHtml>::MIN_LENGTH;

    async fn resolve(
        self,
        extra_attrs: ExtraAttrsMut<'_>,
    ) -> Self::AsyncOutput {
        self.inner.resolve(extra_attrs).await
    }

    fn dry_resolve(&mut self, extra_attrs: ExtraAttrsMut<'_>) {
        self.inner.dry_resolve(extra_attrs);
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) {
        #[cfg(debug_assertions)]
        let vm = self.view_marker.to_owned();
        #[cfg(debug_assertions)]
        if let Some(vm) = vm.as_ref() {
            buf.push_str(&format!("<!--hot-reload|{vm}|open-->"));
        }

        self.inner.to_html_with_buf(
            buf,
            position,
            escape,
            mark_branches,
            extra_attrs,
        );

        #[cfg(debug_assertions)]
        if let Some(vm) = vm.as_ref() {
            buf.push_str(&format!("<!--hot-reload|{vm}|close-->"));
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) where
        Self: Sized,
    {
        #[cfg(debug_assertions)]
        let vm = self.view_marker.to_owned();
        #[cfg(debug_assertions)]
        if let Some(vm) = vm.as_ref() {
            buf.push_sync(&format!("<!--hot-reload|{vm}|open-->"));
        }

        self.inner.to_html_async_with_buf::<OUT_OF_ORDER>(
            buf,
            position,
            escape,
            mark_branches,
            extra_attrs,
        );

        #[cfg(debug_assertions)]
        if let Some(vm) = vm.as_ref() {
            buf.push_sync(&format!("<!--hot-reload|{vm}|close-->"));
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) -> Self::State {
        self.inner
            .hydrate::<FROM_SERVER>(cursor, position, extra_attrs)
    }

    fn into_owned(self) -> Self::Owned {
        View {
            inner: self.inner.into_owned(),
            #[cfg(debug_assertions)]
            view_marker: self.view_marker,
        }
    }
}

impl<T: ToTemplate> ToTemplate for View<T> {
    fn to_template(
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
        position: &mut Position,
    ) {
        T::to_template(buf, class, style, inner_html, position);
    }
}

impl<T: AddAnyAttr> AddAnyAttr for View<T> {
    type Output<SomeNewAttr: Attribute> = View<T::Output<SomeNewAttr>>;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        let View {
            inner,
            #[cfg(debug_assertions)]
            view_marker,
        } = self;
        View {
            inner: inner.add_any_attr(attr),
            #[cfg(debug_assertions)]
            view_marker,
        }
    }
}

/// Collects some iterator of views into a list, so they can be rendered.
///
/// This is a shorthand for `.collect::<Vec<_>>()`, and allows any iterator of renderable
/// items to be collected into a renderable collection.
pub trait CollectView {
    /// The inner view type.
    type View: IntoView;

    /// Collects the iterator into a list of views.
    fn collect_view(self) -> Vec<Self::View>;
}

impl<It, V> CollectView for It
where
    It: IntoIterator<Item = V>,
    V: IntoView,
{
    type View = V;

    fn collect_view(self) -> Vec<Self::View> {
        self.into_iter().collect()
    }
}
