use crate::{children::TypedChildren, IntoView};
use hydration_context::{SerializedDataId, SharedContext};
use leptos_macro::component;
use reactive_graph::{
    computed::ArcMemo,
    effect::RenderEffect,
    owner::Owner,
    signal::ArcRwSignal,
    traits::{Get, Update, With, WithUntracked},
};
use rustc_hash::FxHashMap;
use std::{marker::PhantomData, sync::Arc};
use tachys::{
    html::attribute::Attribute,
    hydration::Cursor,
    renderer::Renderer,
    ssr::StreamBuilder,
    view::{
        add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
        RenderHtml,
    },
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
    FalFn: FnMut(ArcRwSignal<Errors>) -> Fal + Send + 'static,
    Fal: IntoView + Send + 'static,
    Chil: IntoView + Send + 'static,
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
    // TODO unset this outside the ErrorBoundary
    throw_error::set_error_hook(Arc::clone(&hook));
    let children = children.into_inner()();

    ErrorBoundaryView {
        boundary_id,
        errors_empty,
        children,
        errors,
        fallback,
        rndr: PhantomData,
    }
}

#[derive(Debug)]
struct ErrorBoundaryView<Chil, FalFn, Rndr> {
    boundary_id: SerializedDataId,
    errors_empty: ArcMemo<bool>,
    children: Chil,
    fallback: FalFn,
    errors: ArcRwSignal<Errors>,
    rndr: PhantomData<Rndr>,
}

struct ErrorBoundaryViewState<Chil, Fal> {
    // the children are always present; we toggle between them and the fallback as needed
    children: Chil,
    fallback: Option<Fal>,
}

impl<Chil, Fal, Rndr> Mountable<Rndr> for ErrorBoundaryViewState<Chil, Fal>
where
    Chil: Mountable<Rndr>,
    Fal: Mountable<Rndr>,
    Rndr: Renderer,
{
    fn unmount(&mut self) {
        if let Some(fallback) = &mut self.fallback {
            fallback.unmount();
        } else {
            self.children.unmount();
        }
    }

    fn mount(&mut self, parent: &Rndr::Element, marker: Option<&Rndr::Node>) {
        if let Some(fallback) = &mut self.fallback {
            fallback.mount(parent, marker);
        } else {
            self.children.mount(parent, marker);
        }
    }

    fn insert_before_this(&self, child: &mut dyn Mountable<Rndr>) -> bool {
        if let Some(fallback) = &self.fallback {
            fallback.insert_before_this(child)
        } else {
            self.children.insert_before_this(child)
        }
    }
}

impl<Chil, FalFn, Fal, Rndr> Render<Rndr>
    for ErrorBoundaryView<Chil, FalFn, Rndr>
where
    Chil: Render<Rndr> + 'static,
    FalFn: FnMut(ArcRwSignal<Errors>) -> Fal + Send + 'static,
    Fal: Render<Rndr> + 'static,
    Rndr: Renderer,
{
    type State = RenderEffect<ErrorBoundaryViewState<Chil::State, Fal::State>>;

    fn build(mut self) -> Self::State {
        let mut children = Some(self.children.build());
        RenderEffect::new(
            move |prev: Option<
                ErrorBoundaryViewState<Chil::State, Fal::State>,
            >| {
                if let Some(mut state) = prev {
                    match (self.errors_empty.get(), &mut state.fallback) {
                        // no errors, and was showing fallback
                        (true, Some(fallback)) => {
                            fallback.insert_before_this(&mut state.children);
                            fallback.unmount();
                            state.fallback = None;
                        }
                        // yes errors, and was showing children
                        (false, None) => {
                            state.fallback = Some(
                                (self.fallback)(self.errors.clone()).build(),
                            );
                            state
                                .children
                                .insert_before_this(&mut state.fallback);
                            state.children.unmount();
                        }
                        // either there were no errors, and we were already showing the children
                        // or there are errors, but we were already showing the fallback
                        // in either case, rebuilding doesn't require us to do anything
                        _ => {}
                    }
                    state
                } else {
                    let fallback = (!self.errors_empty.get())
                        .then(|| (self.fallback)(self.errors.clone()).build());
                    ErrorBoundaryViewState {
                        children: children.take().unwrap(),
                        fallback,
                    }
                }
            },
        )
    }

    fn rebuild(self, state: &mut Self::State) {
        let new = self.build();
        let mut old = std::mem::replace(state, new);
        old.insert_before_this(state);
        old.unmount();
    }
}

impl<Chil, FalFn, Fal, Rndr> AddAnyAttr<Rndr>
    for ErrorBoundaryView<Chil, FalFn, Rndr>
where
    Chil: RenderHtml<Rndr> + 'static,
    FalFn: FnMut(ArcRwSignal<Errors>) -> Fal + Send + 'static,
    Fal: RenderHtml<Rndr> + Send + 'static,
    Rndr: Renderer,
{
    type Output<SomeNewAttr: Attribute<Rndr>> = ErrorBoundaryView<
        Chil::Output<SomeNewAttr::CloneableOwned>,
        FalFn,
        Rndr,
    >;

    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Rndr>,
    {
        let ErrorBoundaryView {
            boundary_id,
            errors_empty,
            children,
            fallback,
            errors,
            rndr,
        } = self;
        ErrorBoundaryView {
            boundary_id,
            errors_empty,
            children: children.add_any_attr(attr.into_cloneable_owned()),
            fallback,
            errors,
            rndr,
        }
    }
}

impl<Chil, FalFn, Fal, Rndr> RenderHtml<Rndr>
    for ErrorBoundaryView<Chil, FalFn, Rndr>
where
    Chil: RenderHtml<Rndr> + Send + 'static,
    FalFn: FnMut(ArcRwSignal<Errors>) -> Fal + Send + 'static,
    Fal: RenderHtml<Rndr> + Send + 'static,
    Rndr: Renderer,
{
    type AsyncOutput = ErrorBoundaryView<Chil::AsyncOutput, FalFn, Rndr>;

    const MIN_LENGTH: usize = Chil::MIN_LENGTH;

    fn dry_resolve(&mut self) {
        self.children.dry_resolve();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        let ErrorBoundaryView {
            boundary_id,
            errors_empty,
            children,
            fallback,
            errors,
            ..
        } = self;
        ErrorBoundaryView {
            boundary_id,
            errors_empty,
            children: children.resolve().await,
            fallback,
            errors,
            rndr: PhantomData,
        }
    }

    fn to_html_with_buf(
        mut self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
    ) {
        // first, attempt to serialize the children to HTML, then check for errors
        let mut new_buf = String::with_capacity(Chil::MIN_LENGTH);
        let mut new_pos = *position;
        self.children
            .to_html_with_buf(&mut new_buf, &mut new_pos, escape);

        // any thrown errors would've been caught here
        if self.errors.with_untracked(|map| map.is_empty()) {
            buf.push_str(&new_buf);
        } else {
            // otherwise, serialize the fallback instead
            (self.fallback)(self.errors)
                .to_html_with_buf(buf, position, escape);
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        mut self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
    ) where
        Self: Sized,
    {
        // first, attempt to serialize the children to HTML, then check for errors
        let mut new_buf = StreamBuilder::new(buf.clone_id());
        let mut new_pos = *position;
        self.children.to_html_async_with_buf::<OUT_OF_ORDER>(
            &mut new_buf,
            &mut new_pos,
            escape,
        );

        if let Some(sc) = Owner::current_shared_context() {
            sc.seal_errors(&self.boundary_id);
        }

        // any thrown errors would've been caught here
        if self.errors.with_untracked(|map| map.is_empty()) {
            buf.append(new_buf);
        } else {
            // otherwise, serialize the fallback instead
            let mut fallback = String::with_capacity(Fal::MIN_LENGTH);
            (self.fallback)(self.errors).to_html_with_buf(
                &mut fallback,
                position,
                escape,
            );
            buf.push_sync(&fallback);
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        mut self,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self::State {
        let mut children = Some(self.children);
        let cursor = cursor.to_owned();
        let position = position.to_owned();
        RenderEffect::new(
            move |prev: Option<
                ErrorBoundaryViewState<Chil::State, Fal::State>,
            >| {
                if let Some(mut state) = prev {
                    match (self.errors_empty.get(), &mut state.fallback) {
                        // no errors, and was showing fallback
                        (true, Some(fallback)) => {
                            fallback.insert_before_this(&mut state.children);
                            state.fallback.unmount();
                            state.fallback = None;
                        }
                        // yes errors, and was showing children
                        (false, None) => {
                            state.fallback = Some(
                                (self.fallback)(self.errors.clone()).build(),
                            );
                            state
                                .children
                                .insert_before_this(&mut state.fallback);
                            state.children.unmount();
                        }
                        // either there were no errors, and we were already showing the children
                        // or there are errors, but we were already showing the fallback
                        // in either case, rebuilding doesn't require us to do anything
                        _ => {}
                    }
                    state
                } else {
                    let children = children.take().unwrap();
                    let (children, fallback) = if self.errors_empty.get() {
                        (
                            children.hydrate::<FROM_SERVER>(&cursor, &position),
                            None,
                        )
                    } else {
                        (
                            children.build(),
                            Some(
                                (self.fallback)(self.errors.clone())
                                    .hydrate::<FROM_SERVER>(&cursor, &position),
                            ),
                        )
                    };

                    ErrorBoundaryViewState { children, fallback }
                }
            },
        )
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
