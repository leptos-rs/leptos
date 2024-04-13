use crate::{children::TypedChildren, IntoView};
use any_error::{Error, ErrorHook, ErrorId};
use leptos_macro::component;
use leptos_reactive::Effect;
use reactive_graph::{
    computed::ArcMemo,
    signal::ArcRwSignal,
    traits::{Get, Update, With},
};
use rustc_hash::FxHashMap;
use std::{marker::PhantomData, sync::Arc};
use tachys::{
    hydration::Cursor,
    renderer::Renderer,
    view::{Mountable, Position, PositionState, Render, RenderHtml},
};

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
    mut fallback: FalFn,
) -> impl IntoView
where
    FalFn: FnMut(&ArcRwSignal<Errors>) -> Fal + Clone + Send + 'static,
    Fal: IntoView + 'static,
    Fal::AsyncOutput: Send,
    Chil: IntoView + 'static,
    Chil::AsyncOutput: Send,
{
    let hook = Arc::new(ErrorBoundaryErrorHook::default());
    let errors = hook.errors.clone();
    let errors_empty = ArcMemo::new({
        let errors = errors.clone();
        move |_| errors.with(|map| map.is_empty())
    });
    let hook = hook as Arc<dyn ErrorHook>;

    // provide the error hook and render children
    any_error::set_error_hook(Arc::clone(&hook));
    let mut children = Some(children.into_inner()());

    move || ErrorBoundaryView {
        errors_empty: errors_empty.get(),
        children: children.take(),
        fallback: Some((fallback.clone())(&errors)),
        rndr: PhantomData,
    }
}

#[derive(Debug)]
struct ErrorBoundaryView<Chil, Fal, Rndr> {
    errors_empty: bool,
    children: Option<Chil>,
    fallback: Fal,
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
    type FallibleState = ();

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

    fn try_build(self) -> any_error::Result<Self::FallibleState> {
        todo!()
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> any_error::Result<()> {
        todo!()
    }
}

impl<Chil, Fal, Rndr> RenderHtml<Rndr> for ErrorBoundaryView<Chil, Fal, Rndr>
where
    Chil: RenderHtml<Rndr>,
    Fal: RenderHtml<Rndr>,
    Rndr: Renderer,
{
    type AsyncOutput = std::future::Ready<()>; //ErrorBoundaryView<Chil::AsyncOutput, Fal, Rndr>;

    const MIN_LENGTH: usize = Chil::MIN_LENGTH;

    fn resolve(self) -> Self::AsyncOutput {
        todo!()
    }

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        todo!()
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self::State {
        todo!()
    }
}

#[derive(Debug, Default)]
struct ErrorBoundaryErrorHook {
    errors: ArcRwSignal<Errors>,
}

impl ErrorHook for ErrorBoundaryErrorHook {
    fn throw(&self, error: Error) -> ErrorId {
        let key = ErrorId::default();
        self.errors.update(|map| {
            map.insert(key.clone(), error);
        });
        key
    }

    fn clear(&self, id: &any_error::ErrorId) {
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
