use super::{either::Either, NeverError, Position, PositionState, RenderHtml};
use crate::{
    hydration::Cursor,
    ssr::StreamBuilder,
    view::{Mountable, Render, Renderer},
};
use std::marker::PhantomData;

impl<R, T, E> Render<R> for Result<T, E>
where
    T: Render<R>,
    R: Renderer,
{
    type State = <Option<T> as Render<R>>::State;
    type FallibleState = T::State;
    type Error = E;

    fn build(self) -> Self::State {
        self.ok().build()
    }

    fn rebuild(self, state: &mut Self::State) {
        self.ok().rebuild(state);
    }

    fn try_build(self) -> Result<Self::FallibleState, Self::Error> {
        let inner = self?;
        let state = inner.build();
        Ok(state)
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> Result<(), Self::Error> {
        let inner = self?;
        inner.rebuild(state);
        Ok(())
    }
}

impl<R, T, E> RenderHtml<R> for Result<T, E>
where
    T: RenderHtml<R>,
    R: Renderer,
    R::Element: Clone,
    R::Node: Clone,
{
    const MIN_LENGTH: usize = T::MIN_LENGTH;

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut super::Position,
    ) {
        if let Ok(inner) = self {
            inner.to_html_with_buf(buf, position);
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        if let Ok(inner) = self {
            inner.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        self.ok().hydrate::<FROM_SERVER>(cursor, position)
    }
}

pub trait TryCatchBoundary<Fal, FalFn, Rndr>
where
    Self: Sized + Render<Rndr>,
    Fal: Render<Rndr>,
    FalFn: FnMut(Self::Error) -> Fal,
    Rndr: Renderer,
{
    fn catch(self, fallback: FalFn) -> Try<Self, Fal, FalFn, Rndr>;
}

impl<T, Fal, FalFn, Rndr> TryCatchBoundary<Fal, FalFn, Rndr> for T
where
    T: Sized + Render<Rndr>,
    Fal: Render<Rndr>,
    FalFn: FnMut(Self::Error) -> Fal,
    Rndr: Renderer,
{
    fn catch(self, fallback: FalFn) -> Try<Self, Fal, FalFn, Rndr> {
        Try::new(fallback, self)
    }
}

pub struct Try<T, Fal, FalFn, Rndr>
where
    T: Render<Rndr>,
    Fal: Render<Rndr>,
    FalFn: FnMut(T::Error) -> Fal,
    Rndr: Renderer,
{
    child: T,
    fal: FalFn,
    ty: PhantomData<Rndr>,
}

impl<T, Fal, FalFn, Rndr> Try<T, Fal, FalFn, Rndr>
where
    T: Render<Rndr>,
    Fal: Render<Rndr>,
    FalFn: FnMut(T::Error) -> Fal,
    Rndr: Renderer,
{
    pub fn new(fallback: FalFn, child: T) -> Self {
        Self {
            child,
            fal: fallback,
            ty: PhantomData,
        }
    }
}

impl<T, Fal, FalFn, Rndr> Render<Rndr> for Try<T, Fal, FalFn, Rndr>
where
    T: Render<Rndr>,
    Fal: Render<Rndr>,
    FalFn: FnMut(T::Error) -> Fal,
    Rndr: Renderer,
{
    type State = TryState<T, Fal, Rndr>;
    type FallibleState = Self::State;
    type Error = NeverError;

    fn build(mut self) -> Self::State {
        let state = match self.child.try_build() {
            Ok(inner) => Either::Left(inner),
            Err(e) => Either::Right((self.fal)(e).build()),
        };
        let marker = Rndr::create_placeholder();
        TryState { state, marker }
    }

    fn rebuild(mut self, state: &mut Self::State) {
        let marker = state.marker.as_ref();
        match &mut state.state {
            Either::Left(ref mut old) => {
                if let Err(e) = self.child.try_rebuild(old) {
                    old.unmount();
                    let mut new_state = (self.fal)(e).build();
                    Rndr::mount_before(&mut new_state, marker);
                    state.state = Either::Right(new_state);
                }
            }
            Either::Right(old) => match self.child.try_build() {
                Ok(mut new_state) => {
                    old.unmount();
                    Rndr::mount_before(&mut new_state, marker);
                    state.state = Either::Left(new_state);
                }
                Err(e) => {
                    (self.fal)(e).rebuild(old);
                }
            },
        }
    }

    fn try_build(self) -> Result<Self::FallibleState, Self::Error> {
        Ok(self.build())
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> Result<(), Self::Error> {
        Ok(self.rebuild(state))
    }
}

// TODO RenderHtml implementation for ErrorBoundary
impl<T, Fal, FalFn, Rndr> RenderHtml<Rndr> for Try<T, Fal, FalFn, Rndr>
where
    T: Render<Rndr>,
    Fal: RenderHtml<Rndr>,
    FalFn: FnMut(T::Error) -> Fal,
    Rndr: Renderer,
    Rndr::Element: Clone,
    Rndr::Node: Clone,
{
    const MIN_LENGTH: usize = Fal::MIN_LENGTH;

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut super::Position,
    ) {
        todo!()
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut crate::ssr::StreamBuilder,
        position: &mut super::Position,
    ) where
        Self: Sized,
    {
        todo!()
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &crate::hydration::Cursor<Rndr>,
        position: &super::PositionState,
    ) -> Self::State {
        todo!()
    }
}

pub struct TryState<T, Fal, Rndr>
where
    T: Render<Rndr>,
    Fal: Render<Rndr>,
    Rndr: Renderer,
{
    state: Either<T::FallibleState, Fal::State>,
    marker: Rndr::Placeholder,
}

impl<T, Fal, Rndr> Mountable<Rndr> for TryState<T, Fal, Rndr>
where
    T: Render<Rndr>,
    Fal: Render<Rndr>,
    Rndr: Renderer,
{
    fn unmount(&mut self) {
        match &mut self.state {
            Either::Left(left) => left.unmount(),
            Either::Right(right) => right.unmount(),
        }
        self.marker.unmount();
    }

    fn mount(
        &mut self,
        parent: &<Rndr as Renderer>::Element,
        marker: Option<&<Rndr as Renderer>::Node>,
    ) {
        self.marker.mount(parent, marker);
        match &mut self.state {
            Either::Left(left) => {
                left.mount(parent, Some(self.marker.as_ref()))
            }
            Either::Right(right) => {
                right.mount(parent, Some(self.marker.as_ref()))
            }
        }
    }

    fn insert_before_this(
        &self,
        parent: &<Rndr as Renderer>::Element,
        child: &mut dyn Mountable<Rndr>,
    ) -> bool {
        match &self.state {
            Either::Left(left) => left.insert_before_this(parent, child),
            Either::Right(right) => right.insert_before_this(parent, child),
        }
    }
}
