use crate::{
    hooks::Matched,
    location::{LocationProvider, Url},
    matching::Routes,
    params::ParamsMap,
    ChooseView, MatchInterface, MatchNestedRoutes, MatchParams, PathSegment,
    RouteList, RouteListing, RouteMatchId,
};
use any_spawner::Executor;
use either_of::{Either, EitherOf3};
use futures::FutureExt;
use reactive_graph::{
    computed::{ArcMemo, ScopedFuture},
    owner::{provide_context, Owner},
    signal::ArcRwSignal,
    traits::{ReadUntracked, Set},
    transition::AsyncTransition,
    wrappers::write::SignalSetter,
};
use std::{cell::RefCell, iter, mem, rc::Rc};
use tachys::{
    hydration::Cursor,
    reactive_graph::OwnedView,
    ssr::StreamBuilder,
    view::{
        add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
        RenderHtml,
    },
};

pub(crate) struct FlatRoutesView<Loc, Defs, FalFn> {
    pub current_url: ArcRwSignal<Url>,
    pub location: Option<Loc>,
    pub routes: Routes<Defs>,
    pub fallback: FalFn,
    pub outer_owner: Owner,
    pub set_is_routing: Option<SignalSetter<bool>>,
}

pub struct FlatRoutesViewState<Defs, Fal>
where
    Defs: MatchNestedRoutes + 'static,
    Fal: Render + 'static,
{
    #[allow(clippy::type_complexity)]
    view: <EitherOf3<(), Fal, OwnedView<<Defs::Match as MatchInterface>::View>> as Render>::State,
    id: Option<RouteMatchId>,
    owner: Owner,
    params: ArcRwSignal<ParamsMap>,
    path: String,
    url: ArcRwSignal<Url>,
        matched: ArcRwSignal<String>
}

impl<Defs, Fal> Mountable for FlatRoutesViewState<Defs, Fal>
where
    Defs: MatchNestedRoutes + 'static,
    Fal: Render + 'static,
{
    fn unmount(&mut self) {
        self.view.unmount();
    }

    fn mount(
        &mut self,
        parent: &leptos::tachys::renderer::types::Element,
        marker: Option<&leptos::tachys::renderer::types::Node>,
    ) {
        self.view.mount(parent, marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.view.insert_before_this(child)
    }
}

impl<Loc, Defs, FalFn, Fal> Render for FlatRoutesView<Loc, Defs, FalFn>
where
    Loc: LocationProvider,
    Defs: MatchNestedRoutes + 'static,
    FalFn: FnOnce() -> Fal + Send,
    Fal: Render + 'static,
{
    type State = Rc<RefCell<FlatRoutesViewState<Defs, Fal>>>;

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
        let matched = ArcRwSignal::new(
            new_match
                .as_ref()
                .map(|n| n.as_matched().to_owned())
                .unwrap_or_default(),
        );

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
        let params_memo = ArcMemo::from(params.clone());

        // release URL lock
        drop(current_url);

        match new_match {
            None => Rc::new(RefCell::new(FlatRoutesViewState {
                view: EitherOf3::B(fallback()).build(),
                id,
                owner,
                params,
                path,
                url,
                matched,
            })),
            Some(new_match) => {
                let (view, child) = new_match.into_view_and_child();

                #[cfg(debug_assertions)]
                if child.is_some() {
                    panic!(
                        "<FlatRoutes> should not be used with nested routes."
                    );
                }

                let mut view = Box::pin(owner.with(|| {
                    ScopedFuture::new({
                        let url = url.clone();
                        let matched = matched.clone();
                        async move {
                            provide_context(params_memo);
                            provide_context(url);
                            provide_context(Matched(ArcMemo::from(matched)));
                            OwnedView::new(view.choose().await)
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
                        matched,
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
                                matched,
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
            set_is_routing,
        } = self;
        let url_snapshot = current_url.read_untracked();

        // if the path is the same, we do not need to re-route
        // we can just update the search query and go about our day
        let mut initial_state = state.borrow_mut();
        if url_snapshot.path() == initial_state.path {
            initial_state.url.set(url_snapshot.to_owned());
            if let Some(location) = location {
                location.ready_to_complete();
            }
            return;
        }

        // since the path didn't match, we'll update the retained path for future diffing
        initial_state.path.clear();
        initial_state.path.push_str(url_snapshot.path());

        // otherwise, match the new route
        let new_match = routes.match_route(url_snapshot.path());
        let new_id = new_match.as_ref().map(|n| n.as_id());
        let matched_string = new_match
            .as_ref()
            .map(|n| n.as_matched().to_owned())
            .unwrap_or_default();
        let matched_params = new_match
            .as_ref()
            .map(|n| n.to_params().into_iter().collect())
            .unwrap_or_default();

        // if it's the same route, we just update the params
        if new_id == initial_state.id {
            initial_state.params.set(matched_params);
            initial_state.matched.set(matched_string);
            if let Some(location) = location {
                location.ready_to_complete();
            }
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
        let params_memo = ArcMemo::from(params.clone());
        let old_owner = mem::replace(&mut initial_state.owner, owner.clone());
        let old_url = mem::replace(&mut initial_state.url, url.clone());
        let old_params =
            mem::replace(&mut initial_state.params, params.clone());
        let new_matched = ArcRwSignal::new(matched_string);
        let old_matched =
            mem::replace(&mut initial_state.matched, new_matched.clone());

        // we drop the route state here, in case there is a <Redirect/> or similar that occurs
        // while rendering either the fallback or the new route
        drop(initial_state);

        match new_match {
            // render fallback
            None => {
                owner.with(|| {
                    provide_context(url);
                    provide_context(params_memo);
                    provide_context(Matched(ArcMemo::from(new_matched)));
                    EitherOf3::B(fallback())
                        .rebuild(&mut state.borrow_mut().view)
                });
            }
            Some(new_match) => {
                let (view, child) = new_match.into_view_and_child();

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
                            provide_context(params_memo);
                            provide_context(Matched(ArcMemo::from(
                                new_matched,
                            )));
                            let view = OwnedView::new(
                                if let Some(set_is_routing) = set_is_routing {
                                    set_is_routing.set(true);
                                    let value =
                                        AsyncTransition::run(|| view.choose())
                                            .await;
                                    set_is_routing.set(false);
                                    value
                                } else {
                                    view.choose().await
                                },
                            );

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
                            drop(old_matched);
                        }
                    })
                }));
            }
        }
    }
}

impl<Loc, Defs, FalFn, Fal> AddAnyAttr for FlatRoutesView<Loc, Defs, FalFn>
where
    Loc: LocationProvider + Send,
    Defs: MatchNestedRoutes + Send + 'static,
    FalFn: FnOnce() -> Fal + Send,
    Fal: RenderHtml + 'static,
{
    type Output<SomeNewAttr: leptos::attr::Attribute> =
        FlatRoutesView<Loc, Defs, FalFn>;

    fn add_any_attr<NewAttr: leptos::attr::Attribute>(
        self,
        _attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        todo!()
    }
}

impl<Loc, Defs, FalFn, Fal> FlatRoutesView<Loc, Defs, FalFn>
where
    Loc: LocationProvider + Send,
    Defs: MatchNestedRoutes + Send + 'static,
    FalFn: FnOnce() -> Fal + Send,
    Fal: RenderHtml + 'static,
{
    fn choose_ssr(
        self,
    ) -> OwnedView<Either<Fal, <Defs::Match as MatchInterface>::View>> {
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
        let matched = ArcRwSignal::new(
            new_match
                .as_ref()
                .map(|n| n.as_matched().to_owned())
                .unwrap_or_default(),
        );
        let params_memo = ArcMemo::from(params.clone());

        // release URL lock
        drop(current_url);

        let view = match new_match {
            None => Either::Left((self.fallback)()),
            Some(new_match) => {
                let (view, _) = new_match.into_view_and_child();
                let view = owner
                    .with(|| {
                        ScopedFuture::new(async move {
                            provide_context(url);
                            provide_context(params_memo);
                            provide_context(Matched(ArcMemo::from(matched)));
                            view.choose().await
                        })
                    })
                    .now_or_never()
                    .expect("async route used in SSR");
                Either::Right(view)
            }
        };

        OwnedView::new_with_owner(view, owner)
    }
}

impl<Loc, Defs, FalFn, Fal> RenderHtml for FlatRoutesView<Loc, Defs, FalFn>
where
    Loc: LocationProvider + Send,
    Defs: MatchNestedRoutes + Send + 'static,
    FalFn: FnOnce() -> Fal + Send,
    Fal: RenderHtml + 'static,
{
    type AsyncOutput = Self;

    const MIN_LENGTH: usize = <Either<
        Fal,
        <Defs::Match as MatchInterface>::View,
    > as RenderHtml>::MIN_LENGTH;

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
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
                        data.methods,
                        data.regenerate,
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
            let view = self.choose_ssr();
            view.to_html_with_buf(buf, position, escape, mark_branches);
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
        let view = self.choose_ssr();
        view.to_html_async_with_buf::<OUT_OF_ORDER>(
            buf,
            position,
            escape,
            mark_branches,
        )
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
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
        let matched = ArcRwSignal::new(
            new_match
                .as_ref()
                .map(|n| n.as_matched().to_owned())
                .unwrap_or_default(),
        );

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
        let params_memo = ArcMemo::from(params.clone());

        // release URL lock
        drop(current_url);

        match new_match {
            None => Rc::new(RefCell::new(FlatRoutesViewState {
                view: EitherOf3::B(fallback())
                    .hydrate::<FROM_SERVER>(cursor, position),
                id,
                owner,
                params,
                path,
                url,
                matched,
            })),
            Some(new_match) => {
                let (view, child) = new_match.into_view_and_child();

                #[cfg(debug_assertions)]
                if child.is_some() {
                    panic!(
                        "<FlatRoutes> should not be used with nested routes."
                    );
                }

                let mut view = Box::pin(owner.with(|| {
                    ScopedFuture::new({
                        let url = url.clone();
                        let matched = matched.clone();
                        async move {
                            provide_context(params_memo);
                            provide_context(url);
                            provide_context(Matched(ArcMemo::from(matched)));
                            OwnedView::new(view.choose().await)
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
                        matched,
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
