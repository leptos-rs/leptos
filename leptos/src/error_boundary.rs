use crate::{children::TypedChildren, IntoView};
use hydration_context::{SerializedDataId, SharedContext};
use leptos_macro::component;
use reactive_graph::{
    computed::ArcMemo,
    owner::Owner,
    signal::ArcRwSignal,
    traits::{Get, GetUntracked, Update, With, WithUntracked},
};
use rustc_hash::FxHashMap;
use std::{marker::PhantomData, sync::Arc};
use tachys::{
    hydration::Cursor,
    renderer::{CastFrom, Renderer},
    ssr::StreamBuilder,
    view::{Mountable, Position, PositionState, Render, RenderHtml},
};
use throw_error::{Error, ErrorHook, ErrorId};

///
/// ## Beginner's Tip: ErrorBoundary Requires Your Error To Implement std::error::Error.
/// `ErrorBoundary` requires your `Result<T,E>` to implement [IntoView](https://docs.rs/leptos/latest/leptos/trait.IntoView.html).
/// `Result<T,E>` only implements `IntoView` if `E` implements [std::error::Error](https://doc.rust-lang.org/std/error/trait.Error.html).
/// So, for instance, if you pass a `Result<T,String>` where `T` implements [IntoView](https://docs.rs/leptos/latest/leptos/trait.IntoView.html)
/// and attempt to render the error for the purposes of `ErrorBoundary` you'll get a compiler error like this.
///
/// ```rust,ignore
/// error[E0599]: the method `into_view` exists for enum `Result<ViewableLoginFlow, String>`, but its trait bounds were not satisfied
///    --> src/login.rs:229:32
///     |
/// 229 |                     err => err.into_view(),
///     |                                ^^^^^^^^^ method cannot be called on `Result<ViewableLoginFlow, String>` due to unsatisfied trait bounds
///     |
///     = note: the following trait bounds were not satisfied:
///             `<&Result<ViewableLoginFlow, std::string::String> as FnOnce<()>>::Output = _`
///             which is required by `&Result<ViewableLoginFlow, std::string::String>: leptos::IntoView`
///    ... more notes here ...
/// ```
///
/// For more information about how to easily implement `Error` see
/// [thiserror](https://docs.rs/thiserror/latest/thiserror/)
#[component]
pub fn ErrorBoundary<FalFn, Fal, Chil>(
    /// The elements that will be rendered, which may include one or more `Result<_>` types.
    children: TypedChildren<Chil>,
    /// A fallback that will be shown if an error occurs.
    fallback: FalFn,
) -> impl IntoView
where
    FalFn: FnMut(ArcRwSignal<Errors>) -> Fal + Clone + Send + 'static,
    Fal: IntoView + 'static,
    Chil: IntoView + 'static,
{
    let sc = Owner::current_shared_context();
    let boundary_id = sc.as_ref().map(|sc| sc.next_id()).unwrap_or_default();
    let initial_errors =
        sc.map(|sc| sc.errors(&boundary_id)).unwrap_or_default();

    let hook = Arc::new(ErrorBoundaryErrorHook::new(
        boundary_id.clone(),
        initial_errors,
    ));
    let errors = hook.errors.clone();
    let errors_empty = ArcMemo::new({
        let errors = errors.clone();
        move |_| errors.with(|map| map.is_empty())
    });
    let hook = hook as Arc<dyn ErrorHook>;

    // provide the error hook and render children
    throw_error::set_error_hook(Arc::clone(&hook));
    let mut children = Some(children.into_inner()());

    move || ErrorBoundaryView {
        boundary_id: boundary_id.clone(),
        errors_empty: errors_empty.get(),
        children: children.take(),
        fallback: Some((fallback.clone())(errors.clone())),
        errors: errors.clone(),
        rndr: PhantomData,
    }
}

#[derive(Debug)]
struct ErrorBoundaryView<Chil, Fal, Rndr> {
    boundary_id: SerializedDataId,
    errors_empty: bool,
    children: Option<Chil>,
    fallback: Fal,
    errors: ArcRwSignal<Errors>,
    rndr: PhantomData<Rndr>,
}

struct ErrorBoundaryViewState<Chil, Fal, Rndr>
where
    Chil: Render<Rndr>,
    Fal: Render<Rndr>,
    Rndr: Renderer,
{
    showing_fallback: bool,
    // both the children and the fallback are always present, and we toggle between the two of them
    // as needed
    children: Chil::State,
    fallback: Fal::State,
    placeholder: Rndr::Placeholder,
}

impl<Chil, Fal, Rndr> Mountable<Rndr>
    for ErrorBoundaryViewState<Chil, Fal, Rndr>
where
    Chil: Render<Rndr>,
    Fal: Render<Rndr>,
    Rndr: Renderer,
{
    fn unmount(&mut self) {
        if self.showing_fallback {
            self.fallback.unmount();
        } else {
            self.children.unmount();
        }
        self.placeholder.unmount();
    }

    fn mount(&mut self, parent: &Rndr::Element, marker: Option<&Rndr::Node>) {
        if self.showing_fallback {
            self.fallback.mount(parent, marker);
        } else {
            self.children.mount(parent, marker);
        }
        self.placeholder.mount(parent, marker);
    }

    fn insert_before_this(
        &self,
        parent: &Rndr::Element,
        child: &mut dyn Mountable<Rndr>,
    ) -> bool {
        if self.showing_fallback {
            self.fallback.insert_before_this(parent, child)
        } else {
            self.children.insert_before_this(parent, child)
        }
    }
}

impl<Chil, Fal, Rndr> Render<Rndr> for ErrorBoundaryView<Chil, Fal, Rndr>
where
    Chil: Render<Rndr>,
    Fal: Render<Rndr>,
    Rndr: Renderer,
{
    type State = ErrorBoundaryViewState<Chil, Fal, Rndr>;

    fn build(self) -> Self::State {
        let placeholder = Rndr::create_placeholder();
        let children = (self.children.expect(
            "tried to build ErrorBoundary but children were not present",
        ))
        .build();
        let fallback = self.fallback.build();
        ErrorBoundaryViewState {
            showing_fallback: !self.errors_empty,
            children,
            fallback,
            placeholder,
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        match (self.errors_empty, state.showing_fallback) {
            // no errors, and was showing fallback
            (true, true) => {
                state.fallback.unmount();
                Rndr::try_mount_before(
                    &mut state.children,
                    state.placeholder.as_ref(),
                );
            }
            // yes errors, and was showing children
            (false, false) => {
                state.children.unmount();
                Rndr::try_mount_before(
                    &mut state.fallback,
                    state.placeholder.as_ref(),
                );
            }
            // either there were no errors, and we were already showing the children
            // or there are errors, but we were already showing the fallback
            // in either case, rebuilding doesn't require us to do anything
            _ => {}
        }
        state.showing_fallback = !self.errors_empty;
    }
}

impl<Chil, Fal, Rndr> RenderHtml<Rndr> for ErrorBoundaryView<Chil, Fal, Rndr>
where
    Chil: RenderHtml<Rndr>,
    Fal: RenderHtml<Rndr> + Send,
    Rndr: Renderer,
{
    type AsyncOutput = ErrorBoundaryView<Chil::AsyncOutput, Fal, Rndr>;

    const MIN_LENGTH: usize = Chil::MIN_LENGTH;

    async fn resolve(self) -> Self::AsyncOutput {
        let ErrorBoundaryView {
            boundary_id,
            errors_empty,
            children,
            fallback,
            errors,
            ..
        } = self;
        let children = match children {
            None => None,
            Some(children) => Some(children.resolve().await),
        };
        ErrorBoundaryView {
            boundary_id,
            errors_empty,
            children,
            fallback,
            errors,
            rndr: PhantomData,
        }
    }

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        // first, attempt to serialize the children to HTML, then check for errors
        let mut new_buf = String::with_capacity(Chil::MIN_LENGTH);
        let mut new_pos = *position;
        self.children.to_html_with_buf(&mut new_buf, &mut new_pos);

        // any thrown errors would've been caught here
        if self.errors.with_untracked(|map| map.is_empty()) {
            buf.push_str(&new_buf);
        } else {
            // otherwise, serialize the fallback instead
            self.fallback.to_html_with_buf(buf, position);
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        // first, attempt to serialize the children to HTML, then check for errors
        let mut new_buf = StreamBuilder::new(buf.clone_id());
        let mut new_pos = *position;
        self.children
            .to_html_async_with_buf::<OUT_OF_ORDER>(&mut new_buf, &mut new_pos);

        if let Some(sc) = Owner::current_shared_context() {
            sc.seal_errors(&self.boundary_id);
        }

        // any thrown errors would've been caught here
        if self.errors.with_untracked(|map| map.is_empty()) {
            buf.append(new_buf);
        } else {
            // otherwise, serialize the fallback instead
            let mut fallback = String::with_capacity(Fal::MIN_LENGTH);
            self.fallback.to_html_with_buf(&mut fallback, position);
            buf.push_sync(&fallback);
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self::State {
        let children = self.children.expect(
            "tried to hydrate ErrorBoundary but children were not present",
        );
        let (children, fallback) = if self.errors_empty {
            (
                children.hydrate::<FROM_SERVER>(cursor, position),
                self.fallback.build(),
            )
        } else {
            (
                children.build(),
                self.fallback.hydrate::<FROM_SERVER>(cursor, position),
            )
        };

        let placeholder = cursor.next_placeholder(position);

        ErrorBoundaryViewState {
            showing_fallback: !self.errors_empty,
            children,
            fallback,
            placeholder,
        }
    }
}

#[derive(Debug)]
struct ErrorBoundaryErrorHook {
    errors: ArcRwSignal<Errors>,
    id: SerializedDataId,
    shared_context: Option<Arc<dyn SharedContext + Send + Sync>>,
}

impl ErrorBoundaryErrorHook {
    pub fn new(
        id: SerializedDataId,
        initial_errors: impl IntoIterator<Item = (ErrorId, Error)>,
    ) -> Self {
        Self {
            errors: ArcRwSignal::new(Errors(
                initial_errors.into_iter().collect(),
            )),
            id,
            shared_context: Owner::current_shared_context(),
        }
    }
}

impl ErrorHook for ErrorBoundaryErrorHook {
    fn throw(&self, error: Error) -> ErrorId {
        // generate a unique ID
        let key = ErrorId::default(); // TODO unique ID...

        // register it with the shared context, so that it can be serialized from server to client
        // as needed
        if let Some(sc) = &self.shared_context {
            sc.register_error(self.id.clone(), key.clone(), error.clone());
        }

        // add it to the reactive map of errors
        self.errors.update(|map| {
            map.insert(key.clone(), error);
        });

        // return the key, which will be owned by the Result being rendered and can be used to
        // unregister this error if it is rebuilt
        key
    }

    fn clear(&self, id: &throw_error::ErrorId) {
        self.errors.update(|map| {
            map.remove(id);
        });
    }
}

/// A struct to hold all the possible errors that could be provided by child Views
#[derive(Debug, Clone, Default)]
#[repr(transparent)]
pub struct Errors(FxHashMap<ErrorId, Error>);

impl Errors {
    /// Returns `true` if there are no errors.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Add an error to Errors that will be processed by `<ErrorBoundary/>`
    pub fn insert<E>(&mut self, key: ErrorId, error: E)
    where
        E: Into<Error>,
    {
        self.0.insert(key, error.into());
    }

    /// Add an error with the default key for errors outside the reactive system
    pub fn insert_with_default_key<E>(&mut self, error: E)
    where
        E: Into<Error>,
    {
        self.0.insert(Default::default(), error.into());
    }

    /// Remove an error to Errors that will be processed by `<ErrorBoundary/>`
    pub fn remove(&mut self, key: &ErrorId) -> Option<Error> {
        self.0.remove(key)
    }

    /// An iterator over all the errors, in arbitrary order.
    #[inline(always)]
    pub fn iter(&self) -> Iter<'_> {
        Iter(self.0.iter())
    }
}

impl IntoIterator for Errors {
    type Item = (ErrorId, Error);
    type IntoIter = IntoIter;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter())
    }
}

/// An owning iterator over all the errors contained in the [`Errors`] struct.
#[repr(transparent)]
pub struct IntoIter(std::collections::hash_map::IntoIter<ErrorId, Error>);

impl Iterator for IntoIter {
    type Item = (ErrorId, Error);

    #[inline(always)]
    fn next(
        &mut self,
    ) -> std::option::Option<<Self as std::iter::Iterator>::Item> {
        self.0.next()
    }
}

/// An iterator over all the errors contained in the [`Errors`] struct.
#[repr(transparent)]
pub struct Iter<'a>(std::collections::hash_map::Iter<'a, ErrorId, Error>);

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a ErrorId, &'a Error);

    #[inline(always)]
    fn next(
        &mut self,
    ) -> std::option::Option<<Self as std::iter::Iterator>::Item> {
        self.0.next()
    }
}
