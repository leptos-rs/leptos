#![allow(clippy::type_complexity)]
#[cfg(feature = "ssr")]
use super::MarkBranch;
use super::{
    add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
    RenderHtml,
};
use crate::{
    erased::{Erased, ErasedLocal},
    html::attribute::{
        any_attribute::{AnyAttribute, AnyAttributeState, IntoAnyAttribute},
        Attribute,
    },
    hydration::Cursor,
    ssr::StreamBuilder,
};
use futures::future::{join, join_all};
use std::{any::TypeId, fmt::Debug};
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
    value: Erased,
    build: fn(Erased) -> AnyViewState,
    rebuild: fn(Erased, &mut AnyViewState),
    // The fields below are cfg-gated so they will not be included in WASM bundles if not needed.
    // Ordinarily, the compiler can simply omit this dead code because the methods are not called.
    // With this type-erased wrapper, however, the compiler is not *always* able to correctly
    // eliminate that code.
    #[cfg(feature = "ssr")]
    html_len: usize,
    #[cfg(feature = "ssr")]
    to_html:
        fn(Erased, &mut String, &mut Position, bool, bool, Vec<AnyAttribute>),
    #[cfg(feature = "ssr")]
    to_html_async: fn(
        Erased,
        &mut StreamBuilder,
        &mut Position,
        bool,
        bool,
        Vec<AnyAttribute>,
    ),
    #[cfg(feature = "ssr")]
    to_html_async_ooo: fn(
        Erased,
        &mut StreamBuilder,
        &mut Position,
        bool,
        bool,
        Vec<AnyAttribute>,
    ),
    #[cfg(feature = "ssr")]
    #[allow(clippy::type_complexity)]
    resolve: fn(Erased) -> Pin<Box<dyn Future<Output = AnyView> + Send>>,
    #[cfg(feature = "ssr")]
    dry_resolve: fn(&mut Erased),
    #[cfg(feature = "hydrate")]
    #[allow(clippy::type_complexity)]
    hydrate_from_server: fn(Erased, &Cursor, &PositionState) -> AnyViewState,
}

impl Debug for AnyView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyView")
            .field("type_id", &self.type_id)
            .finish_non_exhaustive()
    }
}
/// Retained view state for [`AnyView`].
pub struct AnyViewState {
    type_id: TypeId,
    state: ErasedLocal,
    unmount: fn(&mut ErasedLocal),
    mount: fn(
        &mut ErasedLocal,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ),
    insert_before_this: fn(&ErasedLocal, child: &mut dyn Mountable) -> bool,
    elements: fn(&ErasedLocal) -> Vec<crate::renderer::types::Element>,
}

impl Debug for AnyViewState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyViewState")
            .field("type_id", &self.type_id)
            .field("state", &"")
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

/// A more general version of [`IntoAny`] that allows into [`AnyView`],
/// but also erasing other types that don't implement [`RenderHtml`] like routing.
pub trait IntoMaybeErased {
    /// The type of the output.
    type Output: IntoMaybeErased;

    /// Converts the view into a type-erased view if in erased mode.
    fn into_maybe_erased(self) -> Self::Output;
}

impl<T> IntoMaybeErased for T
where
    T: RenderHtml,
{
    #[cfg(not(erase_components))]
    type Output = Self;

    #[cfg(erase_components)]
    type Output = AnyView;

    fn into_maybe_erased(self) -> Self::Output {
        #[cfg(not(erase_components))]
        {
            self
        }
        #[cfg(erase_components)]
        {
            self.into_owned().into_any()
        }
    }
}

fn mount_any<T>(
    state: &mut ErasedLocal,
    parent: &crate::renderer::types::Element,
    marker: Option<&crate::renderer::types::Node>,
) where
    T: Render,
    T::State: 'static,
{
    state.get_mut::<T::State>().mount(parent, marker)
}

fn unmount_any<T>(state: &mut ErasedLocal)
where
    T: Render,
    T::State: 'static,
{
    state.get_mut::<T::State>().unmount();
}

fn insert_before_this<T>(state: &ErasedLocal, child: &mut dyn Mountable) -> bool
where
    T: Render,
    T::State: 'static,
{
    state.get_ref::<T::State>().insert_before_this(child)
}

fn elements<T>(state: &ErasedLocal) -> Vec<crate::renderer::types::Element>
where
    T: Render,
    T::State: 'static,
{
    state.get_ref::<T::State>().elements()
}

impl<T> IntoAny for T
where
    T: Send,
    T: RenderHtml,
{
    fn into_any(self) -> AnyView {
        #[cfg(feature = "ssr")]
        fn dry_resolve<T: RenderHtml + 'static>(value: &mut Erased) {
            value.get_mut::<T>().dry_resolve();
        }

        #[cfg(feature = "ssr")]
        fn resolve<T: RenderHtml + 'static>(
            value: Erased,
        ) -> Pin<Box<dyn Future<Output = AnyView> + Send>> {
            use futures::FutureExt;

            async move { value.into_inner::<T>().resolve().await.into_any() }
                .boxed()
        }

        #[cfg(feature = "ssr")]
        fn to_html<T: RenderHtml + 'static>(
            value: Erased,
            buf: &mut String,
            position: &mut Position,
            escape: bool,
            mark_branches: bool,
            extra_attrs: Vec<AnyAttribute>,
        ) {
            value.into_inner::<T>().to_html_with_buf(
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs,
            );
        }

        #[cfg(feature = "ssr")]
        fn to_html_async<T: RenderHtml + 'static>(
            value: Erased,
            buf: &mut StreamBuilder,
            position: &mut Position,
            escape: bool,
            mark_branches: bool,
            extra_attrs: Vec<AnyAttribute>,
        ) {
            value.into_inner::<T>().to_html_async_with_buf::<false>(
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs,
            );
        }

        #[cfg(feature = "ssr")]
        fn to_html_async_ooo<T: RenderHtml + 'static>(
            value: Erased,
            buf: &mut StreamBuilder,
            position: &mut Position,
            escape: bool,
            mark_branches: bool,
            extra_attrs: Vec<AnyAttribute>,
        ) {
            value.into_inner::<T>().to_html_async_with_buf::<true>(
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs,
            );
        }

        fn build<T: RenderHtml + 'static>(value: Erased) -> AnyViewState {
            let state = ErasedLocal::new(value.into_inner::<T>().build());
            AnyViewState {
                type_id: TypeId::of::<T>(),
                state,
                mount: mount_any::<T>,
                unmount: unmount_any::<T>,
                insert_before_this: insert_before_this::<T>,
                elements: elements::<T>,
            }
        }

        #[cfg(feature = "hydrate")]
        fn hydrate_from_server<T: RenderHtml + 'static>(
            value: Erased,
            cursor: &Cursor,
            position: &PositionState,
        ) -> AnyViewState {
            let state = ErasedLocal::new(
                value.into_inner::<T>().hydrate::<true>(cursor, position),
            );
            AnyViewState {
                type_id: TypeId::of::<T>(),
                state,
                mount: mount_any::<T>,
                unmount: unmount_any::<T>,
                insert_before_this: insert_before_this::<T>,
                elements: elements::<T>,
            }
        }

        fn rebuild<T: RenderHtml + 'static>(
            value: Erased,
            state: &mut AnyViewState,
        ) {
            let state = state.state.get_mut::<<T as Render>::State>();
            value.into_inner::<T>().rebuild(state);
        }

        let value = self.into_owned();
        AnyView {
            type_id: TypeId::of::<T::Owned>(),
            build: build::<T::Owned>,
            rebuild: rebuild::<T::Owned>,
            #[cfg(feature = "ssr")]
            resolve: resolve::<T::Owned>,
            #[cfg(feature = "ssr")]
            dry_resolve: dry_resolve::<T::Owned>,
            #[cfg(feature = "ssr")]
            html_len: value.html_len(),
            #[cfg(feature = "ssr")]
            to_html: to_html::<T::Owned>,
            #[cfg(feature = "ssr")]
            to_html_async: to_html_async::<T::Owned>,
            #[cfg(feature = "ssr")]
            to_html_async_ooo: to_html_async_ooo::<T::Owned>,
            #[cfg(feature = "hydrate")]
            hydrate_from_server: hydrate_from_server::<T::Owned>,
            value: Erased::new(value),
        }
    }
}

impl Render for AnyView {
    type State = AnyViewState;

    fn build(self) -> Self::State {
        (self.build)(self.value)
    }

    fn rebuild(self, state: &mut Self::State) {
        if self.type_id == state.type_id {
            (self.rebuild)(self.value, state)
        } else {
            let mut new = self.build();
            state.insert_before_this(&mut new);
            state.unmount();
            *state = new;
        }
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
        {
            let type_id = if mark_branches && escape {
                format!("{:?}", self.type_id)
            } else {
                Default::default()
            };
            if mark_branches && escape {
                buf.open_branch(&type_id);
            }
            (self.to_html)(
                self.value,
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs,
            );
            if mark_branches && escape {
                buf.close_branch(&type_id);
                if *position == Position::NextChildAfterText {
                    *position = Position::NextChild;
                }
            }
        }
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
            let type_id = if mark_branches && escape {
                format!("{:?}", self.type_id)
            } else {
                Default::default()
            };
            if mark_branches && escape {
                buf.open_branch(&type_id);
            }
            (self.to_html_async_ooo)(
                self.value,
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs,
            );
            if mark_branches && escape {
                buf.close_branch(&type_id);
                if *position == Position::NextChildAfterText {
                    *position = Position::NextChild;
                }
            }
        } else {
            let type_id = if mark_branches && escape {
                format!("{:?}", self.type_id)
            } else {
                Default::default()
            };
            if mark_branches && escape {
                buf.open_branch(&type_id);
            }
            (self.to_html_async)(
                self.value,
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs,
            );
            if mark_branches && escape {
                buf.close_branch(&type_id);
                if *position == Position::NextChildAfterText {
                    *position = Position::NextChild;
                }
            }
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
        {
            if FROM_SERVER {
                if cfg!(feature = "mark_branches") {
                    cursor.advance_to_placeholder(position);
                }
                let state =
                    (self.hydrate_from_server)(self.value, cursor, position);
                if cfg!(feature = "mark_branches") {
                    cursor.advance_to_placeholder(position);
                }
                state
            } else {
                panic!(
                    "hydrating AnyView from inside a ViewTemplate is not \
                     supported."
                );
            }
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
        (self.unmount)(&mut self.state)
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        (self.mount)(&mut self.state, parent, marker)
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        (self.insert_before_this)(&self.state, child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        (self.elements)(&self.state)
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
