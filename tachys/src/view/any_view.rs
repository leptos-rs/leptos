#[cfg(feature = "ssr")]
use super::MarkBranch;
use super::{
    add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
    RenderHtml,
};
use crate::{
    html::attribute::Attribute, hydration::Cursor, ssr::StreamBuilder,
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
    build: fn(Box<dyn Any>) -> AnyViewState,
    rebuild: fn(TypeId, Box<dyn Any>, &mut AnyViewState),
    // Without erasure, tuples of attrs created by default cause too much type explosion to enable.
    #[cfg(erase_components)]
    add_any_attr: fn(
        Box<dyn Any>,
        crate::html::attribute::any_attribute::AnyAttribute,
    ) -> AnyView,
    // The fields below are cfg-gated so they will not be included in WASM bundles if not needed.
    // Ordinarily, the compiler can simply omit this dead code because the methods are not called.
    // With this type-erased wrapper, however, the compiler is not *always* able to correctly
    // eliminate that code.
    #[cfg(feature = "ssr")]
    html_len: usize,
    #[cfg(feature = "ssr")]
    to_html: fn(Box<dyn Any>, &mut String, &mut Position, bool, bool),
    #[cfg(feature = "ssr")]
    to_html_async:
        fn(Box<dyn Any>, &mut StreamBuilder, &mut Position, bool, bool),
    #[cfg(feature = "ssr")]
    to_html_async_ooo:
        fn(Box<dyn Any>, &mut StreamBuilder, &mut Position, bool, bool),
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

impl<T> IntoAny for T
where
    T: Send,
    T: RenderHtml + 'static,
    T::State: 'static,
{
    fn into_any(self) -> AnyView {
        #[cfg(feature = "ssr")]
        let html_len = self.html_len();

        let value = Box::new(self) as Box<dyn Any + Send>;

        match value.downcast::<AnyView>() {
            // if it's already an AnyView, we don't need to double-wrap it
            Ok(any_view) => *any_view,
            Err(value) => {
                #[cfg(feature = "ssr")]
                let dry_resolve = |value: &mut Box<dyn Any + Send>| {
                    let value = value
                        .downcast_mut::<T>()
                        .expect("AnyView::resolve could not be downcast");
                    value.dry_resolve();
                };

                #[cfg(feature = "ssr")]
                let resolve = |value: Box<dyn Any>| {
                    let value = value
                        .downcast::<T>()
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
                     mark_branches: bool| {
                        let type_id = mark_branches
                            .then(|| format!("{:?}", TypeId::of::<T>()))
                            .unwrap_or_default();
                        let value = value
                            .downcast::<T>()
                            .expect("AnyView::to_html could not be downcast");
                        if mark_branches {
                            buf.open_branch(&type_id);
                        }
                        value.to_html_with_buf(
                            buf,
                            position,
                            escape,
                            mark_branches,
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
                     mark_branches: bool| {
                        let type_id = mark_branches
                            .then(|| format!("{:?}", TypeId::of::<T>()))
                            .unwrap_or_default();
                        let value = value
                            .downcast::<T>()
                            .expect("AnyView::to_html could not be downcast");
                        if mark_branches {
                            buf.open_branch(&type_id);
                        }
                        value.to_html_async_with_buf::<false>(
                            buf,
                            position,
                            escape,
                            mark_branches,
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
                     mark_branches: bool| {
                        let value = value
                            .downcast::<T>()
                            .expect("AnyView::to_html could not be downcast");
                        value.to_html_async_with_buf::<true>(
                            buf,
                            position,
                            escape,
                            mark_branches,
                        );
                    };
                let build = |value: Box<dyn Any>| {
                    let value = value
                        .downcast::<T>()
                        .expect("AnyView::build couldn't downcast");
                    let state = Box::new(value.build());

                    AnyViewState {
                        type_id: TypeId::of::<T>(),
                        state,

                        mount: mount_any::<T>,
                        unmount: unmount_any::<T>,
                        insert_before_this: insert_before_this::<T>,
                    }
                };
                #[cfg(feature = "hydrate")]
                let hydrate_from_server =
                    |value: Box<dyn Any>,
                     cursor: &Cursor,
                     position: &PositionState| {
                        let value = value.downcast::<T>().expect(
                            "AnyView::hydrate_from_server couldn't downcast",
                        );
                        let state =
                            Box::new(value.hydrate::<true>(cursor, position));

                        AnyViewState {
                            type_id: TypeId::of::<T>(),
                            state,

                            mount: mount_any::<T>,
                            unmount: unmount_any::<T>,
                            insert_before_this: insert_before_this::<T>,
                        }
                    };

                let rebuild =
                    |new_type_id: TypeId,
                     value: Box<dyn Any>,
                     state: &mut AnyViewState| {
                        let value = value
                            .downcast::<T>()
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

                // Without erasure, tuples of attrs created by default cause too much type explosion to enable.
                #[cfg(erase_components)]
                let add_any_attr = |value: Box<dyn Any>, attr: crate::html::attribute::any_attribute::AnyAttribute| {
                    let value = value
                        .downcast::<T>()
                        .expect("AnyView::add_any_attr could not be downcast");
                    value.add_any_attr(attr).into_any()
                };

                AnyView {
                    type_id: TypeId::of::<T>(),
                    value,
                    build,
                    rebuild,
                    // Without erasure, tuples of attrs created by default cause too much type explosion to enable.
                    #[cfg(erase_components)]
                    add_any_attr,
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
    type Output<SomeNewAttr: Attribute> = Self;

    #[allow(unused_variables)]
    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        // Without erasure, tuples of attrs created by default cause too much type explosion to enable.
        #[cfg(erase_components)]
        {
            use crate::html::attribute::any_attribute::IntoAnyAttribute;

            let attr = attr.into_cloneable_owned();
            (self.add_any_attr)(self.value, attr.into_any_attr())
        }
        #[cfg(not(erase_components))]
        {
            self
        }
    }
}

impl RenderHtml for AnyView {
    type AsyncOutput = Self;

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
    ) {
        #[cfg(feature = "ssr")]
        (self.to_html)(self.value, buf, position, escape, mark_branches);
        #[cfg(not(feature = "ssr"))]
        {
            _ = mark_branches;
            _ = buf;
            _ = position;
            _ = escape;
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
            );
        } else {
            (self.to_html_async)(
                self.value,
                buf,
                position,
                escape,
                mark_branches,
            );
        }
        #[cfg(not(feature = "ssr"))]
        {
            _ = buf;
            _ = position;
            _ = escape;
            _ = mark_branches;
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
