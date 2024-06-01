use crate::{
    location::{LocationProvider, Url},
    matching::Routes,
    params::ParamsMap,
    ChooseView, MatchInterface, MatchNestedRoutes, MatchParams, Method,
    PathSegment, RouteList, RouteListing, RouteMatchId,
};
use any_spawner::Executor;
use either_of::{Either, EitherOf3};
use futures::FutureExt;
use reactive_graph::{
    computed::{ScopedFuture},
    owner::{provide_context, Owner},
    signal::{ArcRwSignal},
    traits::{ReadUntracked, Set},
    transition::AsyncTransition,
    wrappers::write::SignalSetter,
};
use std::{
    cell::RefCell,
    iter,
    mem,
    rc::Rc,
};
use tachys::{
    hydration::Cursor,
    renderer::Renderer,
    ssr::StreamBuilder,
    view::{
        add_attr::AddAnyAttr,
        Mountable, Position, PositionState, Render, RenderHtml,
    },
};

pub(crate) struct FlatRoutesView<Loc, Defs, Fal, R> {
    pub current_url: ArcRwSignal<Url>,
    pub location: Option<Loc>,
    pub routes: Routes<Defs, R>,
    pub fallback: Fal,
    pub outer_owner: Owner,
    pub set_is_routing: Option<SignalSetter<bool>>
}

/*
impl<Loc, Defs, Fal, R> FlatRoutesView<Loc, Defs, Fal, R>
where
    Loc: LocationProvider,
    Defs: MatchNestedRoutes<R>,
    Fal: Render<R>,
    R: Renderer + 'static,
{
    pub fn choose(
        self,
        prev_owner: Option<&Owner>,
        prev_id: Option<RouteMatchId>,
        prev_params: Option<ArcRwSignal<ParamsMap>>
    ) -> (
        Owner,
        Option<RouteMatchId>,
        ArcRwSignal<ParamsMap>,
        impl Future<Output = Either<Fal, <Defs::Match as MatchInterface<R>>::View>>,
    ) {
        let FlatRoutesView {
            routes,
            path,
            fallback,
            outer_owner,
            ..
        } = self;
        let new_match = routes.match_route(&path.read());
        let new_id = new_match.as_ref().map(|n| n.as_id());

        // update params or replace with new params signal
        // switching out the signal for a newly-created signal here means that navigating from,
        // for example, /foo/42 to /bar does not cause /foo/:id to respond to a change in `id`,
        // because the new set of params is set on a new signal
        let new_params = new_match
            .as_ref()
            .map(|matched| matched
                 .to_params()
                 .into_iter()
                 .collect::<ParamsMap>()).unwrap_or_default();
        let new_params_signal = match prev_params {
            Some(prev_params) if prev_id == new_id => {
                prev_params.set(new_params);
                prev_params.clone()
            }
            _ => {
                let new_params_signal = ArcRwSignal::new(new_params);
                provide_context(ArcRwSignal::new(new_params_signal.clone()));
                                new_params_signal
            }
        };

        let owner = match prev_owner {
            Some(prev_owner) if prev_id == new_id => {
                prev_owner.clone()
            },
            _ => outer_owner.child()
        };

        let (id, fut) = owner.with(|| {
            let id = new_match.as_ref().map(|n| n.as_id());
            (
                id,
                ScopedFuture::new(match new_match {
                    None => EitherFuture::Left {
                        inner: async move { fallback },
                    },
                    Some(matched) => {
                        let (view, child) = matched.into_view_and_child();

                        #[cfg(debug_assertions)]
                        if child.is_some() {
                            panic!(
                                "<FlatRoutes> should not be used with nested \
                                 routes."
                            );
                        }

                        EitherFuture::Right {
                            inner: ScopedFuture::new({ let new_params_signal = new_params_signal.clone(); async move {
                                provide_context(new_params_signal.clone());
                                view.choose().await
                            }}),
                        }
                    }
                }),
            )
        });
        (owner, id, new_params_signal, fut)
    }
}
*/

pub struct FlatRoutesViewState<Defs, Fal, R> 
where
    Defs: MatchNestedRoutes<R> + 'static,
    Fal: Render<R> + 'static,
    R: Renderer + 'static
{
    #[allow(clippy::type_complexity)]
    view: <EitherOf3<(), Fal, <Defs::Match as MatchInterface<R>>::View> as Render<R>>::State,
    id: Option<RouteMatchId>,
    owner: Owner,
    params: ArcRwSignal<ParamsMap>,
    path: String,
    url: ArcRwSignal<Url>
}

impl<Defs, Fal, R> Mountable<R> for FlatRoutesViewState<Defs, Fal, R>
where
    Defs: MatchNestedRoutes<R> + 'static,
    Fal: Render<R> + 'static,
    R: Renderer + 'static,
{
    fn unmount(&mut self) {
        self.view.unmount();
    }

    fn mount(
        &mut self,
        parent: &<R as Renderer>::Element,
        marker: Option<&<R as Renderer>::Node>,
    ) {
        self.view.mount(parent, marker);
    }

    fn insert_before_this(
        &self,
        parent: &<R as Renderer>::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool {
        self.view.insert_before_this(parent, child)
    }
}

impl<Loc, Defs, Fal, R> Render<R> for FlatRoutesView<Loc, Defs, Fal, R>
where
    Loc: LocationProvider,
    Defs: MatchNestedRoutes<R> + 'static,
    Fal: Render<R> + 'static,
    R: Renderer + 'static,
{
    type State = Rc<RefCell<FlatRoutesViewState<Defs, Fal, R>>>;

    fn build(self) -> Self::State {
        let FlatRoutesView {
            current_url,
            routes,
            fallback,
            outer_owner,
            ..
        } = self;
        let current_url = current_url.read_untracked();

        // we always need to match the new route
        let new_match = routes.match_route(current_url.path());
        let id = new_match.as_ref().map(|n| n.as_id());

        // create default starting points for owner, url, path, and params
        // these will be held in state so that future navigations can update or replace them
        let owner = outer_owner.child();
        let url = ArcRwSignal::new(current_url.to_owned());
        let path = current_url.path().to_string();
        let params = ArcRwSignal::new(
            new_match
                .as_ref()
                .map(|n| n.to_params().into_iter().collect())
                .unwrap_or_default(),
        );

        match new_match {
            None => Rc::new(RefCell::new(FlatRoutesViewState {
                view: EitherOf3::B(fallback).build(),
                id,
                owner,
                params,
                path,
                url,
            })),
            Some(matched) => {
                let (view, child) = matched.into_view_and_child();

                #[cfg(debug_assertions)]
                if child.is_some() {
                    panic!(
                        "<FlatRoutes> should not be used with nested routes."
                    );
                }

                let mut view = Box::pin(owner.with(|| {
                    ScopedFuture::new({
                        let params = params.clone();
                        let url = url.clone();
                        async move {
                            provide_context(params);
                            provide_context(url);
                            view.choose().await
                        }
                    })
                }));

                match view.as_mut().now_or_never() {
                    Some(view) => Rc::new(RefCell::new(FlatRoutesViewState {
                        view: EitherOf3::C(view).build(),
                        id,
                        owner,
                        params,
                        path,
                        url,
                    })),
                    None => {
                        let state =
                            Rc::new(RefCell::new(FlatRoutesViewState {
                                view: EitherOf3::A(()).build(),
                                id,
                                owner,
                                params,
                                path,
                                url,
                            }));

                        Executor::spawn_local({
                            let state = Rc::clone(&state);
                            async move {
                                let view = view.await;
                                EitherOf3::C(view)
                                    .rebuild(&mut state.borrow_mut().view);
                            }
                        });

                        state
                    }
                }
            }
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        let FlatRoutesView {
            current_url,
            location,
            routes,
            fallback,
            outer_owner,
            set_is_routing
        } = self;
        let url_snapshot = current_url.read_untracked();

        // if the path is the same, we do not need to re-route
        // we can just update the search query and go about our day
        let mut initial_state = state.borrow_mut();
        if url_snapshot.path() == initial_state.path {
            initial_state.url.set(url_snapshot.to_owned());
            return;
        }

        // since the path didn't match, we'll update the retained path for future diffing
        initial_state.path.clear();
        initial_state.path.push_str(url_snapshot.path());

        // otherwise, match the new route
        let new_match = routes.match_route(url_snapshot.path());
        let new_id = new_match.as_ref().map(|n| n.as_id());
        let matched_params = new_match
            .as_ref()
            .map(|n| n.to_params().into_iter().collect())
            .unwrap_or_default();

        // if it's the same route, we just update the params
        if new_id == initial_state.id {
            initial_state.params.set(matched_params);
            return;
        }

        // otherwise, we need to update the retained path for diffing
        initial_state.id = new_id;

        // otherwise, it's a new route, so we'll need to
        // 1) create a new owner, URL signal, and params signal
        // 2) render the fallback or new route
        let owner = outer_owner.child();
        let url = ArcRwSignal::new(url_snapshot.to_owned());
        let params = ArcRwSignal::new(matched_params);
        let old_owner = mem::replace(&mut initial_state.owner, owner.clone());
        let old_url = mem::replace(&mut initial_state.url, url.clone());
        let old_params =
            mem::replace(&mut initial_state.params, params.clone());

        // we drop the route state here, in case there is a <Redirect/> or similar that occurs
        // while rendering either the fallback or the new route
        drop(initial_state);

        match new_match {
            // render fallback
            None => {
                owner.with(|| {
                    provide_context(url);
                    provide_context(params);
                    EitherOf3::B(fallback).rebuild(&mut state.borrow_mut().view)
                });
            }
            Some(matched) => {
                let (view, child) = matched.into_view_and_child();

                #[cfg(debug_assertions)]
                if child.is_some() {
                    panic!(
                        "<FlatRoutes> should not be used with nested routes."
                    );
                }

                let spawned_path = url_snapshot.path().to_string();

                Executor::spawn_local(owner.with(|| {
                    ScopedFuture::new({
                        let state = Rc::clone(state);
                        async move {
                            provide_context(url);
                            provide_context(params);
                            let view = if let Some(set_is_routing) = set_is_routing {
                                set_is_routing.set(true);
                                let value = AsyncTransition::run(|| view.choose()).await;
                                set_is_routing.set(false);
                                value
                            } else {
                                view.choose().await
                            };

                            // only update the route if it's still the current path
                            // i.e., if we've navigated away before this has loaded, do nothing
                            if current_url.read_untracked().path()
                                == spawned_path
                            {
                                EitherOf3::C(view)
                                    .rebuild(&mut state.borrow_mut().view);
                            }

                            if let Some(location) = location {
                                location.ready_to_complete();
                            }

                            drop(old_owner);
                            drop(old_params);
                            drop(old_url);
                        }
                    })
                }));
            }
        }
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
        _attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<R>,
    {
        todo!()
    }
}

impl<Loc, Defs, Fal, R> FlatRoutesView<Loc, Defs, Fal, R>
where
    Loc: LocationProvider + Send,
    Defs: MatchNestedRoutes<R> + Send + 'static,
    Fal: RenderHtml<R> + 'static,
    R: Renderer + 'static,
{
    fn choose_ssr(
        self,
    ) -> Either<Fal, <Defs::Match as MatchInterface<R>>::View> {
        let current_url = self.current_url.read_untracked();
        let new_match = self.routes.match_route(current_url.path());
        let owner = self.outer_owner.child();
        let url = ArcRwSignal::new(current_url.to_owned());
        let params = ArcRwSignal::new(
            new_match
                .as_ref()
                .map(|n| n.to_params().into_iter().collect::<ParamsMap>())
                .unwrap_or_default(),
        );
        match new_match {
            None => Either::Left(self.fallback),
            Some(matched) => {
                let (view, _) = matched.into_view_and_child();
                let view = owner
                    .with(|| {
                        ScopedFuture::new(async move {
                            provide_context(url);
                            provide_context(params);
                            view.choose().await
                        })
                    })
                    .now_or_never()
                    .expect("async route used in SSR");
                Either::Right(view)
            }
        }
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

    fn dry_resolve(&mut self) { }

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        // if this is being run on the server for the first time, generating all possible routes
        if RouteList::is_generating() {
            // add routes
            let (base, routes) = self.routes.generate_routes();
            let routes = routes
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
            self.choose_ssr().to_html_with_buf(buf, position);
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        self.choose_ssr()
            .to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        // this can be mostly the same as the build() implementation, but with hydrate()
        //
        // however, the big TODO is that we need to support lazy hydration in the case that the
        // route is lazy-loaded on the client -- in this case, we actually can't initially hydrate
        // at all, but need to skip, because the HTML will contain the route even though the
        // client-side route component code is not yet loaded
        let FlatRoutesView {
            current_url,
            routes,
            fallback,
            outer_owner,
            ..
        } = self;
        let current_url = current_url.read_untracked();

        // we always need to match the new route
        let new_match = routes.match_route(current_url.path());
        let id = new_match.as_ref().map(|n| n.as_id());

        // create default starting points for owner, url, path, and params
        // these will be held in state so that future navigations can update or replace them
        let owner = outer_owner.child();
        let url = ArcRwSignal::new(current_url.to_owned());
        let path = current_url.path().to_string();
        let params = ArcRwSignal::new(
            new_match
                .as_ref()
                .map(|n| n.to_params().into_iter().collect())
                .unwrap_or_default(),
        );

        match new_match {
            None => Rc::new(RefCell::new(FlatRoutesViewState {
                view: EitherOf3::B(fallback)
                    .hydrate::<FROM_SERVER>(cursor, position),
                id,
                owner,
                params,
                path,
                url,
            })),
            Some(matched) => {
                let (view, child) = matched.into_view_and_child();

                #[cfg(debug_assertions)]
                if child.is_some() {
                    panic!(
                        "<FlatRoutes> should not be used with nested routes."
                    );
                }

                let mut view = Box::pin(owner.with(|| {
                    ScopedFuture::new({
                        let params = params.clone();
                        let url = url.clone();
                        async move {
                            provide_context(params);
                            provide_context(url);
                            view.choose().await
                        }
                    })
                }));

                match view.as_mut().now_or_never() {
                    Some(view) => Rc::new(RefCell::new(FlatRoutesViewState {
                        view: EitherOf3::C(view)
                            .hydrate::<FROM_SERVER>(cursor, position),
                        id,
                        owner,
                        params,
                        path,
                        url,
                    })),
                    None => {
                        // see comment at the top of this function
                        todo!()
                    }
                }
            }
        }
    }
}
