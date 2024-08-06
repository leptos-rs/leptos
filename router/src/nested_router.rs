use crate::{
    hooks::Matched,
    location::{LocationProvider, Url},
    matching::Routes,
    params::ParamsMap,
    ChooseView, MatchInterface, MatchNestedRoutes, MatchParams, Method,
    PathSegment, RouteList, RouteListing, RouteMatchId,
};
use any_spawner::Executor;
use either_of::{Either, EitherOf3};
use futures::{future::join_all, FutureExt};
use leptos::{component, oco::Oco};
use or_poisoned::OrPoisoned;
use reactive_graph::{
    computed::ScopedFuture,
    owner::{provide_context, use_context, Owner},
    signal::{ArcRwSignal, ArcTrigger},
    traits::{GetUntracked, ReadUntracked, Set, Track, Trigger},
    wrappers::write::SignalSetter,
};
use send_wrapper::SendWrapper;
use std::{
    cell::RefCell,
    fmt::Debug,
    future::Future,
    iter,
    marker::PhantomData,
    mem,
    pin::Pin,
    rc::Rc,
    sync::{Arc, Mutex},
};
use tachys::{
    hydration::Cursor,
    reactive_graph::{OwnedView, Suspend},
    renderer::Renderer,
    ssr::StreamBuilder,
    view::{
        add_attr::AddAnyAttr,
        any_view::{AnyView, IntoAny},
        either::EitherOf3State,
        Mountable, Position, PositionState, Render, RenderHtml,
    },
};

pub(crate) struct NestedRoutesView<Loc, Defs, FalFn, R> {
    pub location: Option<Loc>,
    pub routes: Routes<Defs, R>,
    pub outer_owner: Owner,
    pub current_url: ArcRwSignal<Url>,
    pub base: Option<Oco<'static, str>>,
    pub fallback: FalFn,
    #[allow(unused)] // TODO
    pub set_is_routing: Option<SignalSetter<bool>>,
    pub rndr: PhantomData<R>,
}

pub struct NestedRouteViewState<Fal, R>
where
    Fal: Render<R>,
    R: Renderer + 'static,
{
    path: String,
    current_url: ArcRwSignal<Url>,
    outlets: Vec<RouteContext<R>>,
    // TODO loading fallback
    #[allow(clippy::type_complexity)]
    view: Rc<RefCell<EitherOf3State<(), Fal, AnyView<R>, R>>>,
}

impl<Loc, Defs, FalFn, Fal, R> Render<R>
    for NestedRoutesView<Loc, Defs, FalFn, R>
where
    Loc: LocationProvider,
    Defs: MatchNestedRoutes<R>,
    FalFn: FnOnce() -> Fal,
    Fal: Render<R> + 'static,
    R: Renderer + 'static,
{
    // TODO support fallback while loading
    type State = NestedRouteViewState<Fal, R>;

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
                    &outer_owner,
                );
                outer_owner.with(|| {
                    EitherOf3::C(
                        Outlet(OutletProps::builder().build()).into_any(),
                    )
                })
            }
        };

        Executor::spawn_local({
            let view = Rc::clone(&view);
            let loaders = mem::take(&mut loaders);
            ScopedFuture::new(async move {
                let triggers = join_all(loaders).await;
                for trigger in triggers {
                    trigger.trigger();
                }
                matched_view.rebuild(&mut *view.borrow_mut());
            })
        });

        NestedRouteViewState {
            path: url.path().to_string(),
            current_url,
            outlets,
            view,
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
                EitherOf3::<(), Fal, AnyView<R>>::B((self.fallback)())
                    .rebuild(&mut state.view.borrow_mut());
                state.outlets.clear();
            }
            Some(route) => {
                let mut loaders = Vec::new();
                route.rebuild_nested_route(
                    &self.current_url.read_untracked(),
                    self.base,
                    &mut 0,
                    &mut loaders,
                    &mut state.outlets,
                    &self.outer_owner,
                );

                let location = self.location.clone();
                Executor::spawn_local(async move {
                    let triggers = join_all(loaders).await;
                    // tell each one of the outlet triggers that it's ready
                    for trigger in triggers {
                        trigger.trigger();
                    }
                    if let Some(loc) = location {
                        loc.ready_to_complete();
                    }
                });

                // if it was on the fallback, show the view instead
                if matches!(state.view.borrow().state, EitherOf3::B(_)) {
                    self.outer_owner.with(|| {
                        EitherOf3::<(), Fal, AnyView<R>>::C(
                            Outlet(OutletProps::builder().build()).into_any(),
                        )
                        .rebuild(&mut *state.view.borrow_mut());
                    })
                }
            }
        }

        if let Some(outlet) = state.outlets.first() {
            self.outer_owner.with(|| outlet.provide_contexts());
        }
    }
}

impl<Loc, Defs, Fal, FalFn, R> AddAnyAttr<R>
    for NestedRoutesView<Loc, Defs, FalFn, R>
where
    Loc: LocationProvider + Send,
    Defs: MatchNestedRoutes<R> + Send,
    FalFn: FnOnce() -> Fal + Send,
    Fal: RenderHtml<R> + 'static,
    R: Renderer + 'static,
{
    type Output<SomeNewAttr: leptos::attr::Attribute<R>> =
        NestedRoutesView<Loc, Defs, FalFn, R>;

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

impl<Loc, Defs, FalFn, Fal, R> RenderHtml<R>
    for NestedRoutesView<Loc, Defs, FalFn, R>
where
    Loc: LocationProvider + Send,
    Defs: MatchNestedRoutes<R> + Send,
    FalFn: FnOnce() -> Fal + Send,
    Fal: RenderHtml<R> + 'static,
    R: Renderer + 'static,
{
    type AsyncOutput = Self;

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
                        &outer_owner,
                    );

                    // outlets will not send their views if the loaders are never polled
                    // the loaders are async so that they can lazy-load routes in the browser,
                    // but they should always be synchronously available on the server
                    join_all(mem::take(&mut loaders))
                        .now_or_never()
                        .expect("async routes not supported in SSR");

                    outer_owner.with(|| {
                        Either::Right(
                            Outlet(OutletProps::builder().build()).into_any(),
                        )
                    })
                }
            };
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
                    &outer_owner,
                );

                // outlets will not send their views if the loaders are never polled
                // the loaders are async so that they can lazy-load routes in the browser,
                // but they should always be synchronously available on the server
                join_all(mem::take(&mut loaders))
                    .now_or_never()
                    .expect("async routes not supported in SSR");

                outer_owner.with(|| {
                    Either::Right(
                        Outlet(OutletProps::builder().build()).into_any(),
                    )
                })
            }
        };
        view.to_html_async_with_buf::<OUT_OF_ORDER>(
            buf,
            position,
            escape,
            mark_branches,
        );
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
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
                        &outer_owner,
                    );
                    // TODO support for lazy hydration
                    join_all(mem::take(&mut loaders))
                        .now_or_never()
                        .expect("async routes not supported in SSR");
                    outer_owner.with(|| {
                        EitherOf3::C(
                            Outlet(OutletProps::builder().build()).into_any(),
                        )
                    })
                }
            }
            .hydrate::<FROM_SERVER>(cursor, position),
        ));

        NestedRouteViewState {
            path: url.path().to_string(),
            current_url,
            outlets,
            view,
        }
    }
}

type OutletViewFn<R> = Box<
    dyn Fn() -> Suspend<Pin<Box<dyn Future<Output = AnyView<R>> + Send>>>
        + Send,
>;

pub(crate) struct RouteContext<R>
where
    R: Renderer,
{
    id: RouteMatchId,
    trigger: ArcTrigger,
    url: ArcRwSignal<Url>,
    params: ArcRwSignal<ParamsMap>,
    owner: Owner,
    pub matched: ArcRwSignal<String>,
    base: Option<Oco<'static, str>>,
    view_fn: Arc<Mutex<OutletViewFn<R>>>,
}

impl<R: Renderer> Debug for RouteContext<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouteContext")
            .field("id", &self.id)
            .field("trigger", &self.trigger)
            .field("url", &self.url)
            .field("params", &self.params)
            .field("owner", &self.owner.debug_id())
            .field("matched", &self.matched)
            .field("base", &self.base)
            .finish_non_exhaustive()
    }
}

impl<R> RouteContext<R>
where
    R: Renderer + 'static,
{
    fn provide_contexts(&self) {
        provide_context(self.clone());
    }
}

impl<R> Clone for RouteContext<R>
where
    R: Renderer,
{
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            id: self.id,
            trigger: self.trigger.clone(),
            params: self.params.clone(),
            owner: self.owner.clone(),
            matched: self.matched.clone(),
            base: self.base.clone(),
            view_fn: Arc::clone(&self.view_fn),
        }
    }
}

trait AddNestedRoute<R>
where
    R: Renderer,
{
    fn build_nested_route(
        self,
        url: &Url,
        base: Option<Oco<'static, str>>,
        loaders: &mut Vec<Pin<Box<dyn Future<Output = ArcTrigger>>>>,
        outlets: &mut Vec<RouteContext<R>>,
        parent: &Owner,
    );

    fn rebuild_nested_route(
        self,
        url: &Url,
        base: Option<Oco<'static, str>>,
        items: &mut usize,
        loaders: &mut Vec<Pin<Box<dyn Future<Output = ArcTrigger>>>>,
        outlets: &mut Vec<RouteContext<R>>,
        parent: &Owner,
    );
}

impl<Match, R> AddNestedRoute<R> for Match
where
    Match: MatchInterface<R> + MatchParams,
    R: Renderer + 'static,
{
    fn build_nested_route(
        self,
        url: &Url,
        base: Option<Oco<'static, str>>,
        loaders: &mut Vec<Pin<Box<dyn Future<Output = ArcTrigger>>>>,
        outlets: &mut Vec<RouteContext<R>>,
        parent: &Owner,
    ) {
        let orig_url = url;

        // each Outlet gets its own owner, so it can inherit context from its parent route,
        // a new owner will be constructed if a different route replaces this one in the outlet,
        // so that any signals it creates or context it provides will be cleaned up
        let owner = parent.child();

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
            owner: owner.clone(),
            matched,
            view_fn: Arc::new(Mutex::new(Box::new(|| {
                Suspend::new(Box::pin(async { ().into_any() }))
            }))),
            base: base.clone(),
        };
        outlets.push(outlet.clone());

        // send the initial view through the channel, and recurse through the children
        let (view, child) = self.into_view_and_child();

        loaders.push(Box::pin(owner.with(|| {
            ScopedFuture::new({
                let owner = outlet.owner.clone();
                let params = outlet.params.clone();
                let url = outlet.url.clone();
                let matched = Matched(outlet.matched.clone());
                let view_fn = Arc::clone(&outlet.view_fn);
                async move {
                    provide_context(params);
                    provide_context(url);
                    provide_context(matched);
                    view.preload().await;
                    *view_fn.lock().or_poisoned() = Box::new(move || {
                        let view = view.clone();
                        owner.with(|| {
                            Suspend::new(Box::pin(async move {
                                let view = SendWrapper::new(ScopedFuture::new(
                                    view.choose(),
                                ));
                                let view = view.await;
                                OwnedView::new(view).into_any()
                            })
                                as Pin<
                                    Box<dyn Future<Output = AnyView<R>> + Send>,
                                >)
                        })
                    });
                    trigger
                }
            })
        })));

        // and share the outlet with the parent via context
        // we share it with the *parent* because the <Outlet/> is rendered in or below the parent
        // wherever it appears, <Outlet/> will look for the closest RouteContext
        parent.with(|| outlet.provide_contexts());

        // recursively continue building the tree
        // this is important because to build the view, we need access to the outlet
        // and the outlet will be returned from building this child
        if let Some(child) = child {
            child.build_nested_route(orig_url, base, loaders, outlets, &owner);
        }
    }

    fn rebuild_nested_route(
        self,
        url: &Url,
        base: Option<Oco<'static, str>>,
        items: &mut usize,
        loaders: &mut Vec<Pin<Box<dyn Future<Output = ArcTrigger>>>>,
        outlets: &mut Vec<RouteContext<R>>,
        parent: &Owner,
    ) {
        let current = outlets.get_mut(*items);
        match current {
            // if there's nothing currently in the routes at this point, build from here
            None => {
                self.build_nested_route(url, base, loaders, outlets, parent);
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

                    // assign a new owner, so that contexts and signals owned by the previous route
                    // in this outlet can be dropped
                    let old_owner =
                        mem::replace(&mut current.owner, parent.child());
                    let owner = current.owner.clone();

                    // send the new view, with the new owner, through the channel to the Outlet,
                    // and notify the trigger so that the reactive view inside the Outlet tracking
                    // the trigger runs again
                    loaders.push(Box::pin(owner.with(|| {
                        ScopedFuture::new({
                            let owner = owner.clone();
                            let trigger = current.trigger.clone();
                            let url = current.url.clone();
                            let params = current.params.clone();
                            let matched = Matched(current.matched.clone());
                            let view_fn = Arc::clone(&current.view_fn);
                            async move {
                                provide_context(params);
                                provide_context(url);
                                provide_context(matched);
                                view.preload().await;
                                *view_fn.lock().or_poisoned() =
                                    Box::new(move || {
                                        let owner = owner.clone();
                                        let view = view.clone();
                                        Suspend::new(Box::pin(async move {
                                            let view = SendWrapper::new(
                                                owner.with(|| {
                                                    ScopedFuture::new(
                                                        view.choose(),
                                                    )
                                                }),
                                            );
                                            let view = view.await;
                                            owner.with(|| {
                                                OwnedView::new(view).into_any()
                                            })
                                        }))
                                    });
                                drop(old_owner);
                                drop(old_params);
                                drop(old_url);
                                drop(old_matched);
                                trigger
                            }
                        })
                    })));

                    // remove all the items lower in the tree
                    // if this match is different, all its children will also be different
                    outlets.truncate(*items + 1);

                    // if this children has matches, then rebuild the lower section of the tree
                    if let Some(child) = child {
                        let mut new_outlets = Vec::new();
                        child.build_nested_route(
                            url,
                            base,
                            loaders,
                            &mut new_outlets,
                            &owner,
                        );
                        outlets.extend(new_outlets);
                    }

                    return;
                }

                // otherwise, set the params and URL signals,
                // then just keep rebuilding recursively, checking the remaining routes in the list
                current.matched.set(new_match);
                current.params.set(new_params);
                current.url.set(url.to_owned());
                if let Some(child) = child {
                    let owner = current.owner.clone();
                    *items += 1;
                    child.rebuild_nested_route(
                        url, base, items, loaders, outlets, &owner,
                    );
                }
            }
        }
    }
}

impl<Fal, R> Mountable<R> for NestedRouteViewState<Fal, R>
where
    Fal: Render<R>,
    R: Renderer,
{
    fn unmount(&mut self) {
        self.view.unmount();
    }

    fn mount(&mut self, parent: &R::Element, marker: Option<&R::Node>) {
        self.view.mount(parent, marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable<R>) -> bool {
        self.view.insert_before_this(child)
    }
}

#[component]
pub fn Outlet<R>(#[prop(optional)] rndr: PhantomData<R>) -> impl RenderHtml<R>
where
    R: Renderer + 'static,
{
    _ = rndr;
    move || {
        let ctx = use_context::<RouteContext<R>>()
            .expect("<Outlet/> used without RouteContext being provided.");
        let RouteContext {
            trigger, view_fn, ..
        } = ctx;
        trigger.track();
        let view_fn = view_fn.lock().or_poisoned();
        view_fn()
    }
}
