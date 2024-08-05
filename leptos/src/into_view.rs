#[cfg(debug_assertions)]
use std::borrow::Cow;
use tachys::{
    html::attribute::Attribute,
    hydration::Cursor,
    renderer::dom::Dom,
    ssr::StreamBuilder,
    view::{add_attr::AddAnyAttr, Position, PositionState, Render, RenderHtml},
};

#[derive(Debug)]
pub struct View<T>
where
    T: Sized,
{
    inner: T,
    #[cfg(debug_assertions)]
    view_marker: Option<Cow<'static, str>>,
}

impl<T> View<T> {
    pub fn into_inner(self) -> T {
        self.inner
    }

    #[inline(always)]
    pub fn with_view_marker(
        mut self,
        view_marker: impl Into<Cow<'static, str>>,
    ) -> Self {
        #[cfg(debug_assertions)]
        {
            self.view_marker = Some(view_marker.into());
        }
        self
    }
}

pub trait IntoView
where
    Self: Sized + Render<Dom> + RenderHtml<Dom> + Send,
{
    fn into_view(self) -> View<Self>;
}

impl<T> IntoView for T
where
    T: Sized + Render<Dom> + RenderHtml<Dom> + Send, //+ AddAnyAttr<Dom>,
{
    fn into_view(self) -> View<Self> {
        View {
            inner: self,
            #[cfg(debug_assertions)]
            view_marker: None,
        }
    }
}

impl<T: IntoView> Render<Dom> for View<T> {
    type State = T::State;

    fn build(self) -> Self::State {
        self.inner.build()
    }

    fn rebuild(self, state: &mut Self::State) {
        self.inner.rebuild(state)
    }
}

impl<T: IntoView> RenderHtml<Dom> for View<T> {
    type AsyncOutput = T::AsyncOutput;

    const MIN_LENGTH: usize = <T as RenderHtml<Dom>>::MIN_LENGTH;

    async fn resolve(self) -> Self::AsyncOutput {
        self.inner.resolve().await
    }

    fn dry_resolve(&mut self) {
        self.inner.dry_resolve();
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        #[cfg(debug_assertions)]
        let vm = self.view_marker.to_owned();
        #[cfg(debug_assertions)]
        if let Some(vm) = vm.as_ref() {
            buf.push_str(&format!("<!--hot-reload|{vm}|open-->"));
        }

        self.inner
            .to_html_with_buf(buf, position, escape, mark_branches);

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
        );

        #[cfg(debug_assertions)]
        if let Some(vm) = vm.as_ref() {
            buf.push_sync(&format!("<!--hot-reload|{vm}|close-->"));
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Dom>,
        position: &PositionState,
    ) -> Self::State {
        self.inner.hydrate::<FROM_SERVER>(cursor, position)
    }
}

impl<T: IntoView> AddAnyAttr<Dom> for View<T> {
    type Output<SomeNewAttr: Attribute<Dom>> =
        <T as AddAnyAttr<Dom>>::Output<SomeNewAttr>;

    fn add_any_attr<NewAttr: Attribute<Dom>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Dom>,
    {
        self.inner.add_any_attr(attr)
    }
}

pub trait CollectView {
    type View: IntoView;

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
