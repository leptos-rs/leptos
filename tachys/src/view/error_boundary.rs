use super::{add_attr::AddAnyAttr, Position, PositionState, RenderHtml};
use crate::{
    html::attribute::Attribute,
    hydration::Cursor,
    ssr::StreamBuilder,
    view::{Mountable, Render, Renderer},
};
use throw_error::Error as AnyError;

impl<R, T, E> Render<R> for Result<T, E>
where
    T: Render<R>,
    R: Renderer,
    E: Into<AnyError> + 'static,
{
    type State = ResultState<T::State, R>;

    fn build(self) -> Self::State {
        let placeholder = R::create_placeholder();
        let state = match self {
            Ok(view) => Ok(view.build()),
            Err(e) => Err(throw_error::throw(e.into())),
        };
        ResultState { placeholder, state }
    }

    fn rebuild(self, state: &mut Self::State) {
        match (&mut state.state, self) {
            // both errors: throw the new error and replace
            (Err(prev), Err(new)) => {
                *prev = throw_error::throw(new.into());
            }
            // both Ok: need to rebuild child
            (Ok(old), Ok(new)) => {
                T::rebuild(new, old);
            }
            // Ok => Err: unmount, replace with marker, and throw
            (Ok(old), Err(err)) => {
                old.unmount();
                state.state = Err(throw_error::throw(err));
            }
            // Err => Ok: clear error and build
            (Err(err), Ok(new)) => {
                throw_error::clear(err);
                let mut new_state = new.build();
                R::try_mount_before(&mut new_state, state.placeholder.as_ref());
                state.state = Ok(new_state);
            }
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
    state: Result<T, throw_error::ErrorId>,
}

impl<T, R> Drop for ResultState<T, R>
where
    T: Mountable<R>,
    R: Renderer,
{
    fn drop(&mut self) {
        // when the state is cleared, unregister this error; this item is being dropped and its
        // error should no longer be shown
        if let Err(e) = &self.state {
            throw_error::clear(e);
        }
    }
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
        self.placeholder.unmount();
    }

    fn mount(&mut self, parent: &R::Element, marker: Option<&R::Node>) {
        self.placeholder.mount(parent, marker);
        if let Ok(ref mut state) = self.state {
            state.mount(parent, Some(self.placeholder.as_ref()));
        }
    }

    fn insert_before_this(&self, 
        child: &mut dyn Mountable<R>,
    ) -> bool {
        if self.state.as_ref().map(|n| n.insert_before_this(child)) == Ok(true)
        {
            true
        } else {
            self.placeholder.insert_before_this(child)
        }
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
    ) {
        match self {
            Ok(inner) => inner.to_html_with_buf(buf, position),
            Err(e) => {
                throw_error::throw(e);
            }
        }
        buf.push_str("<!>");
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        match self {
            Ok(inner) => {
                inner.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position)
            }
            Err(e) => {
                throw_error::throw(e);
            }
        }
        buf.push_sync("<!>");
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        // hydrate the state, if it exists
        let state = self
            .map(|s| s.hydrate::<FROM_SERVER>(cursor, position))
            .map_err(|e| throw_error::throw(e.into()));

        let placeholder = cursor.next_placeholder(position);

        ResultState { placeholder, state }
    }
}
