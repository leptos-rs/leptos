use crate::{
    children::{TypedChildren, ViewFnOnce},
    IntoView,
};
use futures::{select, FutureExt, StreamExt};
use hydration_context::SerializedDataId;
use leptos_macro::component;
use reactive_graph::{
    computed::{
        suspense::{LocalResourceNotifier, SuspenseContext},
        ArcMemo, ScopedFuture,
    },
    effect::RenderEffect,
    owner::{provide_context, use_context, Owner},
    signal::ArcRwSignal,
    traits::{Get, GetUntracked, Read, Track, With},
};
use slotmap::{DefaultKey, SlotMap};
use tachys::{
    either::Either,
    html::attribute::Attribute,
    hydration::Cursor,
    reactive_graph::OwnedView,
    renderer::Renderer,
    ssr::StreamBuilder,
    view::{
        add_attr::AddAnyAttr,
        either::{EitherKeepAlive, EitherKeepAliveState},
        Mountable, Position, PositionState, Render, RenderHtml,
    },
};
use throw_error::ErrorHookFuture;

/// TODO docs!
#[component]
pub fn Suspense<Chil>(
    #[prop(optional, into)] fallback: ViewFnOnce,
    children: TypedChildren<Chil>,
) -> impl IntoView
where
    Chil: IntoView + Send + 'static,
{
    let (starts_local, id) = {
        Owner::current_shared_context()
            .map(|sc| {
                let id = sc.next_id();
                (sc.get_incomplete_chunk(&id), id)
            })
            .unwrap_or_else(|| (false, Default::default()))
    };
    let fallback = fallback.run();
    let children = children.into_inner()();
    let tasks = ArcRwSignal::new(SlotMap::<DefaultKey, ()>::new());
    provide_context(SuspenseContext {
        tasks: tasks.clone(),
    });
    let none_pending = ArcMemo::new(move |prev: Option<&bool>| {
        tasks.track();
        if prev.is_none() && starts_local {
            false
        } else {
            tasks.with(SlotMap::is_empty)
        }
    });

    OwnedView::new(SuspenseBoundary::<false, _, _> {
        id,
        none_pending,
        fallback,
        children,
    })
}

pub(crate) struct SuspenseBoundary<const TRANSITION: bool, Fal, Chil> {
    pub id: SerializedDataId,
    pub none_pending: ArcMemo<bool>,
    pub fallback: Fal,
    pub children: Chil,
}

impl<const TRANSITION: bool, Fal, Chil, Rndr> Render<Rndr>
    for SuspenseBoundary<TRANSITION, Fal, Chil>
where
    Fal: Render<Rndr> + Send + 'static,
    Chil: Render<Rndr> + Send + 'static,
    Rndr: Renderer + 'static,
{
    type State = RenderEffect<EitherKeepAliveState<Chil::State, Fal::State>>;

    fn build(self) -> Self::State {
        let mut children = Some(self.children);
        let mut fallback = Some(self.fallback);
        let none_pending = self.none_pending;
        let mut nth_run = 0;

        RenderEffect::new(
            move |prev: Option<
                EitherKeepAliveState<Chil::State, Fal::State>,
            >| {
                // show the fallback if
                // 1) there are pending futures, and
                // 2) we are either in a Suspense (not Transition), or it's the first fallback
                //    (because we initially render the children to register Futures, the "first
                //    fallback" is probably the 2nd run
                let show_b =
                    !none_pending.get() && (!TRANSITION || nth_run < 2);
                nth_run += 1;
                let this = EitherKeepAlive {
                    a: children.take(),
                    b: fallback.take(),
                    show_b,
                };

                if let Some(mut state) = prev {
                    this.rebuild(&mut state);
                    state
                } else {
                    this.build()
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

impl<const TRANSITION: bool, Fal, Chil, Rndr> AddAnyAttr<Rndr>
    for SuspenseBoundary<TRANSITION, Fal, Chil>
where
    Fal: RenderHtml<Rndr> + Send + 'static,
    Chil: RenderHtml<Rndr> + Send + 'static,
    Rndr: Renderer + 'static,
{
    type Output<SomeNewAttr: Attribute<Rndr>> = SuspenseBoundary<
        TRANSITION,
        Fal,
        Chil::Output<SomeNewAttr::CloneableOwned>,
    >;

    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Rndr>,
    {
        let attr = attr.into_cloneable_owned();
        let SuspenseBoundary {
            id,
            none_pending,
            fallback,
            children,
        } = self;
        SuspenseBoundary {
            id,
            none_pending,
            fallback,
            children: children.add_any_attr(attr),
        }
    }
}

impl<const TRANSITION: bool, Fal, Chil, Rndr> RenderHtml<Rndr>
    for SuspenseBoundary<TRANSITION, Fal, Chil>
where
    Fal: RenderHtml<Rndr> + Send + 'static,
    Chil: RenderHtml<Rndr> + Send + 'static,
    Rndr: Renderer + 'static,
{
    // i.e., if this is the child of another Suspense during SSR, don't wait for it: it will handle
    // itself
    type AsyncOutput = Self;

    const MIN_LENGTH: usize = Chil::MIN_LENGTH;

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
    ) {
        self.fallback.to_html_with_buf(buf, position, escape);
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        mut self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
    ) where
        Self: Sized,
    {
        buf.next_id();
        let suspense_context = use_context::<SuspenseContext>().unwrap();

        let owner = Owner::current().unwrap();

        // we need to wait for one of two things: either
        // 1. all tasks are finished loading, or
        // 2. we read from a local resource, meaning this Suspense can never resolve on the server

        // first, create listener for tasks
        let tasks = suspense_context.tasks.clone();
        let (tasks_tx, mut tasks_rx) =
            futures::channel::oneshot::channel::<()>();

        let mut tasks_tx = Some(tasks_tx);
        let eff = reactive_graph::effect::RenderEffect::new_isomorphic({
            move |_| {
                tasks.track();
                if tasks.read().is_empty() {
                    if let Some(tx) = tasks_tx.take() {
                        // If the receiver has dropped, it means the ScopedFuture has already
                        // dropped, so it doesn't matter if we manage to send this.
                        _ = tx.send(());
                    }
                }
            }
        });

        // now, create listener for local resources
        let (local_tx, mut local_rx) =
            futures::channel::oneshot::channel::<()>();
        provide_context(LocalResourceNotifier::from(local_tx));

        // walk over the tree of children once to make sure that all resource loads are registered
        self.children.dry_resolve();

        let mut fut = Box::pin(ScopedFuture::new(ErrorHookFuture::new(
            async move {
                // race the local resource notifier against the set of tasks
                //
                // if there are local resources, we just return the fallback immediately
                //
                // otherwise, we want to wait for resources to load before trying to resolve the body
                //
                // this is *less efficient* than just resolving the body
                // however, it means that you can use reactive accesses to resources/async derived
                // inside component props, at any level, and have those picked up by Suspense, and
                // that it will wait for those to resolve
                select! {
                    // if there are local resources, bail
                    // this will only have fired by this point for local resources accessed
                    // *synchronously*
                    _ = local_rx => {
                        let sc = Owner::current_shared_context().expect("no shared context");
                        sc.set_incomplete_chunk(self.id);
                        None
                    }
                    _ = tasks_rx => {
                        // if we ran this earlier, reactive reads would always be registered as None
                        // this is fine in the case where we want to use Suspend and .await on some future
                        // but in situations like a <For each=|| some_resource.snapshot()/> we actually
                        // want to be able to 1) synchronously read a resource's value, but still 2) wait
                        // for it to load before we render everything
                        let mut children = Box::pin(self.children.resolve().fuse());

                        // we continue racing the children against the "do we have any local
                        // resources?" Future
                        select! {
                            _ = local_rx => {
                                let sc = Owner::current_shared_context().expect("no shared context");
                                sc.set_incomplete_chunk(self.id);
                                None
                            }
                            children = children => {
                                // clean up the (now useless) effect
                                drop(eff);

                                Some(OwnedView::new_with_owner(children, owner))
                            }
                        }
                    }
                }
            },
        )));
        match fut.as_mut().now_or_never() {
            Some(Some(resolved)) => {
                Either::<Fal, _>::Right(resolved)
                    .to_html_async_with_buf::<OUT_OF_ORDER>(
                        buf, position, escape,
                    );
            }
            Some(None) => {
                Either::<_, Chil>::Left(self.fallback)
                    .to_html_async_with_buf::<OUT_OF_ORDER>(
                        buf, position, escape,
                    );
            }
            None => {
                let id = buf.clone_id();

                // out-of-order streams immediately push fallback,
                // wrapped by suspense markers
                if OUT_OF_ORDER {
                    let mut fallback_position = *position;
                    buf.push_fallback(self.fallback, &mut fallback_position);
                    buf.push_async_out_of_order(fut, position);
                } else {
                    buf.push_async({
                        let mut position = *position;
                        async move {
                            let value = match fut.await {
                                None => Either::Left(self.fallback),
                                Some(value) => Either::Right(value),
                            };
                            let mut builder = StreamBuilder::new(id);
                            value.to_html_async_with_buf::<OUT_OF_ORDER>(
                                &mut builder,
                                &mut position,
                                escape,
                            );
                            builder.finish().take_chunks()
                        }
                    });
                    *position = Position::NextChild;
                }
            }
        };
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self::State {
        let cursor = cursor.to_owned();
        let position = position.to_owned();

        let mut children = Some(self.children);
        let mut fallback = Some(self.fallback);
        let none_pending = self.none_pending;
        let mut nth_run = 0;

        RenderEffect::new(
            move |prev: Option<
                EitherKeepAliveState<Chil::State, Fal::State>,
            >| {
                // show the fallback if
                // 1) there are pending futures, and
                // 2) we are either in a Suspense (not Transition), or it's the first fallback
                //    (because we initially render the children to register Futures, the "first
                //    fallback" is probably the 2nd run
                let show_b =
                    !none_pending.get() && (!TRANSITION || nth_run < 1);
                nth_run += 1;
                let this = EitherKeepAlive {
                    a: children.take(),
                    b: fallback.take(),
                    show_b,
                };

                if let Some(mut state) = prev {
                    this.rebuild(&mut state);
                    state
                } else {
                    this.hydrate::<FROM_SERVER>(&cursor, &position)
                }
            },
        )
    }
}
