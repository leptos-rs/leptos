use super::{add_attr::AddAnyAttr, Position, PositionState, RenderHtml};
use crate::{
    html::attribute::Attribute,
    hydration::Cursor,
    ssr::StreamBuilder,
    view::{iterators::OptionState, Mountable, Render, Renderer},
};
use either_of::Either;
use throw_error::Error as AnyError;

impl<R, T, E> Render<R> for Result<T, E>
where
    T: Render<R>,
    R: Renderer,
    E: Into<AnyError> + 'static,
{
    type State = ResultState<T, R>;

    fn build(self) -> Self::State {
        let (state, error) = match self {
            Ok(view) => (Either::Left(view.build()), None),
            Err(e) => (
                Either::Right(Render::<R>::build(())),
                Some(throw_error::throw(e.into())),
            ),
        };
        ResultState { state, error }
    }

    fn rebuild(self, state: &mut Self::State) {
        match (&mut state.state, self) {
            // both errors: throw the new error and replace
            (Either::Right(_), Err(new)) => {
                state.error = Some(throw_error::throw(new.into()))
            }
            // both Ok: need to rebuild child
            (Either::Left(old), Ok(new)) => {
                T::rebuild(new, old);
            }
            // Ok => Err: unmount, replace with marker, and throw
            (Either::Left(old), Err(err)) => {
                let mut new_state = Render::<R>::build(());
                old.insert_before_this(&mut new_state);
                old.unmount();
                state.state = Either::Right(new_state);
                state.error = Some(throw_error::throw(err));
            }
            // Err => Ok: clear error and build
            (Either::Right(old), Ok(new)) => {
                if let Some(err) = state.error.take() {
                    throw_error::clear(&err);
                }
                let mut new_state = new.build();
                old.insert_before_this(&mut new_state);
                old.unmount();
                state.state = Either::Left(new_state);
            }
        }
    }
}

/// View state for a `Result<_, _>` view.
pub struct ResultState<T, R>
where
    T: Render<R>,
    R: Renderer,
{
    /// The view state.
    state: OptionState<T, R>,
    error: Option<throw_error::ErrorId>,
}

impl<T, R> Drop for ResultState<T, R>
where
    T: Render<R>,
    R: Renderer,
{
    fn drop(&mut self) {
        // when the state is cleared, unregister this error; this item is being dropped and its
        // error should no longer be shown
        if let Some(e) = self.error.take() {
            throw_error::clear(&e);
        }
    }
}

impl<T, R> Mountable<R> for ResultState<T, R>
where
    T: Render<R>,
    R: Renderer,
{
    fn unmount(&mut self) {
        self.state.unmount();
    }

    fn mount(&mut self, parent: &R::Element, marker: Option<&R::Node>) {
        self.state.mount(parent, marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable<R>) -> bool {
        self.state.insert_before_this(child)
    }
}

impl<R, T, E> AddAnyAttr<R> for Result<T, E>
where
    T: AddAnyAttr<R>,
    R: Renderer,
    E: Into<AnyError> + Send + 'static,
{
    type Output<SomeNewAttr: Attribute<R>> =
        Result<<T as AddAnyAttr<R>>::Output<SomeNewAttr>, E>;

    fn add_any_attr<NewAttr: Attribute<R>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<R>,
    {
        self.map(|inner| inner.add_any_attr(attr))
    }
}

impl<R, T, E> RenderHtml<R> for Result<T, E>
where
    T: RenderHtml<R>,
    R: Renderer,
    E: Into<AnyError> + Send + 'static,
{
    type AsyncOutput = Result<T::AsyncOutput, E>;

    const MIN_LENGTH: usize = T::MIN_LENGTH;

    fn dry_resolve(&mut self) {
        if let Ok(inner) = self.as_mut() {
            inner.dry_resolve()
        }
    }

    async fn resolve(self) -> Self::AsyncOutput {
        match self {
            Ok(view) => Ok(view.resolve().await),
            Err(e) => Err(e),
        }
    }

    fn html_len(&self) -> usize {
        match self {
            Ok(i) => i.html_len() + 3,
            Err(_) => 0,
        }
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut super::Position,
        escape: bool,
    ) {
        match self {
            Ok(inner) => inner.to_html_with_buf(buf, position, escape),
            Err(e) => {
                throw_error::throw(e);
            }
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
    ) where
        Self: Sized,
    {
        match self {
            Ok(inner) => inner
                .to_html_async_with_buf::<OUT_OF_ORDER>(buf, position, escape),
            Err(e) => {
                throw_error::throw(e);
            }
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        let (state, error) = match self {
            Ok(view) => (
                Either::Left(view.hydrate::<FROM_SERVER>(cursor, position)),
                None,
            ),
            Err(e) => {
                let state = RenderHtml::<R>::hydrate::<FROM_SERVER>(
                    (),
                    cursor,
                    position,
                );
                (Either::Right(state), Some(throw_error::throw(e.into())))
            }
        };
        ResultState { state, error }
    }
}
