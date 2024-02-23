use super::{Mountable, Position, PositionState, Render, RenderHtml};
use crate::{hydration::Cursor, renderer::Renderer};
use std::{
    any::{Any, TypeId},
    marker::PhantomData,
};

pub struct AnyView<R>
where
    R: Renderer,
{
    type_id: TypeId,
    value: Box<dyn Any>,
    to_html: fn(Box<dyn Any>, &mut String, &mut Position),
    build: fn(Box<dyn Any>) -> AnyViewState<R>,
    rebuild: fn(TypeId, Box<dyn Any>, &mut AnyViewState<R>),
    #[allow(clippy::type_complexity)]
    hydrate_from_server:
        fn(Box<dyn Any>, &Cursor<R>, &PositionState) -> AnyViewState<R>,
    #[allow(clippy::type_complexity)]
    hydrate_from_template:
        fn(Box<dyn Any>, &Cursor<R>, &PositionState) -> AnyViewState<R>,
}

pub struct AnyViewState<R>
where
    R: Renderer,
{
    type_id: TypeId,
    state: Box<dyn Any>,
    unmount: fn(&mut dyn Any),
    mount: fn(&mut dyn Any, parent: &R::Element, marker: Option<&R::Node>),
    insert_before_this:
        fn(&dyn Any, parent: &R::Element, child: &mut dyn Mountable<R>) -> bool,
    rndr: PhantomData<R>,
}

pub trait IntoAny<R>
where
    R: Renderer,
{
    fn into_any(self) -> AnyView<R>;
}

fn mount_any<R, T>(
    state: &mut dyn Any,
    parent: &R::Element,
    marker: Option<&R::Node>,
) where
    T: Render<R>,
    T::State: 'static,
    R: Renderer,
{
    let state = state
        .downcast_mut::<T::State>()
        .expect("AnyViewState::as_mountable couldn't downcast state");
    state.mount(parent, marker)
}

fn unmount_any<R, T>(state: &mut dyn Any)
where
    T: Render<R>,
    T::State: 'static,
    R: Renderer,
{
    let state = state
        .downcast_mut::<T::State>()
        .expect("AnyViewState::unmount couldn't downcast state");
    state.unmount();
}

fn insert_before_this<R, T>(
    state: &dyn Any,
    parent: &R::Element,
    child: &mut dyn Mountable<R>,
) -> bool
where
    T: Render<R>,
    T::State: 'static,
    R: Renderer + 'static,
{
    let state = state
        .downcast_ref::<T::State>()
        .expect("AnyViewState::opening_node couldn't downcast state");
    state.insert_before_this(parent, child)
}

impl<T, R> IntoAny<R> for T
where
    T: RenderHtml<R> + 'static,
    T::State: 'static,
    R: Renderer + 'static,
{
    // inlining allows the compiler to remove the unused functions
    // i.e., doesn't ship HTML-generating code that isn't used
    #[inline(always)]
    fn into_any(self) -> AnyView<R> {
        let value = Box::new(self) as Box<dyn Any>;

        let to_html =
            |value: Box<dyn Any>, buf: &mut String, position: &mut Position| {
                let value = value
                    .downcast::<T>()
                    .expect("AnyView::to_html could not be downcast");
                value.to_html_with_buf(buf, position);
            };
        let build = |value: Box<dyn Any>| {
            let value = value
                .downcast::<T>()
                .expect("AnyView::build couldn't downcast");
            let state = Box::new(value.build());

            AnyViewState {
                type_id: TypeId::of::<T>(),
                state,
                rndr: PhantomData,
                mount: mount_any::<R, T>,
                unmount: unmount_any::<R, T>,
                insert_before_this: insert_before_this::<R, T>,
            }
        };
        let hydrate_from_server =
            |value: Box<dyn Any>,
             cursor: &Cursor<R>,
             position: &PositionState| {
                let value = value
                    .downcast::<T>()
                    .expect("AnyView::hydrate_from_server couldn't downcast");
                let state = Box::new(value.hydrate::<true>(cursor, position));

                AnyViewState {
                    type_id: TypeId::of::<T>(),
                    state,
                    rndr: PhantomData,
                    mount: mount_any::<R, T>,
                    unmount: unmount_any::<R, T>,
                    insert_before_this: insert_before_this::<R, T>,
                }
            };
        let hydrate_from_template =
            |value: Box<dyn Any>,
             cursor: &Cursor<R>,
             position: &PositionState| {
                let value = value
                    .downcast::<T>()
                    .expect("AnyView::hydrate_from_server couldn't downcast");
                let state = Box::new(value.hydrate::<true>(cursor, position));

                AnyViewState {
                    type_id: TypeId::of::<T>(),
                    state,
                    rndr: PhantomData,
                    mount: mount_any::<R, T>,
                    unmount: unmount_any::<R, T>,
                    insert_before_this: insert_before_this::<R, T>,
                }
            };
        let rebuild = |new_type_id: TypeId,
                       value: Box<dyn Any>,
                       state: &mut AnyViewState<R>| {
            let value = value
                .downcast::<T>()
                .expect("AnyView::rebuild couldn't downcast value");
            if new_type_id == state.type_id {
                let state = state
                    .state
                    .downcast_mut()
                    .expect("AnyView::rebuild couldn't downcast state");
                value.rebuild(state);
            } else {
                let new = value.into_any().build();

                // TODO mount new state
                /*R::mount_before(&mut new, state.placeholder.as_ref());*/
                state.unmount();
                *state = new;
            }
        };
        AnyView {
            type_id: TypeId::of::<T>(),
            value,
            to_html,
            build,
            rebuild,
            hydrate_from_server,
            hydrate_from_template,
        }
    }
}

impl<R> Render<R> for AnyView<R>
where
    R: Renderer + 'static,
{
    type State = AnyViewState<R>;
    type FallibleState = Self::State;

    fn build(self) -> Self::State {
        (self.build)(self.value)
    }

    fn rebuild(self, state: &mut Self::State) {
        (self.rebuild)(self.type_id, self.value, state)
    }

    fn try_build(self) -> crate::error::Result<Self::FallibleState> {
        todo!()
    }

    fn try_rebuild(
        self,
        _state: &mut Self::FallibleState,
    ) -> crate::error::Result<()> {
        todo!()
    }
}

impl<R> RenderHtml<R> for AnyView<R>
where
    R: Renderer + 'static,
{
    const MIN_LENGTH: usize = 0;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        (self.to_html)(self.value, buf, position);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        if FROM_SERVER {
            (self.hydrate_from_server)(self.value, cursor, position)
        } else {
            (self.hydrate_from_template)(self.value, cursor, position)
        }
    }
}

impl<R> Mountable<R> for AnyViewState<R>
where
    R: Renderer + 'static,
{
    fn unmount(&mut self) {
        (self.unmount)(&mut *self.state)
    }

    fn mount(&mut self, parent: &R::Element, marker: Option<&R::Node>) {
        (self.mount)(&mut *self.state, parent, marker)
    }

    fn insert_before_this(
        &self,
        parent: &<R as Renderer>::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool {
        (self.insert_before_this)(self, parent, child)
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
