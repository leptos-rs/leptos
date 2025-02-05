#![allow(unused_mut)]
#![allow(clippy::type_complexity)]
#[cfg(feature = "ssr")]
use super::MarkBranch;
use super::{
    add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
    RenderHtml,
};
use crate::{
    html::attribute::{
        any_attribute::{AnyAttribute, IntoAnyAttribute},
        Attribute,
    },
    hydration::Cursor,
    ssr::StreamBuilder,
};
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
    extra_attrs: Vec<AnyAttribute>,
    build: fn(Box<dyn Any>, Option<Vec<AnyAttribute>>) -> AnyViewState,
    rebuild: fn(Box<dyn Any>, &mut AnyViewState, Option<Vec<AnyAttribute>>),
    // The fields below are cfg-gated so they will not be included in WASM bundles if not needed.
    // Ordinarily, the compiler can simply omit this dead code because the methods are not called.
    // With this type-erased wrapper, however, the compiler is not *always* able to correctly
    // eliminate that code.
    #[cfg(feature = "ssr")]
    html_len: fn(&Box<dyn Any + Send>, Option<Vec<&AnyAttribute>>) -> usize,
    #[cfg(feature = "ssr")]
    to_html: fn(
        Box<dyn Any>,
        &mut String,
        &mut Position,
        bool,
        bool,
        Option<Vec<AnyAttribute>>,
    ),
    #[cfg(feature = "ssr")]
    to_html_async: fn(
        Box<dyn Any>,
        &mut StreamBuilder,
        &mut Position,
        bool,
        bool,
        Option<Vec<AnyAttribute>>,
    ),
    #[cfg(feature = "ssr")]
    to_html_async_ooo: fn(
        Box<dyn Any>,
        &mut StreamBuilder,
        &mut Position,
        bool,
        bool,
        Option<Vec<AnyAttribute>>,
    ),
    #[cfg(feature = "ssr")]
    #[allow(clippy::type_complexity)]
    resolve: for<'a> fn(
        Box<dyn Any>,
        ExtraAttrsMut<'a>,
    )
        -> Pin<Box<dyn Future<Output = AnyView> + Send + 'a>>,
    #[cfg(feature = "ssr")]
    dry_resolve: fn(&mut Box<dyn Any + Send>, ExtraAttrsMut<'_>),
    #[cfg(feature = "hydrate")]
    #[cfg(feature = "hydrate")]
    #[allow(clippy::type_complexity)]
    hydrate_from_server: fn(
        Box<dyn Any>,
        &Cursor,
        &PositionState,
        Option<Vec<AnyAttribute>>,
    ) -> AnyViewState,
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

#[cfg(feature = "ssr")]
fn resolve<'a, T>(
    value: Box<dyn Any>,
    extra_attrs: ExtraAttrsMut<'a>,
) -> Pin<Box<dyn Future<Output = AnyView> + Send + 'a>>
where
    T: RenderHtml + 'static,
{
    let value = value
        .downcast::<T>()
        .expect("AnyView::resolve could not be downcast");
    Box::pin(async move { value.resolve(extra_attrs).await.into_any() })
}

impl<T> IntoAny for T
where
    T: RenderHtml,
    T::Owned: Send,
{
    fn into_any(self) -> AnyView {
        let value = Box::new(self.into_owned()) as Box<dyn Any + Send>;

        match value.downcast::<AnyView>() {
            // if it's already an AnyView, we don't need to double-wrap it
            Ok(any_view) => *any_view,
            Err(value) => {
                #[cfg(feature = "ssr")]
                let html_len =
                    |value: &Box<dyn Any + Send>, extra_attrs: Option<Vec<&AnyAttribute>>| {
                        let value = value
                            .downcast_ref::<T::Owned>()
                            .expect("AnyView::html_len could not be downcast");
                        value.html_len(extra_attrs)
                    };

                #[cfg(feature = "ssr")]
                let dry_resolve =
                    |value: &mut Box<dyn Any + Send>,
                     extra_attrs: ExtraAttrsMut<'_>| {
                        let value = value
                            .downcast_mut::<T::Owned>()
                            .expect("AnyView::resolve could not be downcast");
                        value.dry_resolve(extra_attrs);
                    };

                #[cfg(feature = "ssr")]
                let to_html =
                    |value: Box<dyn Any>,
                     buf: &mut String,
                     position: &mut Position,
                     escape: bool,
                     mark_branches: bool,
                     extra_attrs: Option<Vec<AnyAttribute>>| {
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
                     extra_attrs: Option<Vec<AnyAttribute>>| {
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
                let to_html_async_ooo = |value: Box<dyn Any>,
                                         buf: &mut StreamBuilder,
                                         position: &mut Position,
                                         escape: bool,
                                         mark_branches: bool,
                                         extra_attrs: Option<
                    Vec<AnyAttribute>,
                >| {
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
                let build = |value: Box<dyn Any>, extra_attrs: Option<Vec<AnyAttribute>>| {
                    let value = value
                        .downcast::<T::Owned>()
                        .expect("AnyView::build couldn't downcast");
                    let state = Box::new(value.build(extra_attrs));

                    AnyViewState {
                        type_id: TypeId::of::<T::Owned>(),
                        state,

                        mount: mount_any::<T::Owned>,
                        unmount: unmount_any::<T::Owned>,
                        insert_before_this: insert_before_this::<T::Owned>,
                    }
                };
                #[cfg(feature = "hydrate")]
                let hydrate_from_server = |value: Box<dyn Any>,
                                           cursor: &Cursor,
                                           position: &PositionState,
                                           extra_attrs: Option<
                    Vec<AnyAttribute>,
                >| {
                    let value = value.downcast::<T::Owned>().expect(
                        "AnyView::hydrate_from_server couldn't downcast",
                    );
                    let state = Box::new(value.hydrate::<true>(
                        cursor,
                        position,
                        extra_attrs,
                    ));

                    AnyViewState {
                        type_id: TypeId::of::<T::Owned>(),
                        state,

                        mount: mount_any::<T::Owned>,
                        unmount: unmount_any::<T::Owned>,
                        insert_before_this: insert_before_this::<T::Owned>,
                    }
                };

                let rebuild =
                    |value: Box<dyn Any>,
                     state: &mut AnyViewState, extra_attrs: Option<Vec<AnyAttribute>>| {
                        let value = value
                            .downcast::<T::Owned>()
                            .expect("AnyView::rebuild couldn't downcast value");
                        let state = state.state.downcast_mut().expect(
                            "AnyView::rebuild couldn't downcast state",
                        );
                        value.rebuild(state, extra_attrs);
                    };

                AnyView {
                    type_id: TypeId::of::<T::Owned>(),
                    value,
                    extra_attrs: vec![],
                    build,
                    rebuild,
                    #[cfg(feature = "ssr")]
                    html_len,
                    #[cfg(feature = "ssr")]
                    resolve: resolve::<T::Owned>,
                    #[cfg(feature = "ssr")]
                    dry_resolve,
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

/// Ignore, this is a hack for pre use<..> syntax.
/// https://github.com/rust-lang/rfcs/blob/master/text/3498-lifetime-capture-rules-2024.md#the-captures-trick
pub trait __Captures<T: ?Sized> {}
impl<T: ?Sized, U: ?Sized> __Captures<T> for U {}

/// A mutable view into the extra attributes stored in an [`AnyView`].
#[derive(Default)]
pub struct ExtraAttrsMut<'a>(Option<Vec<&'a mut Vec<AnyAttribute>>>);
impl<'a> ExtraAttrsMut<'a> {
    /// Create a new mutable view from owned attributes.
    pub fn from_owned(extra_attrs: &'a mut Option<Vec<AnyAttribute>>) -> Self {
        match extra_attrs {
            Some(extra_attrs) => {
                if extra_attrs.is_empty() {
                    Self(None)
                } else {
                    Self(Some(vec![extra_attrs]))
                }
            }
            None => Self(None),
        }
    }

    fn add_layer<'b>(
        mut self,
        extra_attrs: &'b mut Vec<AnyAttribute>,
    ) -> ExtraAttrsMut<'b>
    where
        'a: 'b,
    {
        match (self.0, extra_attrs.is_empty()) {
            (Some(mut extra), false) => {
                extra.push(extra_attrs);
                ExtraAttrsMut(Some(extra))
            }
            (Some(mut extra), true) => {
                self.0 = Some(extra);
                self
            }
            (None, false) => ExtraAttrsMut(Some(vec![extra_attrs])),
            (None, true) => ExtraAttrsMut(None),
        }
    }

    /// Check if there are any extra attributes.
    pub fn is_some(&self) -> bool {
        match &self.0 {
            Some(extra) => extra.is_empty(),
            None => true,
        }
    }

    /// "clone" the mutable view, to allow reuse in e.g. a for loop.
    /// The same as .as_deref_mut() on Option<&mut T>.
    pub fn as_deref_mut(&mut self) -> ExtraAttrsMut<'_> {
        ExtraAttrsMut(
            self.0
                .as_mut()
                .map(|inner| inner.iter_mut().map(|v| &mut **v).collect()),
        )
    }

    /// Iterate over the extra attributes.
    pub fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut AnyAttribute> + __Captures<&'a ()> + '_ {
        match &mut self.0 {
            Some(inner) => itertools::Either::Left(
                inner.iter_mut().flat_map(|v| v.iter_mut()),
            ),
            None => itertools::Either::Right(std::iter::empty()),
        }
    }

    /// Call [`RenderHtml::resolve`] on any extra attributes in parallel.
    pub async fn resolve(self) {
        if let Some(extra_attr_groups) = self.0 {
            futures::future::join_all(extra_attr_groups.into_iter().map(
                |extra_attrs| async move {
                    *extra_attrs =
                        Attribute::resolve(std::mem::take(extra_attrs)).await;
                },
            ))
            .await;
        }
    }
}

fn combine_owned_extra_attrs(
    parent_extra_attrs: Option<Vec<AnyAttribute>>,
    extra_attrs: Vec<AnyAttribute>,
) -> Option<Vec<AnyAttribute>> {
    let extra_attrs = if let Some(mut parent_extra_attrs) = parent_extra_attrs {
        for attr in extra_attrs {
            parent_extra_attrs.push(attr);
        }
        parent_extra_attrs
    } else {
        extra_attrs
    };
    if extra_attrs.is_empty() {
        None
    } else {
        Some(extra_attrs)
    }
}

impl Render for AnyView {
    type State = AnyViewState;

    fn build(self, extra_attrs: Option<Vec<AnyAttribute>>) -> Self::State {
        (self.build)(
            self.value,
            combine_owned_extra_attrs(extra_attrs, self.extra_attrs),
        )
    }

    fn rebuild(
        self,
        state: &mut Self::State,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) {
        if self.type_id == state.type_id {
            (self.rebuild)(
                self.value,
                state,
                combine_owned_extra_attrs(extra_attrs, self.extra_attrs),
            )
        } else {
            let mut new = (self.build)(
                self.value,
                combine_owned_extra_attrs(extra_attrs, self.extra_attrs),
            );
            state.insert_before_this(&mut new);
            state.unmount();
            *state = new;
        }
    }
}

impl AddAnyAttr for AnyView {
    type Output<SomeNewAttr: Attribute> = Self;

    #[allow(unused_variables)]
    fn add_any_attr<NewAttr: Attribute>(
        mut self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        self.extra_attrs
            .push(attr.into_cloneable_owned().into_any_attr());
        self
    }
}

impl RenderHtml for AnyView {
    type AsyncOutput = Self;
    type Owned = Self;

    fn dry_resolve(&mut self, extra_attrs: ExtraAttrsMut<'_>) {
        #[cfg(feature = "ssr")]
        {
            (self.dry_resolve)(
                &mut self.value,
                extra_attrs.add_layer(&mut self.extra_attrs),
            );
        }
        #[cfg(not(feature = "ssr"))]
        {
            _ = extra_attrs;
            panic!(
                "You are rendering AnyView to HTML without the `ssr` feature \
                 enabled."
            );
        }
    }

    async fn resolve(
        mut self,
        extra_attrs: ExtraAttrsMut<'_>,
    ) -> Self::AsyncOutput {
        #[cfg(feature = "ssr")]
        {
            (self.resolve)(
                self.value,
                extra_attrs.add_layer(&mut self.extra_attrs),
            )
            .await
        }
        #[cfg(not(feature = "ssr"))]
        {
            _ = extra_attrs;
            panic!(
                "You are rendering AnyView to HTML without the `ssr` feature \
                 enabled."
            );
        }
    }

    const MIN_LENGTH: usize = 0;

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) {
        #[cfg(feature = "ssr")]
        {
            (self.to_html)(
                self.value,
                buf,
                position,
                escape,
                mark_branches,
                combine_owned_extra_attrs(extra_attrs, self.extra_attrs),
            );
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
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) where
        Self: Sized,
    {
        #[cfg(feature = "ssr")]
        {
            if OUT_OF_ORDER {
                (self.to_html_async_ooo)(
                    self.value,
                    buf,
                    position,
                    escape,
                    mark_branches,
                    combine_owned_extra_attrs(extra_attrs, self.extra_attrs),
                );
            } else {
                (self.to_html_async)(
                    self.value,
                    buf,
                    position,
                    escape,
                    mark_branches,
                    combine_owned_extra_attrs(extra_attrs, self.extra_attrs),
                );
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
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) -> Self::State {
        #[cfg(feature = "hydrate")]
        {
            if FROM_SERVER {
                (self.hydrate_from_server)(
                    self.value,
                    cursor,
                    position,
                    combine_owned_extra_attrs(extra_attrs, self.extra_attrs),
                )
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
            _ = extra_attrs;
            panic!(
                "You are trying to hydrate AnyView without the `hydrate` \
                 feature enabled."
            );
        }
    }

    fn html_len(&self, extra_attrs: Option<Vec<&AnyAttribute>>) -> usize {
        #[cfg(feature = "ssr")]
        {
            (self.html_len)(
                &self.value,
                match (extra_attrs, self.extra_attrs.is_empty()) {
                    (Some(mut extra_attrs), false) => {
                        for attr in &self.extra_attrs {
                            extra_attrs.push(attr);
                        }
                        Some(extra_attrs)
                    }
                    (Some(extra_attrs), true) => Some(extra_attrs),
                    (None, false) => Some(self.extra_attrs.iter().collect()),
                    (None, true) => None,
                },
            )
        }
        #[cfg(not(feature = "ssr"))]
        {
            _ = extra_attrs;
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
