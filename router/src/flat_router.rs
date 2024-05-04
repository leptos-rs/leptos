use crate::{
    location::{Location, RequestUrl, Url},
    matching::Routes,
    params::ParamsMap,
    resolve_path::resolve_path,
    ChooseView, MatchInterface, MatchNestedRoutes, MatchParams, Method,
    PathSegment, RouteList, RouteListing, RouteMatchId,
};
use either_of::Either;
use leptos::{component, oco::Oco, IntoView};
use or_poisoned::OrPoisoned;
use reactive_graph::{
    computed::{ArcMemo, Memo},
    owner::{provide_context, use_context, Owner},
    signal::{ArcRwSignal, ArcTrigger},
    traits::{Get, Read, ReadUntracked, Set, Track, Trigger},
};
use std::{
    borrow::Cow,
    iter,
    marker::PhantomData,
    mem,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
};
use tachys::{
    hydration::Cursor,
    renderer::Renderer,
    ssr::StreamBuilder,
    view::{
        add_attr::AddAnyAttr,
        any_view::{AnyView, AnyViewState, IntoAny},
        either::EitherState,
        Mountable, Position, PositionState, Render, RenderHtml,
    },
};

pub(crate) struct FlatRoutesView<Defs, Fal, R> {
    pub routes: Routes<Defs, R>,
    pub path: ArcMemo<String>,
    pub fallback: Fal,
    pub outer_owner: Owner,
    pub params: ArcRwSignal<ParamsMap>,
}

impl<Defs, Fal, R> FlatRoutesView<Defs, Fal, R>
where
    Defs: MatchNestedRoutes<R>,
    Fal: Render<R>,
    R: Renderer + 'static,
{
    pub fn choose(
        self,
    ) -> Either<Fal, <Defs::Match as MatchInterface<R>>::View> {
        let FlatRoutesView {
            routes,
            path,
            fallback,
            outer_owner,
            params,
        } = self;

        outer_owner.with(|| {
            provide_context(params.clone().read_only());
            let new_match = routes.match_route(&path.read());
            match new_match {
                None => Either::Left(fallback),
                Some(matched) => {
                    let new_params =
                        matched.to_params().into_iter().collect::<ParamsMap>();
                    params.set(new_params);
                    let (view, child) = matched.into_view_and_child();

                    #[cfg(debug_assertions)]
                    if child.is_some() {
                        panic!(
                            "<FlatRoutes> should not be used with nested \
                             routes."
                        );
                    }

                    let view = view.choose();
                    Either::Right(view)
                }
            }
        })
    }
}

impl<Defs, Fal, R> Render<R> for FlatRoutesView<Defs, Fal, R>
where
    Defs: MatchNestedRoutes<R>,
    Fal: Render<R>,
    R: Renderer + 'static,
{
    type State = <Either<Fal, <Defs::Match as MatchInterface<R>>::View> as Render<R>>::State;

    fn build(self) -> Self::State {
        self.choose().build()
    }

    fn rebuild(self, state: &mut Self::State) {
        self.choose().rebuild(state);
    }
}

impl<Defs, Fal, R> AddAnyAttr<R> for FlatRoutesView<Defs, Fal, R>
where
    Defs: MatchNestedRoutes<R> + Send,
    Fal: RenderHtml<R>,
    R: Renderer + 'static,
{
    type Output<SomeNewAttr: leptos::attr::Attribute<R>> =
        FlatRoutesView<Defs, Fal, R>;

    fn add_any_attr<NewAttr: leptos::attr::Attribute<R>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<R>,
    {
        todo!()
    }
}

impl<Defs, Fal, R> RenderHtml<R> for FlatRoutesView<Defs, Fal, R>
where
    Defs: MatchNestedRoutes<R> + Send,
    Fal: RenderHtml<R>,
    R: Renderer + 'static,
{
    type AsyncOutput = Self;

    const MIN_LENGTH: usize = <Either<
        Fal,
        <Defs::Match as MatchInterface<R>>::View,
    > as RenderHtml<R>>::MIN_LENGTH;

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        // if this is being run on the server for the first time, generating all possible routes
        if RouteList::is_generating() {
            // add routes
            let (base, routes) = self.routes.generate_routes();
            let mut routes = routes
                .into_iter()
                .map(|data| {
                    let path = base
                        .into_iter()
                        .flat_map(|base| {
                            iter::once(PathSegment::Static(
                                base.to_string().into(),
                            ))
                        })
                        .chain(data.segments)
                        .collect::<Vec<_>>();
                    RouteListing::new(
                        path,
                        data.ssr_mode,
                        // TODO methods
                        [Method::Get],
                        // TODO static data
                        None,
                    )
                })
                .collect::<Vec<_>>();

            // add fallback
            // TODO fix: causes overlapping route issues on Axum
            /*routes.push(RouteListing::new(
                [PathSegment::Static(
                    base.unwrap_or_default().to_string().into(),
                )],
                SsrMode::Async,
                [
                    Method::Get,
                    Method::Post,
                    Method::Put,
                    Method::Patch,
                    Method::Delete,
                ],
                None,
            ));*/

            RouteList::register(RouteList::from(routes));
        } else {
            self.choose().to_html_with_buf(buf, position);
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        self.choose()
            .to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        self.choose().hydrate::<FROM_SERVER>(cursor, position)
    }
}
