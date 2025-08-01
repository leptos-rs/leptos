use crate::{
    flat_router::MatchedRoute,
    hooks::Matched,
    location::{LocationProvider, Url},
    matching::RouteDefs,
    params::ParamsMap,
    view_transition::start_view_transition,
    ChooseView, MatchInterface, MatchNestedRoutes, MatchParams, PathSegment,
    RouteList, RouteListing, RouteMatchId,
};
use any_spawner::Executor;
use either_of::{Either, EitherOf3};
use futures::{
    channel::oneshot,
    future::{join_all, AbortHandle, Abortable},
    FutureExt,
};
use leptos::{
    attr::any_attribute::AnyAttribute,
    component,
    oco::Oco,
    prelude::{ArcStoredValue, WriteValue},
};
use or_poisoned::OrPoisoned;
use reactive_graph::{
    computed::{ArcMemo, ScopedFuture},
    owner::{provide_context, use_context, Owner},
    signal::{ArcRwSignal, ArcTrigger},
    traits::{Get, GetUntracked, Notify, ReadUntracked, Set, Track},
    transition::AsyncTransition,
    wrappers::write::SignalSetter,
};
use send_wrapper::SendWrapper;
use std::{
    cell::RefCell,
    fmt::Debug,
    future::Future,
    iter, mem,
    pin::Pin,
    rc::Rc,
    sync::{Arc, Mutex},
};
use tachys::{
    hydration::Cursor,
    reactive_graph::{OwnedView, Suspend},
    ssr::StreamBuilder,
    view::{
        add_attr::AddAnyAttr,
        any_view::{AnyView, IntoAny},
        either::EitherOf3State,
        Mountable, Position, PositionState, Render, RenderHtml,
    },
};

pub(crate) struct NestedRoutesView<Loc, Defs, FalFn> {
    pub location: Option<Loc>,
    pub routes: RouteDefs<Defs>,
    pub outer_owner: Owner,
    pub current_url: ArcRwSignal<Url>,
    pub base: Option<Oco<'static, str>>,
    pub fallback: FalFn,
    pub set_is_routing: Option<SignalSetter<bool>>,
    pub transition: bool,
}

/// Retained view state for the nested router.
pub(crate) struct NestedRouteViewState<Fal>
where
    Fal: Render,
{
    path: String,
    current_url: ArcRwSignal<Url>,
    outlets: Vec<RouteContext>,
    // TODO loading fallback
    #[allow(clippy::type_complexity)]
    view: Rc<RefCell<EitherOf3State<(), Fal, AnyView>>>,
    // held to keep the Owner alive until the router is dropped
    #[allow(unused)]
    outer_owner: Owner,
    abort_navigation: ArcStoredValue<Option<AbortHandle>>,
}

impl<Loc, Defs, FalFn, Fal> Render for NestedRoutesView<Loc, Defs, FalFn>
where
    Loc: LocationProvider,
    Defs: MatchNestedRoutes,
    FalFn: FnOnce() -> Fal,
    Fal: Render + 'static,
{
    // TODO support fallback while loading
    type State = NestedRouteViewState<Fal>;

    fn build(self) -> Self::State {
        let NestedRoutesView {
            routes,
            outer_owner,
            current_url,
            fallback,
            base,
            ..
        } = self;

        let mut loaders = Vec::new();
        let mut outlets = Vec::new();
        let url = current_url.read_untracked();
        let path = url.path().to_string();

        // match the route
        let new_match = routes.match_route(url.path());

        // start with an empty view because we'll be loading routes async
        let view = EitherOf3::A(()).build();
        let view = Rc::new(RefCell::new(view));
        let matched_view = match new_match {
            None => EitherOf3::B(fallback()),
            Some(route) => {
                route.build_nested_route(
                    &url,
                    base,
                    &mut loaders,
                    &mut outlets,
                );
                drop(url);

                EitherOf3::C(top_level_outlet(&outlets, &outer_owner))
            }
        };

        Executor::spawn_local({
            let view = Rc::clone(&view);
            let loaders = mem::take(&mut loaders);
            ScopedFuture::new(async move {
                let triggers = join_all(loaders).await;
                for trigger in triggers {
                    trigger.notify();
                }
                matched_view.rebuild(&mut *view.borrow_mut());
            })
        });

        NestedRouteViewState {
            path,
            current_url,
            outlets,
            view,
            outer_owner,
            abort_navigation: Default::default(),
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        let url_snapshot = self.current_url.get_untracked();

        // if the path is the same, we do not need to re-route
        // we can just update the search query and go about our day
        if url_snapshot.path() == state.path {
            for outlet in &state.outlets {
                outlet.url.set(url_snapshot.to_owned());
            }
            return;
        }
        // since the path didn't match, we'll update the retained path for future diffing
        state.path.clear();
        state.path.push_str(url_snapshot.path());

        let new_match = self.routes.match_route(url_snapshot.path());

        state.current_url.set(url_snapshot);

        match new_match {
            None => {
                EitherOf3::<(), Fal, AnyView>::B((self.fallback)())
                    .rebuild(&mut state.view.borrow_mut());
                state.outlets.clear();
                if let Some(loc) = self.location {
                    loc.ready_to_complete();
                }
            }
            Some(route) => {
                if let Some(set_is_routing) = self.set_is_routing {
                    set_is_routing.set(true);
                }

                let mut preloaders = Vec::new();
                let mut full_loaders = Vec::new();
                let different_level = route.rebuild_nested_route(
                    &self.current_url.read_untracked(),
                    self.base,
                    &mut 0,
                    &mut preloaders,
                    &mut full_loaders,
                    &mut state.outlets,
                    self.set_is_routing.is_some(),
                    0,
                );

                let (abort_handle, abort_registration) =
                    AbortHandle::new_pair();

                if let Some(prev_handle) =
                    state.abort_navigation.write_value().replace(abort_handle)
                {
                    prev_handle.abort();
                }

                let location = self.location.clone();
                let is_back = location
                    .as_ref()
                    .map(|nav| nav.is_back().get_untracked())
                    .unwrap_or(false);
                Executor::spawn_local(async move {
                    let triggers = Abortable::new(
                        join_all(preloaders),
                        abort_registration,
                    );
                    if let Ok(triggers) = triggers.await {
                        // tell each one of the outlet triggers that it's ready
                        let notify = move || {
                            for trigger in triggers {
                                trigger.notify();
                            }
                        };
                        if self.transition {
                            start_view_transition(
                                different_level,
                                is_back,
                                notify,
                            );
                        } else {
                            notify();
                        }
                    }
                });

                let abort_navigation = state.abort_navigation.clone();
                Executor::spawn_local(async move {
                    join_all(full_loaders).await;
                    _ = abort_navigation.write_value().take();
                    if let Some(set_is_routing) = self.set_is_routing {
                        set_is_routing.set(false);
                    }
                    if let Some(loc) = location {
                        loc.ready_to_complete();
                    }
                });

                // if it was on the fallback, show the view instead
                if matches!(state.view.borrow().state, EitherOf3::B(_)) {
                    EitherOf3::<(), Fal, AnyView>::C(top_level_outlet(
                        &state.outlets,
                        &self.outer_owner,
                    ))
                    .rebuild(&mut *state.view.borrow_mut());
                }
            }
        }
    }
}

impl<Loc, Defs, Fal, FalFn> AddAnyAttr for NestedRoutesView<Loc, Defs, FalFn>
where
    Loc: LocationProvider + Send,
    Defs: MatchNestedRoutes + Send + 'static,
    FalFn: FnOnce() -> Fal + Send + 'static,
    Fal: RenderHtml + 'static,
{
    type Output<SomeNewAttr: leptos::attr::Attribute> =
        NestedRoutesView<Loc, Defs, FalFn>;

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

impl<Loc, Defs, FalFn, Fal> RenderHtml for NestedRoutesView<Loc, Defs, FalFn>
where
    Loc: LocationProvider + Send,
    Defs: MatchNestedRoutes + Send + 'static,
    FalFn: FnOnce() -> Fal + Send + 'static,
    Fal: RenderHtml + 'static,
{
    type AsyncOutput = Self;
    type Owned = Self;

    const MIN_LENGTH: usize = 0; // TODO

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
            let NestedRoutesView {
                routes,
                outer_owner,
                current_url,
                fallback,
                base,
                ..
            } = self;
            let current_url = current_url.read_untracked();

            let mut outlets = Vec::new();
            let new_match = routes.match_route(current_url.path());
            let view = match new_match {
                None => Either::Left(fallback()),
                Some(route) => {
                    let mut loaders = Vec::new();
                    route.build_nested_route(
                        &current_url,
                        base,
                        &mut loaders,
                        &mut outlets,
                    );

                    // outlets will not send their views if the loaders are never polled
                    // the loaders are async so that they can lazy-load routes in the browser,
                    // but they should always be synchronously available on the server
                    join_all(mem::take(&mut loaders))
                        .now_or_never()
                        .expect("async routes not supported in SSR");

                    Either::Right(top_level_outlet(&outlets, &outer_owner))
                }
            };
            view.to_html_with_buf(
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs,
            );
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Vec<AnyAttribute>,
    ) where
        Self: Sized,
    {
        let NestedRoutesView {
            routes,
            outer_owner,
            current_url,
            fallback,
            base,
            ..
        } = self;
        let current_url = current_url.read_untracked();

        let mut outlets = Vec::new();
        let new_match = routes.match_route(current_url.path());
        let view = match new_match {
            None => Either::Left(fallback()),
            Some(route) => {
                let mut loaders = Vec::new();
                route.build_nested_route(
                    &current_url,
                    base,
                    &mut loaders,
                    &mut outlets,
                );

                // outlets will not send their views if the loaders are never polled
                // the loaders are async so that they can lazy-load routes in the browser,
                // but they should always be synchronously available on the server
                join_all(mem::take(&mut loaders))
                    .now_or_never()
                    .expect("async routes not supported in SSR");

                Either::Right(top_level_outlet(&outlets, &outer_owner))
            }
        };
        view.to_html_async_with_buf::<OUT_OF_ORDER>(
            buf,
            position,
            escape,
            mark_branches,
            extra_attrs,
        );
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        let NestedRoutesView {
            routes,
            outer_owner,
            current_url,
            fallback,
            base,
            ..
        } = self;

        let mut loaders = Vec::new();
        let mut outlets = Vec::new();
        let url = current_url.read_untracked();
        let path = url.path().to_string();

        // match the route
        let new_match = routes.match_route(url.path());

        // start with an empty view because we'll be loading routes async
        let view = Rc::new(RefCell::new(
            match new_match {
                None => EitherOf3::B(fallback()),
                Some(route) => {
                    route.build_nested_route(
                        &url,
                        base,
                        &mut loaders,
                        &mut outlets,
                    );
                    drop(url);

                    join_all(mem::take(&mut loaders)).now_or_never().expect(
                        "lazy routes not supported with hydrate_body(); use \
                         hydrate_lazy() instead",
                    );
                    EitherOf3::C(top_level_outlet(&outlets, &outer_owner))
                }
            }
            .hydrate::<FROM_SERVER>(cursor, position),
        ));

        NestedRouteViewState {
            path,
            current_url,
            outlets,
            view,
            outer_owner,
            abort_navigation: Default::default(),
        }
    }

    async fn hydrate_async(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        let NestedRoutesView {
            routes,
            outer_owner,
            current_url,
            fallback,
            base,
            ..
        } = self;

        let mut loaders = Vec::new();
        let mut outlets = Vec::new();
        let url = current_url.read_untracked();
        let path = url.path().to_string();

        // match the route
        let new_match = routes.match_route(url.path());

        // start with an empty view because we'll be loading routes async
        let view = Rc::new(RefCell::new(
            match new_match {
                None => EitherOf3::B(fallback()),
                Some(route) => {
                    route.build_nested_route(
                        &url,
                        base,
                        &mut loaders,
                        &mut outlets,
                    );
                    drop(url);

                    join_all(mem::take(&mut loaders)).await;
                    EitherOf3::C(top_level_outlet(&outlets, &outer_owner))
                }
            }
            .hydrate::<true>(cursor, position),
        ));

        NestedRouteViewState {
            path,
            current_url,
            outlets,
            view,
            outer_owner,
            abort_navigation: Default::default(),
        }
    }

    fn into_owned(self) -> Self::Owned {
        self
    }
}

type OutletViewFn = Box<dyn FnMut(Owner) -> Suspend<AnyView> + Send>;

pub(crate) struct RouteContext {
    id: RouteMatchId,
    trigger: ArcTrigger,
    url: ArcRwSignal<Url>,
    params: ArcRwSignal<ParamsMap>,
    pub matched: ArcRwSignal<String>,
    base: Option<Oco<'static, str>>,
    view_fn: Arc<Mutex<OutletViewFn>>,
    owner: Arc<Mutex<Option<Owner>>>,
    child: ChildRoute,
}

#[derive(Clone)]
pub(crate) struct ChildRoute(Arc<Mutex<Option<RouteContext>>>);

impl Debug for RouteContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouteContext")
            .field("id", &self.id)
            .field("trigger", &self.trigger)
            .field("url", &self.url)
            .field("params", &self.params)
            .field("matched", &self.matched)
            .field("base", &self.base)
            .finish_non_exhaustive()
    }
}

impl Clone for RouteContext {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            id: self.id,
            trigger: self.trigger.clone(),
            params: self.params.clone(),
            matched: self.matched.clone(),
            base: self.base.clone(),
            view_fn: Arc::clone(&self.view_fn),
            owner: Arc::clone(&self.owner),
            child: self.child.clone(),
        }
    }
}

trait AddNestedRoute {
    fn build_nested_route(
        self,
        url: &Url,
        base: Option<Oco<'static, str>>,
        loaders: &mut Vec<Pin<Box<dyn Future<Output = ArcTrigger>>>>,
        outlets: &mut Vec<RouteContext>,
    );

    #[allow(clippy::too_many_arguments)]
    fn rebuild_nested_route(
        self,
        url: &Url,
        base: Option<Oco<'static, str>>,
        items: &mut usize,
        loaders: &mut Vec<Pin<Box<dyn Future<Output = ArcTrigger>>>>,
        full_loaders: &mut Vec<oneshot::Receiver<Option<Owner>>>,
        outlets: &mut Vec<RouteContext>,
        set_is_routing: bool,
        level: u8,
    ) -> u8;
}

impl<Match> AddNestedRoute for Match
where
    Match: MatchInterface + MatchParams,
{
    fn build_nested_route(
        self,
        url: &Url,
        base: Option<Oco<'static, str>>,
        loaders: &mut Vec<Pin<Box<dyn Future<Output = ArcTrigger>>>>,
        outlets: &mut Vec<RouteContext>,
    ) {
        let orig_url = url;

        // the params signal can be updated to allow the same outlet to update to changes in the
        // params, even if there's not a route match change
        let params = ArcRwSignal::new(self.to_params().into_iter().collect());

        // the URL signal is used for access to things like search query
        // this is provided per nested route, specifically so that navigating *away* from a route
        // does not continuing updating its URL signal, which could do things like triggering
        // resources to run again
        let url = ArcRwSignal::new(url.to_owned());

        // the matched signal will also be updated on every match
        // it's used for relative route resolution
        let matched = ArcRwSignal::new(self.as_matched().to_string());
        let (parent_params, parent_matches): (Vec<_>, Vec<_>) = outlets
            .iter()
            .map(|route| (route.params.clone(), route.matched.clone()))
            .unzip();
        let params_including_parents = {
            let params = params.clone();
            ArcMemo::new({
                move |_| {
                    parent_params
                        .iter()
                        .flat_map(|params| params.get().into_iter())
                        .chain(params.get())
                        .collect::<ParamsMap>()
                }
            })
        };
        let matched_including_parents = {
            let matched = matched.clone();
            ArcMemo::new({
                move |_| {
                    parent_matches
                        .iter()
                        .map(|matched| matched.get())
                        .chain(iter::once(matched.get()))
                        .collect::<String>()
                }
            })
        };

        // the trigger and channel will be used to send new boxed AnyViews to the Outlet;
        // whenever we match a different route, the trigger will be triggered and a new view will
        // be sent through the channel to be rendered by the Outlet
        //
        // combining a trigger and a channel allows us to pass ownership of the view;
        // storing a view in a signal would mean we need to keep a copy stored in the signal and
        // require that we can clone it out
        let trigger = ArcTrigger::new();

        // add this outlet to the end of the outlet stack used for diffing
        let outlet = RouteContext {
            id: self.as_id(),
            url,
            trigger: trigger.clone(),
            params,
            matched,
            view_fn: Arc::new(Mutex::new(Box::new(|_owner| {
                Suspend::new(Box::pin(async { ().into_any() }))
            }))),
            base: base.clone(),
            child: ChildRoute(Arc::new(Mutex::new(None))),
            owner: Arc::new(Mutex::new(None)),
        };
        if !outlets.is_empty() {
            let prev_index = outlets.len().saturating_sub(1);
            *outlets[prev_index].child.0.lock().or_poisoned() =
                Some(outlet.clone());
        }
        outlets.push(outlet.clone());

        // send the initial view through the channel, and recurse through the children
        let (view, child) = self.into_view_and_child();

        loaders.push(Box::pin(ScopedFuture::new({
            let url = outlet.url.clone();
            let matched = Matched(matched_including_parents);
            let view_fn = Arc::clone(&outlet.view_fn);
            let route_owner = Arc::clone(&outlet.owner);
            let outlet = outlet.clone();
            let params = params_including_parents.clone();
            let url = url.clone();
            let matched = matched.clone();
            async move {
                provide_context(params.clone());
                provide_context(url.clone());
                provide_context(matched.clone());
                view.preload().await;
                let child = outlet.child.clone();
                *view_fn.lock().or_poisoned() =
                    Box::new(move |owner_where_used| {
                        *route_owner.lock().or_poisoned() =
                            Some(owner_where_used.clone());
                        let view = view.clone();
                        let child = child.clone();
                        let params = params.clone();
                        let url = url.clone();
                        let matched = matched.clone();
                        owner_where_used.with({
                            let matched = matched.clone();
                            || {
                                let child = child.clone();
                                Suspend::new(Box::pin(async move {
                                    provide_context(child.clone());
                                    provide_context(params.clone());
                                    provide_context(url.clone());
                                    provide_context(matched.clone());
                                    let view = SendWrapper::new(
                                        ScopedFuture::new(view.choose()),
                                    );
                                    let view = view.await;
                                    let view = MatchedRoute(
                                        matched.0.get_untracked(),
                                        view,
                                    );

                                    OwnedView::new(view).into_any()
                                })
                                    as Pin<
                                        Box<
                                            dyn Future<Output = AnyView> + Send,
                                        >,
                                    >)
                            }
                        })
                    });
                trigger
            }
        })));

        // recursively continue building the tree
        // this is important because to build the view, we need access to the outlet
        // and the outlet will be returned from building this child
        if let Some(child) = child {
            child.build_nested_route(orig_url, base, loaders, outlets);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn rebuild_nested_route(
        self,
        url: &Url,
        base: Option<Oco<'static, str>>,
        items: &mut usize,
        preloaders: &mut Vec<Pin<Box<dyn Future<Output = ArcTrigger>>>>,
        full_loaders: &mut Vec<oneshot::Receiver<Option<Owner>>>,
        outlets: &mut Vec<RouteContext>,
        set_is_routing: bool,
        level: u8,
    ) -> u8 {
        let (parent_params, parent_matches): (Vec<_>, Vec<_>) = outlets
            .iter()
            .take(*items)
            .map(|route| (route.params.clone(), route.matched.clone()))
            .unzip();

        if outlets.get(*items).is_some() && *items > 0 {
            *outlets[*items - 1].child.0.lock().or_poisoned() =
                Some(outlets[*items].clone());
        }

        let current = outlets.get_mut(*items);
        match current {
            // if there's nothing currently in the routes at this point, build from here
            None => {
                self.build_nested_route(url, base, preloaders, outlets);
                level
            }
            Some(current) => {
                // a unique ID for each route, which allows us to compare when we get new matches
                // if two IDs are the same, we do not rerender, but only update the params
                // if the IDs are different, we need to replace the remainder of the tree
                let id = self.as_id();

                // build new params and matched strings
                let new_params =
                    self.to_params().into_iter().collect::<ParamsMap>();
                let new_match = self.as_matched().to_owned();

                let (view, child) = self.into_view_and_child();

                // if the IDs don't match, everything below in the tree needs to be swapped:
                // 1) replace this outlet with the next view, with a new owner and new signals for
                //    URL/params
                // 2) remove other outlets that are lower down in the match tree
                // 3) build the rest of the list of matched routes, rather than rebuilding,
                //    as all lower outlets needs to be replaced
                if id != current.id {
                    // update the ID of the match at this depth, so that futures rebuilds diff
                    // against the new ID, not the original one
                    current.id = id;

                    // create new URL and params signals
                    let old_url = mem::replace(
                        &mut current.url,
                        ArcRwSignal::new(url.to_owned()),
                    );
                    let old_params = mem::replace(
                        &mut current.params,
                        ArcRwSignal::new(new_params),
                    );
                    let old_matched = mem::replace(
                        &mut current.matched,
                        ArcRwSignal::new(new_match),
                    );
                    let matched_including_parents = {
                        ArcMemo::new({
                            let matched = current.matched.clone();
                            move |_| {
                                parent_matches
                                    .iter()
                                    .map(|matched| matched.get())
                                    .chain(iter::once(matched.get()))
                                    .collect::<String>()
                            }
                        })
                    };
                    let params_including_parents = {
                        let params = current.params.clone();
                        ArcMemo::new({
                            move |_| {
                                parent_params
                                    .iter()
                                    .flat_map(|params| params.get().into_iter())
                                    .chain(params.get())
                                    .collect::<ParamsMap>()
                            }
                        })
                    };

                    let (full_tx, full_rx) = oneshot::channel();
                    let full_tx = Mutex::new(Some(full_tx));
                    full_loaders.push(full_rx);
                    let outlet = current.clone();

                    // send the new view, with the new owner, through the channel to the Outlet,
                    // and notify the trigger so that the reactive view inside the Outlet tracking
                    // the trigger runs again
                    preloaders.push(Box::pin(ScopedFuture::new({
                        let trigger = current.trigger.clone();
                        let url = current.url.clone();
                        let matched = Matched(matched_including_parents);
                        let view_fn = Arc::clone(&current.view_fn);
                        let route_owner = Arc::clone(&current.owner);
                        let child = outlet.child.clone();
                        async move {
                            let child = child.clone();
                            if set_is_routing {
                                AsyncTransition::run(|| view.preload()).await;
                            } else {
                                view.preload().await;
                            }
                            *view_fn.lock().or_poisoned() =
                                Box::new(move |owner_where_used| {
                                    let prev_owner = route_owner
                                        .lock()
                                        .or_poisoned()
                                        .replace(owner_where_used.clone());
                                    let view = view.clone();
                                    let full_tx =
                                        full_tx.lock().or_poisoned().take();
                                    let child = child.clone();
                                    let params =
                                        params_including_parents.clone();
                                    let url = url.clone();
                                    let matched = matched.clone();
                                    Suspend::new(Box::pin(async move {
                                        let view = SendWrapper::new(
                                            owner_where_used.with(|| {
                                                provide_context(child.clone());
                                                provide_context(params);
                                                provide_context(url);
                                                provide_context(matched);
                                                ScopedFuture::new(async move {
                                                    if set_is_routing {
                                                        AsyncTransition::run(
                                                            || view.choose(),
                                                        )
                                                        .await
                                                    } else {
                                                        view.choose().await
                                                    }
                                                })
                                            }),
                                        );

                                        let view = view.await;

                                        if let Some(tx) = full_tx {
                                            _ = tx.send(prev_owner);
                                        }
                                        owner_where_used.with(|| {
                                            OwnedView::new(view).into_any()
                                        })
                                    }))
                                });

                            drop(old_params);
                            drop(old_url);
                            drop(old_matched);
                            trigger
                        }
                    })));

                    // remove all the items lower in the tree
                    // if this match is different, all its children will also be different
                    outlets.truncate(*items + 1);

                    // if this children has matches, then rebuild the lower section of the tree
                    if let Some(child) = child {
                        child
                            .build_nested_route(url, base, preloaders, outlets);
                    } else {
                        *outlets[*items].child.0.lock().or_poisoned() = None;
                    }

                    return level;
                }

                // otherwise, set the params and URL signals,
                // then just keep rebuilding recursively, checking the remaining routes in the list
                current.matched.set(new_match);
                current.params.set(new_params);
                current.url.set(url.to_owned());
                if let Some(child) = child {
                    *items += 1;
                    child.rebuild_nested_route(
                        url,
                        base,
                        items,
                        preloaders,
                        full_loaders,
                        outlets,
                        set_is_routing,
                        level + 1,
                    )
                } else {
                    *current.child.0.lock().or_poisoned() = None;
                    level
                }
            }
        }
    }
}

impl<Fal> Mountable for NestedRouteViewState<Fal>
where
    Fal: Render,
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

    fn elements(&self) -> Vec<tachys::renderer::types::Element> {
        self.view.elements()
    }
}

fn top_level_outlet(outlets: &[RouteContext], outer_owner: &Owner) -> AnyView {
    let outlet = outlets.first().unwrap();
    let child = outlet.child.clone();
    let view_fn = outlet.view_fn.clone();
    let trigger = outlet.trigger.clone();
    outer_owner.clone().with(|| {
        provide_context(child.clone());
        let outer_owner = outer_owner.clone();
        (move || {
            trigger.track();
            let mut view_fn = view_fn.lock().or_poisoned();
            view_fn(outer_owner.child())
        })
        .into_any()
    })
}

/// Displays the child route nested in a parent route, allowing you to control exactly where
/// that child route is displayed. Renders nothing if there is no nested child.
#[component]
pub fn Outlet() -> impl RenderHtml
where
{
    let ChildRoute(child) = use_context()
        .expect("<Outlet/> used without RouteContext being provided.");
    let child = child.lock().or_poisoned().clone();
    let outer_owner = Owner::current().unwrap();
    child.map(|child| {
        move || {
            child.trigger.track();
            let mut view_fn = child.view_fn.lock().or_poisoned();
            view_fn(outer_owner.child())
        }
    })
}
