use crate::{
    html::attribute::Attribute,
    hydration::Cursor,
    renderer::Renderer,
    ssr::StreamBuilder,
    view::{
        add_attr::AddAnyAttr, iterators::OptionState, Mountable, Position,
        PositionState, Render, RenderHtml,
    },
};
use any_spawner::Executor;
use futures::{select, FutureExt};
use reactive_graph::{
    computed::{
        suspense::{LocalResourceNotifier, SuspenseContext},
        ScopedFuture,
    },
    owner::{provide_context, use_context},
};
use std::{cell::RefCell, fmt::Debug, future::Future, pin::Pin, rc::Rc};

/// A suspended `Future`, which can be used in the view.
#[derive(Clone)]
pub struct Suspend<Fut>(pub ScopedFuture<Fut>); // TODO probably shouldn't be pub

impl<Fut> Suspend<Fut> {
    /// Creates a new suspended view.
    pub fn new(fut: Fut) -> Self {
        Self(ScopedFuture::new(fut))
    }
}

impl<Fut> Debug for Suspend<Fut> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Suspend").finish()
    }
}

/// Retained view state for [`Suspend`].
pub struct SuspendState<T, Rndr>
where
    T: Render<Rndr>,
    Rndr: Renderer,
{
    inner: Rc<RefCell<OptionState<T, Rndr>>>,
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

    fn insert_before_this(&self, child: &mut dyn Mountable<Rndr>) -> bool {
        self.inner.borrow_mut().insert_before_this(child)
    }
}

impl<Fut, Rndr> Render<Rndr> for Suspend<Fut>
where
    Fut: Future + 'static,
    Fut::Output: Render<Rndr>,
    Rndr: Renderer + 'static,
{
    type State = SuspendState<Fut::Output, Rndr>;

    // TODO cancelation if it fires multiple times
    fn build(self) -> Self::State {
        // poll the future once immediately
        // if it's already available, start in the ready state
        // otherwise, start with the fallback
        let mut fut = Box::pin(self.0);
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
        let fut = self.0;
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
}

impl<Fut, Rndr> AddAnyAttr<Rndr> for Suspend<Fut>
where
    Fut: Future + Send + 'static,
    Fut::Output: AddAnyAttr<Rndr>,
    Rndr: Renderer + 'static,
{
    type Output<SomeNewAttr: Attribute<Rndr>> = Suspend<
        Pin<
            Box<
                dyn Future<
                        Output = <Fut::Output as AddAnyAttr<Rndr>>::Output<
                            SomeNewAttr::CloneableOwned,
                        >,
                    > + Send,
            >,
        >,
    >;

    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Rndr>,
    {
        let attr = attr.into_cloneable_owned();
        let ScopedFuture {
            owner,
            observer,
            fut,
        } = self.0;
        Suspend(ScopedFuture {
            owner,
            observer,
            fut: Box::pin(async move {
                let this = fut.await;
                this.add_any_attr(attr)
            }),
        })
    }
}

impl<Fut, Rndr> RenderHtml<Rndr> for Suspend<Fut>
where
    Fut: Future + Send + 'static,
    Fut::Output: RenderHtml<Rndr>,
    Rndr: Renderer + 'static,
{
    type AsyncOutput = Option<Fut::Output>;

    const MIN_LENGTH: usize = Fut::Output::MIN_LENGTH;

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        // TODO wrap this with a Suspense as needed
        // currently this is just used for Routes, which creates a Suspend but never actually needs
        // it (because we don't lazy-load routes on the server)
        if let Some(inner) = self.0.now_or_never() {
            inner.to_html_with_buf(buf, position, escape, mark_branches);
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) where
        Self: Sized,
    {
        let mut fut = Box::pin(self.0);
        match fut.as_mut().now_or_never() {
            Some(inner) => inner.to_html_async_with_buf::<OUT_OF_ORDER>(
                buf,
                position,
                escape,
                mark_branches,
            ),
            None => {
                if use_context::<SuspenseContext>().is_none() {
                    buf.next_id();
                    let (local_tx, mut local_rx) =
                        futures::channel::oneshot::channel::<()>();
                    provide_context(LocalResourceNotifier::from(local_tx));
                    let mut fut = fut.fuse();
                    let fut = async move {
                        select! {
                            _  = local_rx => None,
                            value = fut => Some(value)
                        }
                    };
                    let id = buf.clone_id();

                    // out-of-order streams immediately push fallback,
                    // wrapped by suspense markers
                    if OUT_OF_ORDER {
                        let mut fallback_position = *position;
                        buf.push_fallback::<(), Rndr>(
                            (),
                            &mut fallback_position,
                            mark_branches,
                        );
                        buf.push_async_out_of_order(
                            fut,
                            position,
                            mark_branches,
                        );
                    } else {
                        buf.push_async({
                            let mut position = *position;
                            async move {
                                let value = fut.await;
                                let mut builder = StreamBuilder::new(id);
                                value.to_html_async_with_buf::<OUT_OF_ORDER>(
                                    &mut builder,
                                    &mut position,
                                    escape,
                                    mark_branches,
                                );
                                builder.finish().take_chunks()
                            }
                        });
                        *position = Position::NextChild;
                    }
                }
            }
        }
    }

    // TODO cancellation
    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self::State {
        // poll the future once immediately
        // if it's already available, start in the ready state
        // otherwise, start with the fallback
        let mut fut = Box::pin(self.0);
        let initial = fut.as_mut().now_or_never();
        let initially_pending = initial.is_none();
        let inner = Rc::new(RefCell::new(
            initial.hydrate::<FROM_SERVER>(cursor, position),
        ));

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

    async fn resolve(self) -> Self::AsyncOutput {
        Some(self.0.await)
    }

    fn dry_resolve(&mut self) {}
}
