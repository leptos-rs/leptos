use crate::{
    children::{TypedChildren, ViewFnOnce},
    IntoView,
};
use futures::FutureExt;
use leptos_macro::component;
use reactive_graph::{
    computed::{suspense::SuspenseContext, ArcMemo, ScopedFuture},
    owner::{provide_context, use_context},
    signal::ArcRwSignal,
    traits::{Get, Read, Track, With},
};
use slotmap::{DefaultKey, SlotMap};
use tachys::{
    either::Either,
    html::attribute::Attribute,
    hydration::Cursor,
    reactive_graph::{OwnedView, RenderEffectState},
    renderer::Renderer,
    ssr::StreamBuilder,
    view::{
        add_attr::AddAnyAttr,
        either::{EitherKeepAlive, EitherKeepAliveState},
        Position, PositionState, Render, RenderHtml,
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
    let fallback = fallback.run();
    let children = children.into_inner()();
    let tasks = ArcRwSignal::new(SlotMap::<DefaultKey, ()>::new());
    provide_context(SuspenseContext {
        tasks: tasks.clone(),
    });
    let none_pending = ArcMemo::new(move |_| tasks.with(SlotMap::is_empty));

    OwnedView::new(SuspenseBoundary::<false, _, _> {
        none_pending,
        fallback,
        children,
    })
}

pub(crate) struct SuspenseBoundary<const TRANSITION: bool, Fal, Chil> {
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
    type State =
        RenderEffectState<EitherKeepAliveState<Chil::State, Fal::State, Rndr>>;

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
            none_pending,
            fallback,
            children,
        } = self;
        SuspenseBoundary {
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

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        self.fallback.to_html_with_buf(buf, position);
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        mut self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        buf.next_id();
        let suspense_context = use_context::<SuspenseContext>().unwrap();

        let tasks = suspense_context.tasks.clone();
        let (tx, rx) = futures::channel::oneshot::channel::<()>();

        let mut tx = Some(tx);
        let eff =
            reactive_graph::effect::RenderEffect::new_isomorphic(move |_| {
                tasks.track();
                if tasks.read().is_empty() {
                    if let Some(tx) = tx.take() {
                        // If the receiver has dropped, it means the ScopedFuture has already
                        // dropped, so it doesn't matter if we manage to send this.
                        _ = tx.send(());
                    }
                }
            });

        // walk over the tree of children once to make sure that all resource loads are registered
        self.children.dry_resolve();

        let mut fut =
            Box::pin(ScopedFuture::new(ErrorHookFuture::new(async {
                // wait for all the resources to have loaded before trying to resolve the body
                // this is *less efficient* than just resolving the body
                // however, it means that you can use reactive accesses to resources/async derived
                // inside component props, at any level, and have those picked up by Suspense, and
                // that it will wait for those to resolve
                _ = rx.await;

                // if we ran this earlier, reactive reads would always be registered as None
                // this is fine in the case where we want to use Suspend and .await on some future
                // but in situations like a <For each=|| some_resource.snapshot()/> we actually
                // want to be able to 1) synchronously read a resource's value, but still 2) wait
                // for it to load before we render everything
                let children = self.children.resolve().await;

                // clean up the (now useless) effect
                drop(eff);
                children
            })));
        match fut.as_mut().now_or_never() {
            Some(resolved) => {
                Either::<Fal, _>::Right(resolved)
                    .to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
            }
            None => {
                let id = buf.clone_id();

                // out-of-order streams immediately push fallback,
                // wrapped by suspense markers
                if OUT_OF_ORDER {
                    let mut fallback_position = *position;
                    buf.push_fallback(self.fallback, &mut fallback_position);
                    buf.push_async_out_of_order(
                        false, /* TODO should_block */ fut, position,
                    );
                } else {
                    buf.push_async(
                        false, // TODO should_block
                        {
                            let mut position = *position;
                            async move {
                                let value = fut.await;
                                let mut builder = StreamBuilder::new(id);
                                Either::<Fal, _>::Right(value)
                                    .to_html_async_with_buf::<OUT_OF_ORDER>(
                                        &mut builder,
                                        &mut position,
                                    );
                                builder.finish().take_chunks()
                            }
                        },
                    );
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
            let show_b = !none_pending.get() && (!TRANSITION || nth_run < 1);
            nth_run += 1;
            EitherKeepAlive {
                a: children.take(),
                b: fallback.take(),
                show_b,
            }
        })
        .hydrate::<FROM_SERVER>(cursor, position)
    }
}
