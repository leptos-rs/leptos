use super::{Position, PositionState, RenderHtml};
use crate::{
    hydration::Cursor,
    renderer::CastFrom,
    ssr::StreamBuilder,
    view::{Mountable, Render, Renderer},
};
use any_error::Error as AnyError;
use std::marker::PhantomData;

impl<R, T, E> Render<R> for Result<T, E>
where
    T: Render<R>,
    R: Renderer,
    E: Into<AnyError> + 'static,
{
    type State = ResultState<T::State, R>;
    type FallibleState = T::State;
    type AsyncOutput = Result<T::AsyncOutput, E>;

    fn build(self) -> Self::State {
        let placeholder = R::create_placeholder();
        let state = match self {
            Ok(view) => Ok(view.build()),
            Err(e) => Err(any_error::throw(e.into())),
        };
        ResultState { placeholder, state }
    }

    fn rebuild(self, state: &mut Self::State) {
        match (&mut state.state, self) {
            // both errors: throw the new error and replace
            (Err(prev), Err(new)) => {
                *prev = any_error::throw(new.into());
            }
            // both Ok: need to rebuild child
            (Ok(old), Ok(new)) => {
                T::rebuild(new, old);
            }
            // Ok => Err: unmount, replace with marker, and throw
            (Ok(old), Err(err)) => {
                old.unmount();
                state.state = Err(any_error::throw(err));
            }
            // Err => Ok: clear error and build
            (Err(err), Ok(new)) => {
                any_error::clear(err);
                let mut new_state = new.build();
                R::mount_before(&mut new_state, state.placeholder.as_ref());
                state.state = Ok(new_state);
            }
        }
    }

    fn try_build(self) -> any_error::Result<Self::FallibleState> {
        todo!()
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> any_error::Result<()> {
        todo!()
    }

    async fn resolve(self) -> Self::AsyncOutput {
        match self {
            Ok(view) => Ok(view.resolve().await),
            Err(e) => Err(e),
        }
    }
}

/// View state for a `Result<_, _>` view.
pub struct ResultState<T, R>
where
    T: Mountable<R>,
    R: Renderer,
{
    /// Marks the location of this view.
    placeholder: R::Placeholder,
    /// The view state.
    state: Result<T, any_error::ErrorId>,
}

impl<T, R> Mountable<R> for ResultState<T, R>
where
    T: Mountable<R>,
    R: Renderer,
{
    fn unmount(&mut self) {
        if let Ok(ref mut state) = self.state {
            state.unmount();
        }
        // TODO investigate: including this seems to break error boundaries, although it doesn't
        // make sense to me why it would be a problem
        // self.placeholder.unmount();
    }

    fn mount(&mut self, parent: &R::Element, marker: Option<&R::Node>) {
        self.placeholder.mount(parent, marker);
        if let Ok(ref mut state) = self.state {
            state.mount(parent, Some(self.placeholder.as_ref()));
        }
    }

    fn insert_before_this(
        &self,
        parent: &R::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool {
        if self
            .state
            .as_ref()
            .map(|n| n.insert_before_this(parent, child))
            == Ok(true)
        {
            true
        } else {
            self.placeholder.insert_before_this(parent, child)
        }
    }
}

impl<R, T, E> RenderHtml<R> for Result<T, E>
where
    T: RenderHtml<R>,
    R: Renderer,
    E: Into<AnyError> + 'static,
{
    const MIN_LENGTH: usize = T::MIN_LENGTH;

    fn html_len(&self) -> usize {
        match self {
            Ok(i) => i.html_len(),
            Err(_) => 0,
        }
    }

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
        // hydrate the state, if it exists
        let state = self
            .map(|s| s.hydrate::<FROM_SERVER>(cursor, position))
            .map_err(|e| any_error::throw(e.into()));

        // pull the placeholder
        if position.get() == Position::FirstChild {
            cursor.child();
        } else {
            cursor.sibling();
        }
        let placeholder = cursor.current().to_owned();
        let placeholder = R::Placeholder::cast_from(placeholder).unwrap();
        position.set(Position::NextChild);

        ResultState { placeholder, state }
    }
}

pub trait TryCatchBoundary<Fal, FalFn, Rndr>
where
    Self: Sized + Render<Rndr>,
    Fal: Render<Rndr>,
    FalFn: FnMut(AnyError) -> Fal,
    Rndr: Renderer,
{
    fn catch(self, fallback: FalFn) -> Try<Self, Fal, FalFn, Rndr>;
}

impl<T, Fal, FalFn, Rndr> TryCatchBoundary<Fal, FalFn, Rndr> for T
where
    T: Sized + Render<Rndr>,
    Fal: Render<Rndr>,
    FalFn: FnMut(AnyError) -> Fal,
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
    FalFn: FnMut(AnyError) -> Fal,
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
    FalFn: FnMut(AnyError) -> Fal,
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
    FalFn: FnMut(AnyError) -> Fal,
    Rndr: Renderer,
{
    type State = TryState<T, Fal, Rndr>;
    type FallibleState = Self::State;
    type AsyncOutput = Try<T::AsyncOutput, Fal, FalFn, Rndr>;

    fn build(mut self) -> Self::State {
        let inner = match self.child.try_build() {
            Ok(inner) => TryStateState::Success(Some(inner)),
            Err(e) => TryStateState::InitialFail((self.fal)(e).build()),
        };
        let marker = Rndr::create_placeholder();
        TryState { inner, marker }
    }

    fn rebuild(mut self, state: &mut Self::State) {
        let marker = state.marker.as_ref();
        let res = match &mut state.inner {
            TryStateState::Success(old) => {
                let old_unwrapped =
                    old.as_mut().expect("children removed before expected");
                if let Err(e) = self.child.try_rebuild(old_unwrapped) {
                    old_unwrapped.unmount();
                    let mut new_state = (self.fal)(e).build();
                    Rndr::mount_before(&mut new_state, marker);
                    Some(Err((old.take(), new_state)))
                } else {
                    None
                }
            }
            TryStateState::InitialFail(old) => match self.child.try_build() {
                Err(e) => {
                    (self.fal)(e).rebuild(old);
                    None
                }
                Ok(mut new_state) => {
                    old.unmount();
                    Rndr::mount_before(&mut new_state, marker);
                    Some(Ok(new_state))
                }
            },
            TryStateState::SubsequentFail { fallback, .. } => {
                match self.child.try_build() {
                    Err(e) => {
                        (self.fal)(e).rebuild(fallback);
                        None
                    }
                    Ok(mut new_children) => {
                        fallback.unmount();
                        Rndr::mount_before(&mut new_children, marker);
                        Some(Ok(new_children))
                    }
                }
            }
        };
        match res {
            Some(Ok(new_children)) => {
                state.inner = TryStateState::Success(Some(new_children))
            }
            Some(Err((_children, fallback))) => {
                state.inner = TryStateState::SubsequentFail {
                    _children,
                    fallback,
                }
            }
            None => {}
        }
    }

    fn try_build(self) -> any_error::Result<Self::FallibleState> {
        Ok(self.build())
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> any_error::Result<()> {
        self.rebuild(state);
        Ok(())
    }

    async fn resolve(self) -> Self::AsyncOutput {
        todo!()
    }
}

// TODO RenderHtml implementation for ErrorBoundary
impl<T, Fal, FalFn, Rndr> RenderHtml<Rndr> for Try<T, Fal, FalFn, Rndr>
where
    T: Render<Rndr>,
    Fal: RenderHtml<Rndr>,
    FalFn: FnMut(AnyError) -> Fal,
    Rndr: Renderer,
{
    const MIN_LENGTH: usize = Fal::MIN_LENGTH;

    fn to_html_with_buf(
        self,
        _buf: &mut String,
        _position: &mut super::Position,
    ) {
        todo!()
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        _buf: &mut crate::ssr::StreamBuilder,
        _position: &mut super::Position,
    ) where
        Self: Sized,
    {
        todo!()
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _cursor: &crate::hydration::Cursor<Rndr>,
        _position: &super::PositionState,
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
    inner: TryStateState<T, Fal, Rndr>,
    marker: Rndr::Placeholder,
}

enum TryStateState<T, Fal, Rndr>
where
    T: Render<Rndr>,
    Fal: Render<Rndr>,
    Rndr: Renderer,
{
    Success(Option<T::FallibleState>),
    InitialFail(Fal::State),
    SubsequentFail {
        // they exist here only to be kept alive
        // this is important if the children are holding some reactive state that
        // caused the error boundary to be triggered in the first place
        _children: Option<T::FallibleState>,
        fallback: Fal::State,
    },
}

impl<T, Fal, Rndr> Mountable<Rndr> for TryState<T, Fal, Rndr>
where
    T: Render<Rndr>,
    Fal: Render<Rndr>,
    Rndr: Renderer,
{
    fn unmount(&mut self) {
        match &mut self.inner {
            TryStateState::Success(m) => m
                .as_mut()
                .expect("children removed before expected")
                .unmount(),
            TryStateState::InitialFail(m) => m.unmount(),
            TryStateState::SubsequentFail { fallback, .. } => {
                fallback.unmount()
            }
        }
        self.marker.unmount();
    }

    fn mount(
        &mut self,
        parent: &<Rndr as Renderer>::Element,
        marker: Option<&<Rndr as Renderer>::Node>,
    ) {
        self.marker.mount(parent, marker);
        match &mut self.inner {
            TryStateState::Success(m) => m
                .as_mut()
                .expect("children removed before expected")
                .mount(parent, Some(self.marker.as_ref())),
            TryStateState::InitialFail(m) => {
                m.mount(parent, Some(self.marker.as_ref()))
            }
            TryStateState::SubsequentFail { fallback, .. } => {
                fallback.mount(parent, Some(self.marker.as_ref()))
            }
        }
    }

    fn insert_before_this(
        &self,
        parent: &<Rndr as Renderer>::Element,
        child: &mut dyn Mountable<Rndr>,
    ) -> bool {
        match &self.inner {
            TryStateState::Success(m) => m
                .as_ref()
                .expect("children removed before expected")
                .insert_before_this(parent, child),
            TryStateState::InitialFail(m) => {
                m.insert_before_this(parent, child)
            }
            TryStateState::SubsequentFail { fallback, .. } => {
                fallback.insert_before_this(parent, child)
            }
        }
    }
}
