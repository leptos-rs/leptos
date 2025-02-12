#[cfg(feature = "ssr")]
use super::MarkBranch;
use super::{
    add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
    RenderHtml,
};
use crate::{
    html::attribute::{
        any_attribute::{AnyAttribute, AnyAttributeState, IntoAnyAttribute},
        Attribute,
    },
    hydration::Cursor,
    ssr::StreamBuilder,
};
use futures::future::{join, join_all};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
};
#[cfg(feature = "ssr")]
use std::{future::Future, pin::Pin};

/// A type-erased view. This can be used if control flow requires that multiple different types of
/// view must be received, and it is either impossible or too cumbersome to use the `EitherOf___`
/// enums.
///
/// It can also be used to create recursive components, which otherwise cannot return themselves
/// due to the static typing of the view tree.
///
/// Generally speaking, using `AnyView` restricts the amount of information available to the
/// compiler and should be limited to situations in which it is necessary to preserve the maximum
/// amount of type information possible.
pub struct AnyView {
    type_id: TypeId,
    value: Box<dyn Any + Send>,
    build: fn(Box<dyn Any>) -> AnyViewState,
    rebuild: fn(TypeId, Box<dyn Any>, &mut AnyViewState),
    // The fields below are cfg-gated so they will not be included in WASM bundles if not needed.
    // Ordinarily, the compiler can simply omit this dead code because the methods are not called.
    // With this type-erased wrapper, however, the compiler is not *always* able to correctly
    // eliminate that code.
    #[cfg(feature = "ssr")]
    html_len: usize,
    #[cfg(feature = "ssr")]
    to_html: fn(
        Box<dyn Any>,
        &mut String,
        &mut Position,
        bool,
        bool,
        Vec<AnyAttribute>,
    ),
    #[cfg(feature = "ssr")]
    to_html_async: fn(
        Box<dyn Any>,
        &mut StreamBuilder,
        &mut Position,
        bool,
        bool,
        Vec<AnyAttribute>,
    ),
    #[cfg(feature = "ssr")]
    to_html_async_ooo: fn(
        Box<dyn Any>,
        &mut StreamBuilder,
        &mut Position,
        bool,
        bool,
        Vec<AnyAttribute>,
    ),
    #[cfg(feature = "ssr")]
    #[allow(clippy::type_complexity)]
    resolve: fn(Box<dyn Any>) -> Pin<Box<dyn Future<Output = AnyView> + Send>>,
    #[cfg(feature = "ssr")]
    dry_resolve: fn(&mut Box<dyn Any + Send>),
    #[cfg(feature = "hydrate")]
    #[cfg(feature = "hydrate")]
    #[allow(clippy::type_complexity)]
    hydrate_from_server:
        fn(Box<dyn Any>, &Cursor, &PositionState) -> AnyViewState,
}

/// Retained view state for [`AnyView`].
pub struct AnyViewState {
    type_id: TypeId,
    state: Box<dyn Any>,
    unmount: fn(&mut dyn Any),
    mount: fn(
        &mut dyn Any,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ),
    insert_before_this: fn(&dyn Any, child: &mut dyn Mountable) -> bool,
    elements: fn(&dyn Any) -> Vec<crate::renderer::types::Element>,
}

impl Debug for AnyViewState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyViewState")
            .field("type_id", &self.type_id)
            .field("state", &self.state)
            .field("unmount", &self.unmount)
            .field("mount", &self.mount)
            .field("insert_before_this", &self.insert_before_this)
            .finish()
    }
}

/// Allows converting some view into [`AnyView`].
pub trait IntoAny {
    /// Converts the view into a type-erased [`AnyView`].
    fn into_any(self) -> AnyView;
}

fn mount_any<T>(
    state: &mut dyn Any,
    parent: &crate::renderer::types::Element,
    marker: Option<&crate::renderer::types::Node>,
) where
    T: Render,
    T::State: 'static,
{
    let state = state
        .downcast_mut::<T::State>()
        .expect("AnyViewState::as_mountable couldn't downcast state");
    state.mount(parent, marker)
}

fn unmount_any<T>(state: &mut dyn Any)
where
    T: Render,
    T::State: 'static,
{
    let state = state
        .downcast_mut::<T::State>()
        .expect("AnyViewState::unmount couldn't downcast state");
    state.unmount();
}

fn insert_before_this<T>(state: &dyn Any, child: &mut dyn Mountable) -> bool
where
    T: Render,
    T::State: 'static,
{
    let state = state
        .downcast_ref::<T::State>()
        .expect("AnyViewState::insert_before_this couldn't downcast state");
    state.insert_before_this(child)
}

fn elements<T>(state: &dyn Any) -> Vec<crate::renderer::types::Element>
where
    T: Render,
    T::State: 'static,
{
    let state = state
        .downcast_ref::<T::State>()
        .expect("AnyViewState::insert_before_this couldn't downcast state");
    state.elements()
}

impl<T> IntoAny for T
where
    T: Send,
    T: RenderHtml,
{
    fn into_any(self) -> AnyView {
        #[cfg(feature = "ssr")]
        let html_len = self.html_len();

        let value = Box::new(self.into_owned()) as Box<dyn Any + Send>;

        match value.downcast::<AnyView>() {
            // if it's already an AnyView, we don't need to double-wrap it
            Ok(any_view) => *any_view,
            Err(value) => {
                #[cfg(feature = "ssr")]
                let dry_resolve = |value: &mut Box<dyn Any + Send>| {
                    let value = value
                        .downcast_mut::<T::Owned>()
                        .expect("AnyView::resolve could not be downcast");
                    value.dry_resolve();
                };

                #[cfg(feature = "ssr")]
                let resolve = |value: Box<dyn Any>| {
                    let value = value
                        .downcast::<T::Owned>()
                        .expect("AnyView::resolve could not be downcast");
                    Box::pin(async move { value.resolve().await.into_any() })
                        as Pin<Box<dyn Future<Output = AnyView> + Send>>
                };
                #[cfg(feature = "ssr")]
                let to_html =
                    |value: Box<dyn Any>,
                     buf: &mut String,
                     position: &mut Position,
                     escape: bool,
                     mark_branches: bool,
                     extra_attrs: Vec<AnyAttribute>| {
                        let type_id = mark_branches
                            .then(|| format!("{:?}", TypeId::of::<T::Owned>()))
                            .unwrap_or_default();
                        let value = value
                            .downcast::<T::Owned>()
                            .expect("AnyView::to_html could not be downcast");
                        if mark_branches {
                            buf.open_branch(&type_id);
                        }
                        value.to_html_with_buf(
                            buf,
                            position,
                            escape,
                            mark_branches,
                            extra_attrs,
                        );
                        if mark_branches {
                            buf.close_branch(&type_id);
                        }
                    };
                #[cfg(feature = "ssr")]
                let to_html_async =
                    |value: Box<dyn Any>,
                     buf: &mut StreamBuilder,
                     position: &mut Position,
                     escape: bool,
                     mark_branches: bool,
                     extra_attrs: Vec<AnyAttribute>| {
                        let type_id = mark_branches
                            .then(|| format!("{:?}", TypeId::of::<T::Owned>()))
                            .unwrap_or_default();
                        let value = value
                            .downcast::<T::Owned>()
                            .expect("AnyView::to_html could not be downcast");
                        if mark_branches {
                            buf.open_branch(&type_id);
                        }
                        value.to_html_async_with_buf::<false>(
                            buf,
                            position,
                            escape,
                            mark_branches,
                            extra_attrs,
                        );
                        if mark_branches {
                            buf.close_branch(&type_id);
                        }
                    };
                #[cfg(feature = "ssr")]
                let to_html_async_ooo =
                    |value: Box<dyn Any>,
                     buf: &mut StreamBuilder,
                     position: &mut Position,
                     escape: bool,
                     mark_branches: bool,
                     extra_attrs: Vec<AnyAttribute>| {
                        let value = value
                            .downcast::<T::Owned>()
                            .expect("AnyView::to_html could not be downcast");
                        value.to_html_async_with_buf::<true>(
                            buf,
                            position,
                            escape,
                            mark_branches,
                            extra_attrs,
                        );
                    };
                let build = |value: Box<dyn Any>| {
                    let value = value
                        .downcast::<T::Owned>()
                        .expect("AnyView::build couldn't downcast");
                    let state = Box::new(value.build());

                    AnyViewState {
                        type_id: TypeId::of::<T::Owned>(),
                        state,
                        mount: mount_any::<T::Owned>,
                        unmount: unmount_any::<T::Owned>,
                        insert_before_this: insert_before_this::<T::Owned>,
                        elements: elements::<T::Owned>,
                    }
                };
                #[cfg(feature = "hydrate")]
                let hydrate_from_server =
                    |value: Box<dyn Any>,
                     cursor: &Cursor,
                     position: &PositionState| {
                        let value = value.downcast::<T::Owned>().expect(
                            "AnyView::hydrate_from_server couldn't downcast",
                        );
                        let state =
                            Box::new(value.hydrate::<true>(cursor, position));

                        AnyViewState {
                            type_id: TypeId::of::<T::Owned>(),
                            state,
                            mount: mount_any::<T::Owned>,
                            unmount: unmount_any::<T::Owned>,
                            insert_before_this: insert_before_this::<T::Owned>,
                            elements: elements::<T::Owned>,
                        }
                    };

                let rebuild =
                    |new_type_id: TypeId,
                     value: Box<dyn Any>,
                     state: &mut AnyViewState| {
                        let value = value
                            .downcast::<T::Owned>()
                            .expect("AnyView::rebuild couldn't downcast value");
                        if new_type_id == state.type_id {
                            let state = state.state.downcast_mut().expect(
                                "AnyView::rebuild couldn't downcast state",
                            );
                            value.rebuild(state);
                        } else {
                            let mut new = value.into_any().build();
                            state.insert_before_this(&mut new);
                            state.unmount();
                            *state = new;
                        }
                    };

                AnyView {
                    type_id: TypeId::of::<T::Owned>(),
                    value,
                    build,
                    rebuild,
                    #[cfg(feature = "ssr")]
                    resolve,
                    #[cfg(feature = "ssr")]
                    dry_resolve,
                    #[cfg(feature = "ssr")]
                    html_len,
                    #[cfg(feature = "ssr")]
                    to_html,
                    #[cfg(feature = "ssr")]
                    to_html_async,
                    #[cfg(feature = "ssr")]
                    to_html_async_ooo,
                    #[cfg(feature = "hydrate")]
                    hydrate_from_server,
                }
            }
        }
    }
}

impl Render for AnyView {
    type State = AnyViewState;

    fn build(self) -> Self::State {
        (self.build)(self.value)
    }

    fn rebuild(self, state: &mut Self::State) {
        (self.rebuild)(self.type_id, self.value, state)
    }
}

impl AddAnyAttr for AnyView {
    type Output<SomeNewAttr: Attribute> = AnyViewWithAttrs;

    #[allow(unused_variables)]
    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        AnyViewWithAttrs {
            view: self,
            attrs: vec![attr.into_cloneable_owned().into_any_attr()],
        }
    }
}

impl RenderHtml for AnyView {
    type AsyncOutput = Self;
    type Owned = Self;

    fn dry_resolve(&mut self) {
        #[cfg(feature = "ssr")]
        {
            (self.dry_resolve)(&mut self.value)
        }
        #[cfg(not(feature = "ssr"))]
        panic!(
            "You are rendering AnyView to HTML without the `ssr` feature \
             enabled."
        );
    }

    async fn resolve(self) -> Self::AsyncOutput {
        #[cfg(feature = "ssr")]
        {
            (self.resolve)(self.value).await
        }
        #[cfg(not(feature = "ssr"))]
        panic!(
            "You are rendering AnyView to HTML without the `ssr` feature \
             enabled."
        );
    }

    const MIN_LENGTH: usize = 0;

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Vec<AnyAttribute>,
    ) {
        #[cfg(feature = "ssr")]
        (self.to_html)(
            self.value,
            buf,
            position,
            escape,
            mark_branches,
            extra_attrs,
        );
        #[cfg(not(feature = "ssr"))]
        {
            _ = mark_branches;
            _ = buf;
            _ = position;
            _ = escape;
            _ = extra_attrs;
            panic!(
                "You are rendering AnyView to HTML without the `ssr` feature \
                 enabled."
            );
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Vec<AnyAttribute>,
    ) where
        Self: Sized,
    {
        #[cfg(feature = "ssr")]
        if OUT_OF_ORDER {
            (self.to_html_async_ooo)(
                self.value,
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs,
            );
        } else {
            (self.to_html_async)(
                self.value,
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs,
            );
        }
        #[cfg(not(feature = "ssr"))]
        {
            _ = buf;
            _ = position;
            _ = escape;
            _ = mark_branches;
            _ = extra_attrs;
            panic!(
                "You are rendering AnyView to HTML without the `ssr` feature \
                 enabled."
            );
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        #[cfg(feature = "hydrate")]
        if FROM_SERVER {
            (self.hydrate_from_server)(self.value, cursor, position)
        } else {
            panic!(
                "hydrating AnyView from inside a ViewTemplate is not \
                 supported."
            );
        }
        #[cfg(not(feature = "hydrate"))]
        {
            _ = cursor;
            _ = position;
            panic!(
                "You are trying to hydrate AnyView without the `hydrate` \
                 feature enabled."
            );
        }
    }

    fn html_len(&self) -> usize {
        #[cfg(feature = "ssr")]
        {
            self.html_len
        }
        #[cfg(not(feature = "ssr"))]
        {
            0
        }
    }

    fn into_owned(self) -> Self::Owned {
        self
    }
}

impl Mountable for AnyViewState {
    fn unmount(&mut self) {
        (self.unmount)(&mut *self.state)
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        (self.mount)(&mut *self.state, parent, marker)
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        (self.insert_before_this)(&*self.state, child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        (self.elements)(&*self.state)
    }
}

/// wip
pub struct AnyViewWithAttrs {
    view: AnyView,
    attrs: Vec<AnyAttribute>,
}

impl Render for AnyViewWithAttrs {
    type State = AnyViewWithAttrsState;

    fn build(self) -> Self::State {
        let view = self.view.build();
        let elements = view.elements();
        let mut attrs = Vec::with_capacity(elements.len() * self.attrs.len());
        for attr in self.attrs {
            for el in &elements {
                attrs.push(attr.clone().build(el))
            }
        }
        AnyViewWithAttrsState { view, attrs }
    }

    fn rebuild(self, state: &mut Self::State) {
        self.view.rebuild(&mut state.view);
        self.attrs.rebuild(&mut state.attrs);
    }
}

impl RenderHtml for AnyViewWithAttrs {
    type AsyncOutput = Self;
    type Owned = Self;
    const MIN_LENGTH: usize = 0;

    fn dry_resolve(&mut self) {
        self.view.dry_resolve();
        for attr in &mut self.attrs {
            attr.dry_resolve();
        }
    }

    async fn resolve(self) -> Self::AsyncOutput {
        let resolve_view = self.view.resolve();
        let resolve_attrs =
            join_all(self.attrs.into_iter().map(|attr| attr.resolve()));
        let (view, attrs) = join(resolve_view, resolve_attrs).await;
        Self { view, attrs }
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        mut extra_attrs: Vec<AnyAttribute>,
    ) {
        // `extra_attrs` will be empty here in most cases, but it will have
        // attributes in it already if this is, itself, receiving additional attrs
        extra_attrs.extend(self.attrs);
        self.view.to_html_with_buf(
            buf,
            position,
            escape,
            mark_branches,
            extra_attrs,
        );
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        mut extra_attrs: Vec<AnyAttribute>,
    ) where
        Self: Sized,
    {
        extra_attrs.extend(self.attrs);
        self.view.to_html_async_with_buf::<OUT_OF_ORDER>(
            buf,
            position,
            escape,
            mark_branches,
            extra_attrs,
        );
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        let view = self.view.hydrate::<FROM_SERVER>(cursor, position);
        let elements = view.elements();
        let mut attrs = Vec::with_capacity(elements.len() * self.attrs.len());
        for attr in self.attrs {
            for el in &elements {
                attrs.push(attr.clone().hydrate::<FROM_SERVER>(el));
            }
        }
        AnyViewWithAttrsState { view, attrs }
    }

    fn html_len(&self) -> usize {
        self.view.html_len()
            + self.attrs.iter().map(|attr| attr.html_len()).sum::<usize>()
    }

    fn into_owned(self) -> Self::Owned {
        self
    }
}

impl AddAnyAttr for AnyViewWithAttrs {
    type Output<SomeNewAttr: Attribute> = AnyViewWithAttrs;

    fn add_any_attr<NewAttr: Attribute>(
        mut self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        self.attrs.push(attr.into_cloneable_owned().into_any_attr());
        self
    }
}

/// wip
pub struct AnyViewWithAttrsState {
    view: AnyViewState,
    attrs: Vec<AnyAttributeState>,
}

impl Mountable for AnyViewWithAttrsState {
    fn unmount(&mut self) {
        self.view.unmount();
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        self.view.mount(parent, marker)
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.view.insert_before_this(child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        self.view.elements()
    }
}

/*
#[cfg(test)]
mod tests {
    use super::IntoAny;
    use crate::{
        html::element::{p, span},
        renderer::mock_dom::MockDom,
        view::{any_view::AnyView, RenderHtml},
    };

    #[test]
    fn should_handle_html_creation() {
        let x = 1;
        let mut buf = String::new();
        let view: AnyView<MockDom> = if x == 0 {
            p((), "foo").into_any()
        } else {
            span((), "bar").into_any()
        };
        view.to_html(&mut buf, &Default::default());
        assert_eq!(buf, "<span>bar</span><!>");
    }
}
 */
