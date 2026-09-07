use crate::{
    ChooseView, MatchInterface, MatchNestedRoutes, PathSegment, RouteList,
    RouteListing, RouteMatchId,
    hooks::Matched,
    location::{LocationProvider, Url},
    matching::{MatchParams, RouteDefs},
    nested_router::wait_until_route_settled,
    params::ParamsMap,
    view_transition::start_view_transition,
};
use any_spawner::Executor;
use either_of::Either;
use futures::{FutureExt, channel::oneshot};
use leptos::attr::{Attribute, any_attribute::AnyAttribute};
use reactive_graph::{
    computed::{ArcMemo, ScopedFuture, suspense::RouteSettleContext},
    owner::{Owner, provide_context},
    signal::ArcRwSignal,
    traits::{GetUntracked, ReadUntracked, Set},
    transition::AsyncTransition,
    wrappers::write::SignalSetter,
};
use std::{cell::RefCell, iter, mem, rc::Rc};
use tachys::{
    hydration::Cursor,
    reactive_graph::OwnedView,
    ssr::StreamBuilder,
    view::{
        MarkBranch, Mountable, Position, PositionState, Render, RenderFlags,
        RenderHtml,
        add_attr::AddAnyAttr,
        any_view::{AnyView, AnyViewState, IntoAny},
    },
};

pub(crate) struct FlatRoutesView<Loc, Defs, FalFn> {
    pub current_url: ArcRwSignal<Url>,
    pub location: Option<Loc>,
    pub routes: RouteDefs<Defs>,
    pub fallback: FalFn,
    pub outer_owner: Owner,
    pub set_is_routing: Option<SignalSetter<bool>>,
    pub transition: bool,
}

/// Retained view state for the flat router.
pub(crate) struct FlatRoutesViewState {
    #[allow(clippy::type_complexity)]
    view: AnyViewState,
    id: Option<RouteMatchId>,
    // identifies the route instance: replaced whenever a new route is
    // rendered, so a navigation task knows whether its view is still wanted
    owner: Owner,
    // identifies the navigation: incremented on every navigation, so a task
    // knows whether it is still the newest one and may complete the
    // navigation (clear is_routing, complete the location)
    navigation: u64,
    // the settle context of the route instance's view, when it was chosen
    // during a navigation that uses set_is_routing: polled by every
    // navigation that displays this route instance, whether it chose the
    // view or reused it with new params
    route_settle: Option<RouteSettleContext>,
    params: ArcRwSignal<ParamsMap>,
    path: String,
    url: ArcRwSignal<Url>,
    matched: ArcRwSignal<String>,
}

impl Mountable for FlatRoutesViewState {
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

    fn elements(&self) -> Vec<tachys::renderer::types::Element> {
        self.view.elements()
    }
}

impl<Loc, Defs, FalFn, Fal> Render for FlatRoutesView<Loc, Defs, FalFn>
where
    Loc: LocationProvider,
    Defs: MatchNestedRoutes + 'static,
    FalFn: FnOnce() -> Fal + Send,
    Fal: IntoAny,
{
    type State = Rc<RefCell<FlatRoutesViewState>>;

    fn build(self) -> Self::State {
        let FlatRoutesView {
            current_url,
            routes,
            fallback,
            outer_owner,
            set_is_routing,
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
                view: fallback().into_any().build(),
                id,
                owner,
                navigation: 0,
                route_settle: None,
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

                // is_routing is not set for the initial load, but the route
                // gets a settle context, so a params-only navigation that
                // reuses it while it is still loading waits for it
                let route_settle =
                    set_is_routing.map(|_| RouteSettleContext::new());
                let build_guard = route_settle.as_ref().and_then(|c| c.task());
                let mut view = Box::pin(owner.with(|| {
                    provide_context(params_memo);
                    provide_context(url.clone());
                    provide_context(Matched(ArcMemo::from(matched.clone())));
                    if let Some(route_settle) = &route_settle {
                        provide_context(route_settle.clone());
                    }

                    ScopedFuture::new(async move {
                        OwnedView::new(view.choose().await)
                    })
                }));

                // close the context once the initial load has settled, so
                // boundaries created later do not register with it
                // (is_routing is not involved)
                let close_when_settled = route_settle.clone();
                let close_when_settled = move || {
                    if let Some(route_settle) = close_when_settled {
                        Executor::spawn_local(wait_until_route_settled(
                            route_settle,
                        ));
                    }
                };

                match view.as_mut().now_or_never() {
                    Some(view) => {
                        let view = view.into_any().build();
                        drop(build_guard);
                        close_when_settled();
                        Rc::new(RefCell::new(FlatRoutesViewState {
                            view,
                            id,
                            owner,
                            navigation: 0,
                            route_settle,
                            params,
                            path,
                            url,
                            matched,
                        }))
                    }
                    None => {
                        let spawned_owner = owner.clone();
                        let state =
                            Rc::new(RefCell::new(FlatRoutesViewState {
                                view: ().into_any().build(),
                                id,
                                owner,
                                navigation: 0,
                                route_settle,
                                params,
                                path,
                                url,
                                matched,
                            }));

                        Executor::spawn_local({
                            let state = Rc::clone(&state);
                            async move {
                                let view = view.await;
                                let is_current =
                                    state.borrow().owner == spawned_owner;
                                if is_current {
                                    view.into_any()
                                        .rebuild(&mut state.borrow_mut().view);
                                }
                                drop(build_guard);
                                close_when_settled();
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
            transition,
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
            let params = initial_state.params.clone();
            let matched = initial_state.matched.clone();
            // this is a navigation of its own (so an older one must not
            // complete it), but it keeps the route instance (so an older
            // navigation to this route may still render its view, and this
            // navigation waits for that view to settle too)
            let navigation_id = initial_state.navigation.wrapping_add(1);
            initial_state.navigation = navigation_id;
            let route_settle = initial_state.route_settle.clone();
            drop(initial_state);
            let update = move || {
                params.set(matched_params);
                matched.set(matched_string);
            };
            match set_is_routing {
                // nothing is chosen or built, so hold is_routing for the
                // resources that reload because of the new params, and for
                // the route's view if it is still loading
                Some(set_is_routing) => {
                    set_is_routing.set(true);
                    let ((), reloaded) = AsyncTransition::track(update);
                    Executor::spawn_local({
                        let state = Rc::clone(state);
                        async move {
                            reloaded.await;
                            if let Some(route_settle) = route_settle {
                                wait_until_route_settled(route_settle).await;
                            }
                            if state.borrow().navigation == navigation_id {
                                set_is_routing.set(false);
                                if let Some(location) = location {
                                    location.ready_to_complete();
                                }
                            }
                        }
                    });
                }
                None => {
                    update();
                    if let Some(location) = location {
                        location.ready_to_complete();
                    }
                }
            }
            return;
        }

        // otherwise, we need to update the retained path for diffing
        initial_state.id = new_id;
        let navigation_id = initial_state.navigation.wrapping_add(1);
        initial_state.navigation = navigation_id;

        // when set_is_routing is used, register a context the route's
        // suspense boundaries report readiness to, so we can keep is_routing
        // true until the built route (including any <ProtectedRoute> content,
        // whose resources are only created when the route is built) has
        // loaded. It belongs to the route instance: a later params-only
        // navigation to the same route polls it too.
        let route_settle = match (&new_match, set_is_routing) {
            (Some(_), Some(set_is_routing)) => {
                set_is_routing.set(true);
                Some(RouteSettleContext::new())
            }
            _ => None,
        };
        if let Some(previous) =
            mem::replace(&mut initial_state.route_settle, route_settle.clone())
        {
            previous.close();
        }
        // held until the view has been built, so the route cannot look
        // settled before its boundaries have had a chance to register
        let build_guard = route_settle.as_ref().and_then(|c| c.task());

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
                    fallback().into_any().rebuild(&mut state.borrow_mut().view)
                });
                // navigating to the fallback completes immediately; clear
                // is_routing here, because a superseded in-flight navigation
                // no longer clears it itself
                if let Some(set_is_routing) = set_is_routing {
                    set_is_routing.set(false);
                }
                if let Some(location) = location {
                    location.ready_to_complete();
                }
            }
            Some(new_match) => {
                let (view, child) = new_match.into_view_and_child();

                #[cfg(debug_assertions)]
                if child.is_some() {
                    panic!(
                        "<FlatRoutes> should not be used with nested routes."
                    );
                }

                let spawned_owner = owner.clone();

                let is_back = location
                    .as_ref()
                    .map(|nav| nav.is_back().get_untracked())
                    .unwrap_or(false);
                Executor::spawn_local(owner.with(|| {
                    provide_context(url);
                    provide_context(params_memo);
                    provide_context(Matched(ArcMemo::from(new_matched)));
                    if let Some(route_settle) = &route_settle {
                        provide_context(route_settle.clone());
                    }

                    ScopedFuture::new({
                        let state = Rc::clone(state);
                        async move {
                            let view =
                                OwnedView::new(if set_is_routing.is_some() {
                                    AsyncTransition::run(|| view.choose()).await
                                } else {
                                    view.choose().await
                                });

                            // only render the view if this route instance is
                            // still the one wanted (we may have navigated to
                            // another route, or to this route again, before
                            // it loaded); only complete the navigation if no
                            // newer navigation has started since, which a
                            // params-only navigation to the same route does
                            // without replacing the route instance
                            let is_current_route = {
                                let state = Rc::clone(&state);
                                let spawned_owner = spawned_owner.clone();
                                move || state.borrow().owner == spawned_owner
                            };
                            let is_current_navigation = {
                                let state = Rc::clone(&state);
                                move || {
                                    state.borrow().navigation == navigation_id
                                }
                            };
                            if is_current_route() {
                                let (built_tx, built_rx) =
                                    oneshot::channel::<()>();
                                let rebuild = {
                                    let is_current_route =
                                        is_current_route.clone();
                                    move || {
                                        if is_current_route() {
                                            view.into_any().rebuild(
                                                &mut state.borrow_mut().view,
                                            );
                                        }
                                        drop(build_guard);
                                        _ = built_tx.send(());
                                    }
                                };
                                if transition {
                                    start_view_transition(0, is_back, rebuild);
                                } else {
                                    rebuild();
                                }
                                // a view transition defers the rebuild; the
                                // route's boundaries only register once it has
                                // been built, so wait for it before polling
                                _ = built_rx.await;

                                // wait for the built route to settle before
                                // clearing is_routing
                                if let Some(route_settle) = route_settle {
                                    wait_until_route_settled(route_settle)
                                        .await;
                                }
                                if is_current_navigation() {
                                    if let Some(set_is_routing) = set_is_routing
                                    {
                                        set_is_routing.set(false);
                                    }
                                    if let Some(location) = location {
                                        location.ready_to_complete();
                                    }
                                }
                            } else {
                                // never rendered: nothing can register with
                                // its context, which a newer navigation has
                                // already replaced
                                drop(build_guard);
                                if let Some(route_settle) = route_settle {
                                    route_settle.close();
                                }
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
    FalFn: FnOnce() -> Fal + Send + 'static,
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

#[derive(Debug)]
pub(crate) struct MatchedRoute(pub String, pub AnyView);

impl MatchedRoute {
    fn branch_name(&self) -> String {
        format!("{}|{:?}", self.0, self.1.as_type_id())
    }
}

impl Render for MatchedRoute {
    type State = <AnyView as Render>::State;

    fn build(self) -> Self::State {
        self.1.build()
    }

    fn rebuild(self, state: &mut Self::State) {
        self.1.rebuild(state);
    }
}

impl AddAnyAttr for MatchedRoute {
    type Output<SomeNewAttr: Attribute> = Self;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        let MatchedRoute(id, view) = self;
        MatchedRoute(id, view.add_any_attr(attr).into_any())
    }
}

impl RenderHtml for MatchedRoute {
    type AsyncOutput = Self;
    type Owned = Self;
    const MIN_LENGTH: usize = 0;

    fn dry_resolve(&mut self) {
        self.1.dry_resolve();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        let MatchedRoute(id, view) = self;
        let view = view.resolve().await;
        MatchedRoute(id, view)
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        flags: RenderFlags,
        extra_attrs: Vec<AnyAttribute>,
    ) {
        let branch_name =
            (flags.mark_branches && flags.escape).then(|| self.branch_name());
        if let Some(bn) = &branch_name {
            buf.open_branch(bn);
        }
        self.1.to_html_with_buf(buf, position, flags, extra_attrs);
        if let Some(bn) = &branch_name {
            buf.close_branch(bn);
            if *position == Position::NextChildAfterText {
                *position = Position::NextChild;
            }
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        flags: RenderFlags,
        extra_attrs: Vec<AnyAttribute>,
    ) where
        Self: Sized,
    {
        let branch_name =
            (flags.mark_branches && flags.escape).then(|| self.branch_name());
        if let Some(bn) = &branch_name {
            buf.open_branch(bn);
        }
        self.1.to_html_async_with_buf::<OUT_OF_ORDER>(
            buf,
            position,
            flags,
            extra_attrs,
        );
        if let Some(bn) = &branch_name {
            buf.close_branch(bn);
            if *position == Position::NextChildAfterText {
                *position = Position::NextChild;
            }
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        self.1.hydrate::<FROM_SERVER>(cursor, position)
    }

    async fn hydrate_async(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        self.1.hydrate_async(cursor, position).await
    }

    fn into_owned(self) -> Self::Owned {
        self
    }
}

impl<Loc, Defs, FalFn, Fal> FlatRoutesView<Loc, Defs, FalFn>
where
    Loc: LocationProvider + Send,
    Defs: MatchNestedRoutes + Send + 'static,
    FalFn: FnOnce() -> Fal + Send,
    Fal: RenderHtml + 'static,
{
    fn choose_ssr(self) -> OwnedView<AnyView> {
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
            None => (self.fallback)().into_any(),
            Some(new_match) => {
                let id = new_match.as_matched().to_string();
                let (view, _) = new_match.into_view_and_child();
                let view = owner
                    .with(|| {
                        provide_context(url);
                        provide_context(params_memo);
                        provide_context(Matched(ArcMemo::from(matched)));

                        ScopedFuture::new(async move { view.choose().await })
                    })
                    .now_or_never()
                    .expect("async route used in SSR");
                let view = MatchedRoute(id, view);
                view.into_any()
            }
        };

        OwnedView::new_with_owner(view, owner)
    }
}

impl<Loc, Defs, FalFn, Fal> RenderHtml for FlatRoutesView<Loc, Defs, FalFn>
where
    Loc: LocationProvider + Send,
    Defs: MatchNestedRoutes + Send + 'static,
    FalFn: FnOnce() -> Fal + Send + 'static,
    Fal: RenderHtml + 'static,
{
    type AsyncOutput = Self;
    type Owned = Self;

    const MIN_LENGTH: usize = <Either<Fal, AnyView> as RenderHtml>::MIN_LENGTH;

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        flags: RenderFlags,
        extra_attrs: Vec<AnyAttribute>,
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
            view.to_html_with_buf(buf, position, flags, extra_attrs);
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        flags: RenderFlags,
        extra_attrs: Vec<AnyAttribute>,
    ) where
        Self: Sized,
    {
        let view = self.choose_ssr();
        view.to_html_async_with_buf::<OUT_OF_ORDER>(
            buf,
            position,
            flags,
            extra_attrs,
        )
    }

    #[track_caller]
    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
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
                view: fallback()
                    .into_any()
                    .hydrate::<FROM_SERVER>(cursor, position),
                id,
                owner,
                navigation: 0,
                route_settle: None,
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
                    provide_context(params_memo);
                    provide_context(url.clone());
                    provide_context(Matched(ArcMemo::from(matched.clone())));

                    ScopedFuture::new(async move {
                        OwnedView::new(view.choose().await)
                    })
                }));

                match view.as_mut().now_or_never() {
                    Some(view) => Rc::new(RefCell::new(FlatRoutesViewState {
                        view: view
                            .into_any()
                            .hydrate::<FROM_SERVER>(cursor, position),
                        id,
                        owner,
                        navigation: 0,
                        route_settle: None,
                        params,
                        path,
                        url,
                        matched,
                    })),
                    None => {
                        panic!(
                            "lazy routes should not be used with \
                             hydrate_body(); use hydrate_lazy() instead"
                        );
                    }
                }
            }
        }
    }

    async fn hydrate_async(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
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
                view: fallback()
                    .into_any()
                    .hydrate_async(cursor, position)
                    .await,
                id,
                owner,
                navigation: 0,
                route_settle: None,
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

                let view = Box::pin(owner.with(|| {
                    provide_context(params_memo);
                    provide_context(url.clone());
                    provide_context(Matched(ArcMemo::from(matched.clone())));

                    ScopedFuture::new(async move {
                        OwnedView::new(view.choose().await)
                    })
                }));

                let view = view.await;

                Rc::new(RefCell::new(FlatRoutesViewState {
                    view: view.into_any().hydrate_async(cursor, position).await,
                    id,
                    owner,
                    navigation: 0,
                    route_settle: None,
                    params,
                    path,
                    url,
                    matched,
                }))
            }
        }
    }

    fn into_owned(self) -> Self::Owned {
        self
    }
}
