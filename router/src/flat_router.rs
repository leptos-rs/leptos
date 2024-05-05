use crate::{
    location::{Location, LocationProvider, RequestUrl, Url},
    matching::Routes,
    params::ParamsMap,
    resolve_path::resolve_path,
    ChooseView, MatchInterface, MatchNestedRoutes, MatchParams, Method,
    PathSegment, RouteList, RouteListing, RouteMatchId,
};
use any_spawner::Executor;
use either_of::{Either, EitherFuture, EitherOf3};
use leptos::{component, oco::Oco, IntoView};
use or_poisoned::OrPoisoned;
use reactive_graph::{
    computed::{ArcMemo, Memo, ScopedFuture},
    owner::{provide_context, use_context, Owner},
    signal::{ArcRwSignal, ArcTrigger},
    traits::{Get, GetUntracked, Read, ReadUntracked, Set, Track, Trigger},
};
use std::{
    borrow::Cow,
    cell::RefCell,
    iter,
    marker::PhantomData,
    mem,
    rc::Rc,
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

pub(crate) struct FlatRoutesView<Loc, Defs, Fal, R> {
    pub location: Option<Loc>,
    pub routes: Routes<Defs, R>,
    pub path: ArcMemo<String>,
    pub fallback: Fal,
    pub outer_owner: Owner,
    pub params: ArcRwSignal<ParamsMap>,
}

impl<Loc, Defs, Fal, R> FlatRoutesView<Loc, Defs, Fal, R>
where
    Loc: LocationProvider,
    Defs: MatchNestedRoutes<R>,
    Fal: Render<R>,
    R: Renderer + 'static,
{
    pub async fn choose(
        self,
    ) -> Either<Fal, <Defs::Match as MatchInterface<R>>::View> {
        let FlatRoutesView {
            routes,
            path,
            fallback,
            outer_owner,
            params,
            ..
        } = self;

        outer_owner
            .with(|| {
                provide_context(params.clone().read_only());
                let new_match = routes.match_route(&path.read());
                match new_match {
                    None => EitherFuture::Left {
                        inner: async move { fallback },
                    },
                    Some(matched) => {
                        let new_params = matched
                            .to_params()
                            .into_iter()
                            .collect::<ParamsMap>();
                        params.set(new_params);
                        let (view, child) = matched.into_view_and_child();

                        #[cfg(debug_assertions)]
                        if child.is_some() {
                            panic!(
                                "<FlatRoutes> should not be used with nested \
                                 routes."
                            );
                        }

                        EitherFuture::Right {
                            inner: ScopedFuture::new(view.choose()),
                        }
                    }
                }
            })
            .await
    }
}

impl<Loc, Defs, Fal, R> Render<R> for FlatRoutesView<Loc, Defs, Fal, R>
where
    Loc: LocationProvider,
    Defs: MatchNestedRoutes<R> + 'static,
    Fal: Render<R> + 'static,
    R: Renderer + 'static,
{
    type State = Rc<
        RefCell<
            // TODO loading indicator
            <EitherOf3<(), Fal, <Defs::Match as MatchInterface<R>>::View> as Render<
                R,
            >>::State,
        >,
    >;

    fn build(self) -> Self::State {
        let state = Rc::new(RefCell::new(EitherOf3::A(()).build()));
        let spawned_path = self.path.get_untracked();
        let current_path = self.path.clone();
        let location = self.location.clone();
        let route = self.choose();
        Executor::spawn_local({
            let state = Rc::clone(&state);
            async move {
                let loaded_route = route.await;
                // only update the route if it's still the current path
                // i.e., if we've navigated away before this has loaded, do nothing
                if &spawned_path == &*current_path.read_untracked() {
                    let new_view = match loaded_route {
                        Either::Left(i) => EitherOf3::B(i),
                        Either::Right(i) => EitherOf3::C(i),
                    };
                    new_view.rebuild(&mut state.borrow_mut());
                    if let Some(location) = location {
                        location.ready_to_complete();
                    }
                }
            }
        });
        state
    }

    fn rebuild(self, state: &mut Self::State) {
        let spawned_path = self.path.get_untracked();
        let current_path = self.path.clone();
        let location = self.location.clone();
        let route = self.choose();
        Executor::spawn_local({
            let state = Rc::clone(&*state);
            async move {
                let loaded_route = route.await;
                // only update the route if it's still the current path
                // i.e., if we've navigated away before this has loaded, do nothing
                if &spawned_path == &*current_path.read_untracked() {
                    let new_view = match loaded_route {
                        Either::Left(i) => EitherOf3::B(i),
                        Either::Right(i) => EitherOf3::C(i),
                    };
                    new_view.rebuild(&mut state.borrow_mut());
                    if let Some(location) = location {
                        location.ready_to_complete();
                    }
                }
            }
        });
    }
}

impl<Loc, Defs, Fal, R> AddAnyAttr<R> for FlatRoutesView<Loc, Defs, Fal, R>
where
    Loc: LocationProvider + Send,
    Defs: MatchNestedRoutes<R> + Send + 'static,
    Fal: RenderHtml<R> + 'static,
    R: Renderer + 'static,
{
    type Output<SomeNewAttr: leptos::attr::Attribute<R>> =
        FlatRoutesView<Loc, Defs, Fal, R>;

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

impl<Loc, Defs, Fal, R> RenderHtml<R> for FlatRoutesView<Loc, Defs, Fal, R>
where
    Loc: LocationProvider + Send,
    Defs: MatchNestedRoutes<R> + Send + 'static,
    Fal: RenderHtml<R> + 'static,
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
            todo!()
            // self.choose().to_html_with_buf(buf, position);
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        todo!()
        //    self.choose()
        //       .to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        todo!()
        // self.choose().hydrate::<FROM_SERVER>(cursor, position)
    }
}
