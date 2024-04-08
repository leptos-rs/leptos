use crate::{
    children::{TypedChildren, ViewFnOnce},
    IntoView,
};
use any_spawner::Executor;
use futures::FutureExt;
use leptos_macro::component;
use reactive_graph::{
    computed::{ArcMemo, ScopedFuture},
    owner::{provide_context, use_context},
    signal::ArcRwSignal,
    traits::{Get, Update, With, Writeable},
};
use slotmap::{DefaultKey, SlotMap};
use std::{cell::RefCell, fmt::Debug, future::Future, rc::Rc};
use tachys::{
    hydration::Cursor,
    reactive_graph::RenderEffectState,
    renderer::Renderer,
    ssr::StreamBuilder,
    view::{
        either::{EitherKeepAlive, EitherKeepAliveState},
        iterators::OptionState,
        Mountable, Position, PositionState, Render, RenderHtml,
    },
};

/// TODO docs!
#[component]
pub fn Suspense<Chil>(
    #[prop(optional, into)] fallback: ViewFnOnce,
    children: TypedChildren<Chil>,
) -> impl IntoView
where
    Chil: IntoView + 'static,
{
    let fallback = fallback.run();
    let children = children.into_inner()();
    let tasks = ArcRwSignal::new(SlotMap::<DefaultKey, ()>::new());
    provide_context(SuspenseContext {
        tasks: tasks.clone(),
    });
    let none_pending = ArcMemo::new(move |_| tasks.with(SlotMap::is_empty));
    SuspenseBoundary::<false, _, _> {
        none_pending,
        fallback,
        children,
    }
}

pub(crate) struct SuspenseBoundary<const TRANSITION: bool, Fal, Chil> {
    pub none_pending: ArcMemo<bool>,
    pub fallback: Fal,
    pub children: Chil,
}

impl<const TRANSITION: bool, Fal, Chil, Rndr> Render<Rndr>
    for SuspenseBoundary<TRANSITION, Fal, Chil>
where
    Fal: Render<Rndr> + 'static,
    Chil: Render<Rndr> + 'static,
    Rndr: Renderer + 'static,
{
    type State =
        RenderEffectState<EitherKeepAliveState<Chil::State, Fal::State, Rndr>>;
    type FallibleState = ();
    type AsyncOutput = ();

    fn build(self) -> Self::State {
        let mut children = Some(self.children);
        let mut fallback = Some(self.fallback);
        let none_pending = self.none_pending;
        let mut nth_run = 0;

        (move || {
            // show the fallback if
            // 1) there are pending futures, and
            // 2) we are either in a Suspense (not Transition), or it's the first fallback
            //    (because we initially render the children to register Futures, the "first
            //    fallback" is probably the 2nd run
            let show_b = !none_pending.get() && (!TRANSITION || nth_run < 2);
            nth_run += 1;
            EitherKeepAlive {
                a: children.take(),
                b: fallback.take(),
                show_b,
            }
        })
        .build()
    }

    fn rebuild(self, _state: &mut Self::State) {}

    fn try_build(self) -> any_error::Result<Self::FallibleState> {
        todo!()
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> any_error::Result<()> {
        todo!()
    }

    async fn resolve(self) -> Self::AsyncOutput {}
}

impl<const TRANSITION: bool, Fal, Chil, Rndr> RenderHtml<Rndr>
    for SuspenseBoundary<TRANSITION, Fal, Chil>
where
    Fal: RenderHtml<Rndr> + 'static,
    Chil: RenderHtml<Rndr> + 'static,
    Rndr: Renderer + 'static,
{
    const MIN_LENGTH: usize = Chil::MIN_LENGTH;

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

#[derive(Clone, Debug)]
pub(crate) struct SuspenseContext {
    pub tasks: ArcRwSignal<SlotMap<DefaultKey, ()>>,
}

impl SuspenseContext {
    pub fn task_id(&self) -> TaskHandle {
        let key = self.tasks.write().insert(());
        TaskHandle {
            tasks: self.tasks.clone(),
            key,
        }
    }
}

/// A unique identifier that removes itself from the set of tasks when it is dropped.
#[derive(Debug)]
pub(crate) struct TaskHandle {
    tasks: ArcRwSignal<SlotMap<DefaultKey, ()>>,
    key: DefaultKey,
}

impl Drop for TaskHandle {
    fn drop(&mut self) {
        self.tasks.update(|tasks| {
            tasks.remove(self.key);
        });
    }
}

pub trait FutureViewExt: Sized {
    fn wait(self) -> Suspend<Self>
    where
        Self: Future,
    {
        Suspend { fut: self }
    }
}

impl<F> FutureViewExt for F where F: Future + Sized {}

pub struct Suspend<Fut> {
    pub fut: Fut,
}

impl<Fut> Debug for Suspend<Fut> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Suspend").finish()
    }
}

pub struct SuspendState<T, Rndr>
where
    T: Render<Rndr>,
    Rndr: Renderer,
{
    inner: Rc<RefCell<OptionState<T::State, Rndr>>>,
}

impl<T, Rndr> Mountable<Rndr> for SuspendState<T, Rndr>
where
    T: Render<Rndr>,
    Rndr: Renderer,
{
    fn unmount(&mut self) {
        self.inner.borrow_mut().unmount();
    }

    fn mount(&mut self, parent: &Rndr::Element, marker: Option<&Rndr::Node>) {
        self.inner.borrow_mut().mount(parent, marker);
    }

    fn insert_before_this(
        &self,
        parent: &Rndr::Element,
        child: &mut dyn Mountable<Rndr>,
    ) -> bool {
        self.inner.borrow_mut().insert_before_this(parent, child)
    }
}

impl<Fut, Rndr> Render<Rndr> for Suspend<Fut>
where
    Fut: Future + 'static,
    Fut::Output: Render<Rndr>,
    Rndr: Renderer + 'static,
{
    type State = SuspendState<Fut::Output, Rndr>;
    type FallibleState = Self::State;
    type AsyncOutput = ();

    // TODO cancelation if it fires multiple times
    fn build(self) -> Self::State {
        // poll the future once immediately
        // if it's already available, start in the ready state
        // otherwise, start with the fallback
        let mut fut = Box::pin(ScopedFuture::new(self.fut));
        let initial = fut.as_mut().now_or_never();
        let initially_pending = initial.is_none();
        let inner = Rc::new(RefCell::new(initial.build()));

        // get a unique ID if there's a SuspenseContext
        let id = use_context::<SuspenseContext>().map(|sc| sc.task_id());

        // if the initial state was pending, spawn a future to wait for it
        // spawning immediately means that our now_or_never poll result isn't lost
        // if it wasn't pending at first, we don't need to poll the Future again
        if initially_pending {
            Executor::spawn_local({
                let state = Rc::clone(&inner);
                async move {
                    let value = fut.as_mut().await;
                    drop(id);
                    Some(value).rebuild(&mut *state.borrow_mut());
                }
            });
        }

        SuspendState { inner }
    }

    fn rebuild(self, state: &mut Self::State) {
        // get a unique ID if there's a SuspenseContext
        let fut = ScopedFuture::new(self.fut);
        let id = use_context::<SuspenseContext>().map(|sc| sc.task_id());

        // spawn the future, and rebuild the state when it resolves
        Executor::spawn_local({
            let state = Rc::clone(&state.inner);
            async move {
                let value = fut.await;
                drop(id);
                Some(value).rebuild(&mut *state.borrow_mut());
            }
        });
    }

    fn try_build(self) -> any_error::Result<Self::FallibleState> {
        todo!()
    }

    fn try_rebuild(
        self,
        _state: &mut Self::FallibleState,
    ) -> any_error::Result<()> {
        todo!()
    }

    async fn resolve(self) -> Self::AsyncOutput {}
}

impl<Fut, Rndr> RenderHtml<Rndr> for Suspend<Fut>
where
    Fut: Future + Send + 'static,
    Fut::Output: RenderHtml<Rndr>,
    Rndr: Renderer + 'static,
{
    const MIN_LENGTH: usize = Fut::Output::MIN_LENGTH;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        todo!()
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        todo!();
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self::State {
        todo!()
    }
}
