use crate::{children::TypedChildrenMut, IntoView};
use any_error::{Error, ErrorHook, ErrorId};
use leptos_macro::component;
use reactive_graph::{
    computed::ArcMemo,
    effect::RenderEffect,
    signal::ArcRwSignal,
    traits::{Get, GetUntracked, Track, Update, With},
};
use rustc_hash::FxHashMap;
use std::{marker::PhantomData, sync::Arc};
use tachys::{
    either::Either,
    reactive_graph::RenderEffectState,
    renderer::Renderer,
    view::{Mountable, Render, RenderHtml},
};

#[component]
pub fn ErrorBoundary<FalFn, Fal, Chil>(
    /// The elements that will be rendered, which may include one or more `Result<_>` types.
    children: TypedChildrenMut<Chil>,
    /// A fallback that will be shown if an error occurs.
    fallback: FalFn,
) -> impl IntoView
where
    FalFn: FnMut(&ArcRwSignal<Errors>) -> Fal + Clone + Send + 'static,
    Fal: IntoView + 'static,
    Chil: IntoView + 'static,
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

    ErrorBoundaryView {
        errors,
        errors_empty,
        children,
        fallback,
        fal_ty: PhantomData,
        rndr: PhantomData,
    }
}

#[derive(Debug)]
struct ErrorBoundaryView<Chil, FalFn, Fal, Rndr> {
    errors: ArcRwSignal<Errors>,
    errors_empty: ArcMemo<bool>,
    children: TypedChildrenMut<Chil>,
    fallback: FalFn,
    fal_ty: PhantomData<Fal>,
    rndr: PhantomData<Rndr>,
}

impl<Chil, FalFn, Fal, Rndr> Render<Rndr>
    for ErrorBoundaryView<Chil, FalFn, Fal, Rndr>
where
    Chil: Render<Rndr> + 'static,
    Chil::State: 'static,
    Fal: Render<Rndr> + 'static,
    Fal::State: 'static,
    FalFn: FnMut(&ArcRwSignal<Errors>) -> Fal + Send + 'static,
    Rndr: Renderer,
{
    type State = ErrorBoundaryViewState<Chil, Fal, Rndr>;
    type FallibleState = ();
    type AsyncOutput = ErrorBoundaryView<Chil, FalFn, Fal, Rndr>;

    fn build(self) -> Self::State {
        let Self {
            errors,
            errors_empty,
            children,
            mut fallback,
            fal_ty,
            rndr,
        } = self;
        let placeholder = Rndr::create_placeholder();
        let mut children = Some(children);
        let effect = RenderEffect::new({
            let placeholder = placeholder.clone();
            move |prev: Option<
                Either<Chil::State, (Fal::State, Chil::State)>,
            >| {
                errors_empty.track();
                if let Some(prev) = prev {
                    match (errors_empty.get_untracked(), prev) {
                        // no errors, and already showing children
                        (true, Either::Left(children)) => {
                            Either::Left(children)
                        }
                        // no errors, and was showing fallback
                        (true, Either::Right((mut fallback, mut children))) => {
                            fallback.unmount();
                            Rndr::mount_before(
                                &mut children,
                                placeholder.as_ref(),
                            );
                            Either::Left(children)
                        }
                        // yes errors, and was showing children
                        (false, Either::Left(mut chil)) => {
                            chil.unmount();
                            let mut fal = fallback(&errors).build();
                            Rndr::mount_before(&mut fal, placeholder.as_ref());
                            Either::Right((fal, chil))
                        }
                        // yes errors, and was showing fallback
                        (false, Either::Right(_)) => todo!(),
                    }
                } else {
                    let children = children.take().unwrap();
                    let mut children = children.into_inner();
                    let children = children().into_inner().build();
                    if errors_empty.get_untracked() {
                        Either::Left(children)
                    } else {
                        Either::Right((fallback(&errors).build(), children))
                    }
                }
            }
        });
        ErrorBoundaryViewState {
            effect,
            placeholder,
            chil_ty: PhantomData,
            fal_ty,
            rndr,
        }
    }

    fn rebuild(self, state: &mut Self::State) {}

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
        self
    }
}

impl<Chil, FalFn, Fal, Rndr> RenderHtml<Rndr>
    for ErrorBoundaryView<Chil, FalFn, Fal, Rndr>
where
    Chil: Render<Rndr> + 'static,
    Chil::State: 'static,
    Fal: Render<Rndr> + 'static,
    Fal::State: 'static,
    FalFn: FnMut(&ArcRwSignal<Errors>) -> Fal + Send + 'static,
    Rndr: Renderer,
{
    const MIN_LENGTH: usize = 0; //Chil::MIN_LENGTH;

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut tachys::view::Position,
    ) {
        todo!()
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &tachys::hydration::Cursor<Rndr>,
        position: &tachys::view::PositionState,
    ) -> Self::State {
        todo!()
    }
}

struct ErrorBoundaryViewState<Chil, Fal, Rndr>
where
    Chil: Render<Rndr>,
    Chil::State: 'static,
    Fal: Render<Rndr>,
    Fal::State: 'static,
    Rndr: Renderer,
{
    effect: RenderEffect<Either<Chil::State, (Fal::State, Chil::State)>>,
    placeholder: Rndr::Placeholder,
    chil_ty: PhantomData<Chil>,
    fal_ty: PhantomData<Fal>,
    rndr: PhantomData<Rndr>,
}

impl<Chil, Fal, Rndr> Mountable<Rndr>
    for ErrorBoundaryViewState<Chil, Fal, Rndr>
where
    Chil: Render<Rndr>,
    Fal: Render<Rndr>,
    Rndr: Renderer,
{
    fn unmount(&mut self) {
        self.effect.with_value_mut(|state| match state {
            Either::Left(chil) => chil.unmount(),
            Either::Right((fal, _)) => fal.unmount(),
        });
        //self.placeholder.unmount();
    }

    fn mount(
        &mut self,
        parent: &<Rndr as Renderer>::Element,
        marker: Option<&<Rndr as Renderer>::Node>,
    ) {
        self.placeholder.mount(parent, marker);
        self.effect.with_value_mut(|state| match state {
            Either::Left(chil) => {
                chil.mount(parent, Some(self.placeholder.as_ref()))
            }
            Either::Right((fal, _)) => {
                fal.mount(parent, Some(self.placeholder.as_ref()))
            }
        });
    }

    fn insert_before_this(
        &self,
        parent: &<Rndr as Renderer>::Element,
        child: &mut dyn Mountable<Rndr>,
    ) -> bool {
        self.effect
            .with_value_mut(|state| match state {
                Either::Left(chil) => chil.insert_before_this(parent, child),
                Either::Right((fal, _)) => {
                    fal.insert_before_this(parent, child)
                }
            })
            .unwrap_or(false)
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
