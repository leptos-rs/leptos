use crate::{
    ChooseView, MatchInterface, MatchNestedRoutes, MatchParams, PathSegment,
    RouteList, RouteListing, RouteMatchId,
    flat_router::MatchedRoute,
    hooks::Matched,
    location::{LocationProvider, Url},
    matching::RouteDefs,
    params::ParamsMap,
    view_transition::start_view_transition,
};
use any_spawner::Executor;
use either_of::{Either, EitherOf3};
use futures::{
    FutureExt,
    channel::oneshot,
    future::{
        AbortHandle, AbortRegistration, Abortable, Aborted, Shared, join_all,
    },
    task::AtomicWaker,
};
use leptos::{attr::any_attribute::AnyAttribute, component, oco::Oco};
use or_poisoned::OrPoisoned;
use reactive_graph::{
    computed::{ArcMemo, ScopedFuture},
    graph::{
        AnySource, AnySubscriber, Observer, ReactiveNode, Source, Subscriber,
        WithObserver,
    },
    owner::{Owner, provide_context, use_context},
    signal::{ArcRwSignal, ArcTrigger},
    traits::{Get, GetUntracked, Notify, ReadUntracked, Set, Track, Write},
    transition::AsyncTransition,
    wrappers::write::SignalSetter,
};
use send_wrapper::SendWrapper;
use std::{
    cell::{Cell, RefCell},
    fmt::Debug,
    future::{Future, pending, poll_fn},
    iter, mem,
    pin::Pin,
    rc::Rc,
    sync::{Arc, Mutex, Weak},
    task::Poll,
};
use tachys::{
    hydration::Cursor,
    reactive_graph::{OwnedView, Suspend},
    ssr::StreamBuilder,
    view::{
        Mountable, Position, PositionState, Render, RenderFlags, RenderHtml,
        add_attr::AddAnyAttr,
        any_view::{AnyView, IntoAny},
        either::EitherOf3State,
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
    // incremented on every navigation, so that work started for an earlier
    // one can tell that it has been superseded
    navigation: Rc<Cell<u64>>,
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
                    &outer_owner,
                    false,
                );
                drop(url);

                EitherOf3::C(top_level_outlet(&outlets[0], &outer_owner, None))
            }
        };

        let navigation = Rc::new(Cell::new(0));
        Executor::spawn_local({
            let view = Rc::clone(&view);
            let loaders = mem::take(&mut loaders);
            let navigation = Rc::clone(&navigation);
            ScopedFuture::new(async move {
                // a navigation cancels the preloads of the outlets it
                // replaces or removes; the others still install their views,
                // which that navigation reuses
                let triggers = join_all(loaders).await.into_iter().flatten();
                for trigger in triggers {
                    trigger.notify();
                }
                // a navigation that started in the meantime has rendered the
                // outlets it shows (or the fallback) itself
                if navigation.get() == 0 {
                    matched_view.rebuild(&mut *view.borrow_mut());
                }
            })
        });

        NestedRouteViewState {
            path,
            current_url,
            outlets,
            view,
            outer_owner,
            navigation,
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

        *state.current_url.write_untracked() = url_snapshot;
        let navigation_id = state.navigation.get().wrapping_add(1);
        state.navigation.set(navigation_id);

        match new_match {
            None => {
                EitherOf3::<(), Fal, AnyView>::B((self.fallback)())
                    .rebuild(&mut state.view.borrow_mut());
                for outlet in &state.outlets {
                    outlet.abort_preload();
                }
                state.outlets.clear();
                // the fallback is shown at once; an earlier navigation that is
                // still loading no longer completes (see below)
                if let Some(set_is_routing) = self.set_is_routing {
                    set_is_routing.set(false);
                }
                if let Some(loc) = self.location {
                    loc.ready_to_complete();
                }
            }
            Some(route) => {
                if let Some(set_is_routing) = self.set_is_routing {
                    set_is_routing.set(true);
                }

                let mut preloaders = Vec::new();
                let mut full_loaders: Vec<FullLoader> = Vec::new();
                // a navigation from the fallback (or while the initial load
                // is still pending) that holds the previous page renders the
                // new top-level view only once it has been chosen
                let top_level_shown =
                    matches!(state.view.borrow().state, EitherOf3::C(_));
                let hold_top_level =
                    !top_level_shown && self.set_is_routing.is_some();
                let different_level = route.rebuild_nested_route(
                    &self.current_url.read_untracked(),
                    self.base,
                    &mut 0,
                    &mut preloaders,
                    &mut full_loaders,
                    &mut state.outlets,
                    self.set_is_routing.is_some(),
                    0,
                    &self.outer_owner,
                );

                let top_level = hold_top_level.then(|| {
                    let (shown_tx, shown_rx) = oneshot::channel::<()>();
                    full_loaders.push(Box::pin(async move {
                        _ = shown_rx.await;
                    }));
                    (
                        state.outlets[0].clone(),
                        Rc::clone(&state.view),
                        Rc::clone(&state.navigation),
                        self.outer_owner.clone(),
                        shown_tx,
                    )
                });

                let location = self.location.clone();
                let is_back = location
                    .as_ref()
                    .map(|nav| nav.is_back().get_untracked())
                    .unwrap_or(false);
                Executor::spawn_local(async move {
                    // a later navigation cancels the preloads of the outlets
                    // it replaces or removes; the others still install their
                    // views, which that navigation reuses
                    let triggers = join_all(preloaders)
                        .await
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>();
                    if let Some((
                        outlet,
                        view,
                        navigation,
                        outer_owner,
                        shown,
                    )) = top_level
                    {
                        // a navigation that started since shows something
                        // else: choosing this view would only create it
                        if navigation.get() != navigation_id {
                            return;
                        }
                        let chosen =
                            choose_ahead(&outlet, outer_owner.child()).await;
                        if navigation.get() == navigation_id {
                            EitherOf3::<(), Fal, AnyView>::C(top_level_outlet(
                                &outlet,
                                &outer_owner,
                                Some(chosen),
                            ))
                            .rebuild(&mut *view.borrow_mut());
                            _ = shown.send(());
                        }
                        return;
                    }
                    if !triggers.is_empty() {
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

                let navigation = Rc::clone(&state.navigation);
                Executor::spawn_local(async move {
                    join_all(full_loaders).await;
                    // a later navigation owns is_routing and the location
                    if navigation.get() != navigation_id {
                        return;
                    }
                    if let Some(set_is_routing) = self.set_is_routing {
                        set_is_routing.set(false);
                    }
                    if let Some(loc) = location {
                        loc.ready_to_complete();
                    }
                });

                // if the top-level outlet is not rendered yet (fallback, or
                // the initial load was still pending), show the view instead,
                // unless it is held until the view has been chosen (see above)
                if !top_level_shown && !hold_top_level {
                    EitherOf3::<(), Fal, AnyView>::C(top_level_outlet(
                        &state.outlets[0],
                        &self.outer_owner,
                        None,
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
                        false,
                    );

                    // outlets will not send their views if the loaders are never polled
                    // the loaders are async so that they can lazy-load routes in the browser,
                    // but they should always be synchronously available on the server
                    join_all(mem::take(&mut loaders))
                        .now_or_never()
                        .expect("async routes not supported in SSR");

                    Either::Right(top_level_outlet(
                        &outlets[0],
                        &outer_owner,
                        None,
                    ))
                }
            };
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
                    false,
                );

                let preload_owners = outlets
                    .iter()
                    .map(|o| o.preload_owner.clone())
                    .collect::<Vec<_>>();
                outer_owner
                    .with(|| Owner::on_cleanup(move || drop(preload_owners)));

                // outlets will not send their views if the loaders are never polled
                // the loaders are async so that they can lazy-load routes in the browser,
                // but they should always be synchronously available on the server
                join_all(mem::take(&mut loaders))
                    .now_or_never()
                    .expect("async routes not supported in SSR");

                Either::Right(top_level_outlet(&outlets[0], &outer_owner, None))
            }
        };
        view.to_html_async_with_buf::<OUT_OF_ORDER>(
            buf,
            position,
            flags,
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
                        &outer_owner,
                        false,
                    );
                    drop(url);

                    join_all(mem::take(&mut loaders)).now_or_never().expect(
                        "lazy routes not supported with hydrate_body(); use \
                         hydrate_lazy() instead",
                    );
                    EitherOf3::C(top_level_outlet(
                        &outlets[0],
                        &outer_owner,
                        None,
                    ))
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
            navigation: Default::default(),
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
                        &outer_owner,
                        false,
                    );
                    drop(url);

                    join_all(mem::take(&mut loaders)).await;
                    EitherOf3::C(top_level_outlet(
                        &outlets[0],
                        &outer_owner,
                        None,
                    ))
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
            navigation: Default::default(),
        }
    }

    fn into_owned(self) -> Self::Owned {
        self
    }
}

/// Chooses the view of an outlet; the owner it is given becomes the owner of
/// the view (see `with_owner`).
type OutletViewFn = Box<dyn FnMut(Owner) -> ViewFuture + Send>;

type ViewFuture = Pin<Box<dyn Future<Output = AnyView> + Send>>;

pub(crate) struct RouteContext {
    id: RouteMatchId,
    trigger: ArcTrigger,
    url: ArcRwSignal<Url>,
    params: ArcRwSignal<ParamsMap>,
    pub matched: ArcRwSignal<String>,
    base: Option<Oco<'static, str>>,
    view_fn: Arc<Mutex<OutletViewFn>>,
    // the views scheduled for this outlet, and the one installed in `view_fn`
    views: Arc<Mutex<Views>>,
    owner: Arc<Mutex<Option<Owner>>>,
    preload_owner: Owner,
    child: ChildRoute,
    // cancels the pending preload of the view most recently scheduled for
    // this outlet: a navigation that replaces or removes the outlet aborts
    // it, while one that reuses the outlet leaves it alone
    preload_abort: Arc<Mutex<Option<AbortHandle>>>,
    // the <Outlet/>s of the parent's view that currently render this outlet
    renderers: Arc<Renderers>,
}

/// The views that navigations have scheduled for an outlet, counted in the
/// order they were scheduled: the newest one is the one to show.
#[derive(Default)]
struct Views {
    // how many views have been scheduled
    scheduled: usize,
    // which one of them the outlet's `view_fn` chooses
    installed: usize,
    // resolves once the newest view has been installed, or once its preload
    // has been cancelled
    installing: Option<Shared<oneshot::Receiver<()>>>,
}

/// A view scheduled for an outlet, whose preload reports through this.
struct ScheduledView {
    id: usize,
    installed: oneshot::Sender<()>,
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

impl RouteContext {
    /// Cancels the preload of the view most recently scheduled for this
    /// outlet, if it is still pending.
    fn abort_preload(&self) {
        if let Some(handle) = self.preload_abort.lock().or_poisoned().take() {
            handle.abort();
        }
    }

    /// Schedules a new view for this outlet.
    fn schedule_view(&self) -> ScheduledView {
        let mut views = self.views.lock().or_poisoned();
        views.scheduled += 1;
        let (installed, installing) = oneshot::channel();
        views.installing = Some(installing.shared());
        ScheduledView {
            id: views.scheduled,
            installed,
        }
    }

    /// Records that the preload of the view scheduled as `id` has installed it
    /// in `view_fn`.
    fn view_installed(&self, id: usize, installed: oneshot::Sender<()>) {
        self.views.lock().or_poisoned().installed = id;
        _ = installed.send(());
    }

    /// If the newest view scheduled for this outlet has not been installed
    /// yet, resolves once it has been, or once its preload has been cancelled.
    fn view_installing(&self) -> Option<Shared<oneshot::Receiver<()>>> {
        let views = self.views.lock().or_poisoned();
        if views.installed == views.scheduled {
            None
        } else {
            views.installing.clone()
        }
    }

    /// Registers a new preload for this outlet, cancelling the previous one,
    /// and returns the registration that makes it abortable.
    fn new_preload(&self) -> AbortRegistration {
        let (handle, registration) = AbortHandle::new_pair();
        if let Some(previous) =
            self.preload_abort.lock().or_poisoned().replace(handle)
        {
            previous.abort();
        }
        registration
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
            views: Arc::clone(&self.views),
            owner: Arc::clone(&self.owner),
            child: self.child.clone(),
            preload_owner: self.preload_owner.clone(),
            preload_abort: Arc::clone(&self.preload_abort),
            renderers: Arc::clone(&self.renderers),
        }
    }
}

/// Preloads an outlet's view, then installs it and resolves with the trigger
/// that renders it, or with `Err` if the preload was cancelled (see
/// `RouteContext::preload_abort`).
type Preloader = Pin<Box<dyn Future<Output = Result<ArcTrigger, Aborted>>>>;

/// Resolves once the view a navigation shows in an outlet has been chosen.
type FullLoader = Pin<Box<dyn Future<Output = ()>>>;

/// Counts the `<Outlet/>`s that render an outlet (see
/// `RouteContext::renderers`).
#[derive(Default)]
struct Renderers(Mutex<RenderersInner>);

#[derive(Default)]
struct RenderersInner {
    count: usize,
    // woken when the count drops to zero; each one belongs to a future that
    // waits for it, and is dropped along with it
    waiting: Vec<Weak<AtomicWaker>>,
}

impl Renderers {
    /// Counts an `<Outlet/>` that renders the outlet, until the current owner
    /// is cleaned up: when the `<Outlet/>` renders it again, or is disposed of.
    fn track(this: &Arc<Self>) {
        this.0.lock().or_poisoned().count += 1;
        Owner::on_cleanup({
            let this = Arc::clone(this);
            move || {
                let waiting = {
                    let mut inner = this.0.lock().or_poisoned();
                    inner.count = inner.count.saturating_sub(1);
                    if inner.count == 0 {
                        mem::take(&mut inner.waiting)
                    } else {
                        Vec::new()
                    }
                };
                for waker in waiting.iter().filter_map(Weak::upgrade) {
                    waker.wake();
                }
            }
        });
    }

    /// Whether nothing renders the outlet. If something does, `waker` is
    /// woken once nothing does anymore.
    fn poll_none(&self, waker: &Arc<AtomicWaker>) -> bool {
        let mut inner = self.0.lock().or_poisoned();
        if inner.count == 0 {
            return true;
        }
        // forget the futures that no longer wait
        inner.waiting.retain(|waiting| waiting.strong_count() > 0);
        if !inner
            .waiting
            .iter()
            .any(|waiting| waiting.as_ptr() == Arc::as_ptr(waker))
        {
            inner.waiting.push(Arc::downgrade(waker));
        }
        false
    }
}

/// Resolves once the view scheduled for an outlet has been chosen, or once
/// nothing renders the outlet: a view that is not rendered is never chosen,
/// and is not part of what the navigation shows.
async fn chosen_while_rendered(
    mut chosen: oneshot::Receiver<Option<Owner>>,
    renderers: Arc<Renderers>,
) {
    let waker = Arc::new(AtomicWaker::new());
    poll_fn(|cx| {
        waker.register(cx.waker());
        if chosen.poll_unpin(cx).is_ready() || renderers.poll_none(&waker) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await
}

trait AddNestedRoute {
    /// Builds the outlets for this match and those below it. `navigating` is
    /// set when a navigation that holds the previous page (see
    /// `set_is_routing`) builds them.
    fn build_nested_route(
        self,
        url: &Url,
        base: Option<Oco<'static, str>>,
        loaders: &mut Vec<Preloader>,
        outlets: &mut Vec<RouteContext>,
        outer_owner: &Owner,
        navigating: bool,
    );

    #[allow(clippy::too_many_arguments)]
    fn rebuild_nested_route(
        self,
        url: &Url,
        base: Option<Oco<'static, str>>,
        items: &mut usize,
        loaders: &mut Vec<Preloader>,
        full_loaders: &mut Vec<FullLoader>,
        outlets: &mut Vec<RouteContext>,
        set_is_routing: bool,
        level: u8,
        outer_owner: &Owner,
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
        loaders: &mut Vec<Preloader>,
        outlets: &mut Vec<RouteContext>,
        outer_owner: &Owner,
        navigating: bool,
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
                Box::pin(async { ().into_any() })
            }))),
            views: Default::default(),
            base: base.clone(),
            child: ChildRoute(Arc::new(Mutex::new(None))),
            owner: Arc::new(Mutex::new(None)),
            preload_owner: outer_owner.child(),
            preload_abort: Default::default(),
            renderers: Default::default(),
        };
        let preload_registration = outlet.new_preload();
        let ScheduledView { id, installed } = outlet.schedule_view();
        if !outlets.is_empty() {
            let prev_index = outlets.len().saturating_sub(1);
            *outlets[prev_index].child.0.lock().or_poisoned() =
                Some(outlet.clone());
        }
        outlets.push(outlet.clone());

        // send the initial view through the channel, and recurse through the children
        let (view, child) = self.into_view_and_child();

        let preloader = ScopedFuture::new({
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
                outlet
                    .preload_owner
                    .with(|| {
                        provide_context(params.clone());
                        provide_context(url.clone());
                        provide_context(matched.clone());
                        ScopedFuture::new(async {
                            if navigating {
                                AsyncTransition::run(|| view.preload()).await;
                            } else {
                                view.preload().await;
                            }
                        })
                    })
                    .await;
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
                        Box::pin(with_owner(owner_where_used, async move {
                            provide_context(child);
                            provide_context(params);
                            provide_context(url);
                            provide_context(matched.clone());
                            let view = choose_view(view, navigating).await;
                            let view =
                                MatchedRoute(matched.0.get_untracked(), view);
                            OwnedView::new(view).into_any()
                        }))
                    });
                outlet.view_installed(id, installed);
                trigger
            }
        });
        loaders.push(Box::pin(Abortable::new(preloader, preload_registration)));

        // recursively continue building the tree
        // this is important because to build the view, we need access to the outlet
        // and the outlet will be returned from building this child
        if let Some(child) = child {
            child.build_nested_route(
                orig_url,
                base,
                loaders,
                outlets,
                outer_owner,
                navigating,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn rebuild_nested_route(
        self,
        url: &Url,
        base: Option<Oco<'static, str>>,
        items: &mut usize,
        preloaders: &mut Vec<Preloader>,
        full_loaders: &mut Vec<FullLoader>,
        outlets: &mut Vec<RouteContext>,
        set_is_routing: bool,
        level: u8,
        outer_owner: &Owner,
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
                self.build_nested_route(
                    url,
                    base,
                    preloaders,
                    outlets,
                    outer_owner,
                    set_is_routing,
                );
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
                    let old_preload_owner = mem::replace(
                        &mut current.preload_owner,
                        outer_owner.child(),
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
                    let ScheduledView { id, installed } =
                        current.schedule_view();
                    // the router always renders the top-level outlet, while a
                    // child outlet is only rendered if its parent's view has
                    // an <Outlet/> on screen
                    full_loaders.push(if *items == 0 {
                        Box::pin(async move {
                            _ = full_rx.await;
                        })
                    } else {
                        Box::pin(chosen_while_rendered(
                            full_rx,
                            Arc::clone(&current.renderers),
                        ))
                    });
                    let outlet = current.clone();

                    // send the new view, with the new owner, through the channel to the Outlet,
                    // and notify the trigger so that the reactive view inside the Outlet tracking
                    // the trigger runs again
                    let preload_registration = current.new_preload();
                    let preloader = ScopedFuture::new({
                        let trigger = current.trigger.clone();
                        let url = current.url.clone();
                        let matched = Matched(matched_including_parents);
                        let view_fn = Arc::clone(&current.view_fn);
                        let route_owner = Arc::clone(&current.owner);
                        let child = outlet.child.clone();
                        async move {
                            let child = child.clone();
                            outlet
                                .preload_owner
                                .with(|| {
                                    provide_context(
                                        params_including_parents.clone(),
                                    );
                                    provide_context(url.clone());
                                    provide_context(matched.clone());
                                    ScopedFuture::new(async {
                                        if set_is_routing {
                                            AsyncTransition::run(|| {
                                                view.preload()
                                            })
                                            .await;
                                        } else {
                                            view.preload().await;
                                        }
                                    })
                                })
                                .await;
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
                                    Box::pin(with_owner(
                                        owner_where_used,
                                        async move {
                                            provide_context(child);
                                            provide_context(params);
                                            provide_context(url);
                                            provide_context(matched);
                                            let view = choose_view(
                                                view,
                                                set_is_routing,
                                            )
                                            .await;
                                            if let Some(tx) = full_tx {
                                                _ = tx.send(prev_owner);
                                            }
                                            OwnedView::new(view).into_any()
                                        },
                                    ))
                                });
                            outlet.view_installed(id, installed);

                            drop(old_params);
                            drop(old_url);
                            drop(old_matched);
                            drop(old_preload_owner);
                            trigger
                        }
                    });
                    preloaders.push(Box::pin(Abortable::new(
                        preloader,
                        preload_registration,
                    )));

                    // remove all the items lower in the tree
                    // if this match is different, all its children will also be different
                    for outlet in outlets.iter().skip(*items + 1) {
                        outlet.abort_preload();
                    }
                    outlets.truncate(*items + 1);

                    // if this children has matches, then rebuild the lower section of the tree
                    if let Some(child) = child {
                        child.build_nested_route(
                            url,
                            base,
                            preloaders,
                            outlets,
                            outer_owner,
                            set_is_routing,
                        );
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
                        outer_owner,
                    )
                } else {
                    *current.child.0.lock().or_poisoned() = None;
                    level
                }
            }
        }
    }
}

/// Polls `fut` with `owner` as the current owner, without changing the current
/// observer.
fn with_owner<T>(
    owner: Owner,
    fut: impl Future<Output = T> + Send + 'static,
) -> impl Future<Output = T> + Send + 'static {
    let mut fut = Box::pin(fut);
    poll_fn(move |cx| owner.with(|| fut.as_mut().poll(cx)))
}

/// Chooses `view`. During a navigation that holds the previous page
/// (`navigating`), it chooses it in an async transition, which waits for the
/// resources created while the view is created, and then chooses the views of
/// the child outlets that it renders, so that they are shown along with it.
async fn choose_view(view: impl ChooseView, navigating: bool) -> AnyView {
    if !navigating {
        return SendWrapper::new(ScopedFuture::new(view.choose())).await;
    }
    let children = ChildChoices::open();
    provide_context(children.clone());
    let view =
        SendWrapper::new(ScopedFuture::new(AsyncTransition::run(|| {
            view.choose()
        })))
        .await;
    children.choose().await;
    view
}

/// The child outlets of a view that is being chosen during a navigation that
/// holds the previous page: the `<Outlet/>`s created while it is chosen, i.e.
/// those in the view itself rather than in content it renders later.
#[derive(Clone)]
struct ChildChoices(Arc<Mutex<Option<Vec<ChildChoice>>>>);

struct ChildChoice {
    outlet: RouteContext,
    // the owner of the <Outlet/>, under which the view is created
    owner: Owner,
    slot: ChosenSlot,
}

/// How many rounds [`ChildChoices::choose`] takes to choose the view of an
/// outlet, when later navigations schedule other views for it meanwhile.
const CHILD_CHOICE_ROUNDS: usize = 8;

/// Where the view chosen for an `<Outlet/>` waits for it to render.
type ChosenSlot = Arc<Mutex<Option<Chosen>>>;

impl ChildChoices {
    fn open() -> Self {
        Self(Arc::new(Mutex::new(Some(Vec::new()))))
    }

    /// Registers the outlet that an `<Outlet/>` owned by `owner` renders, if
    /// the view is still being chosen.
    fn register(
        &self,
        outlet: RouteContext,
        owner: Owner,
    ) -> Option<ChosenSlot> {
        let mut children = self.0.lock().or_poisoned();
        let slot = ChosenSlot::default();
        children.as_mut()?.push(ChildChoice {
            outlet,
            owner,
            slot: Arc::clone(&slot),
        });
        Some(slot)
    }

    /// Stops registering outlets, and chooses the views of those registered.
    async fn choose(self) {
        let children = self.0.lock().or_poisoned().take().unwrap_or_default();
        join_all(children.into_iter().map(
            |ChildChoice {
                 outlet,
                 owner,
                 slot,
             }| async move {
                // a later navigation may schedule another view for the
                // outlet while this one is chosen: then that one is chosen,
                // once its preload has installed it
                for _ in 0..CHILD_CHOICE_ROUNDS {
                    if let Some(installing) = outlet.view_installing() {
                        _ = installing.await;
                        continue;
                    }
                    let chosen = choose_ahead(&outlet, owner.child()).await;
                    if chosen.is_current(&outlet) {
                        *slot.lock().or_poisoned() = Some(chosen);
                        break;
                    }
                }
            },
        ))
        .await;
    }
}

/// An outlet's view, chosen ahead of rendering it.
struct Chosen {
    view: AnyView,
    sources: Arc<ReadSources>,
    // the view of the outlet it was chosen from (see `Views`)
    view_id: usize,
}

impl Chosen {
    /// Whether no navigation has scheduled another view for `outlet` since the
    /// one it was chosen from.
    fn is_current(&self, outlet: &RouteContext) -> bool {
        self.view_id == outlet.views.lock().or_poisoned().scheduled
    }

    /// Renders the view, as a `Suspend` that has already resolved would.
    fn into_view(self) -> Suspend<AnyView> {
        self.sources.forward();
        let view = self.view;
        Suspend::new(async move { view })
    }
}

/// Chooses the view of `outlet` in `owner`, ahead of rendering it.
fn choose_ahead(
    outlet: &RouteContext,
    owner: Owner,
) -> impl Future<Output = Chosen> + Send + 'static {
    let sources = Arc::new(ReadSources::default());
    let (view_id, view) =
        ReadSources::subscriber(&sources).with_observer(|| {
            let mut view_fn = outlet.view_fn.lock().or_poisoned();
            let view_id = outlet.views.lock().or_poisoned().installed;
            (view_id, ScopedFuture::new(view_fn(owner)))
        });
    async move {
        Chosen {
            view: view.await,
            sources,
            view_id,
        }
    }
}

/// Collects the reactive sources read while a view is chosen ahead of
/// rendering it, and has what renders it subscribe to them, as `Suspend` does
/// when it chooses a view itself: a route's view is then updated when what it
/// read while it was created changes, however it was chosen.
#[derive(Default)]
struct ReadSources(Mutex<Vec<AnySource>>);

impl ReadSources {
    fn subscriber(this: &Arc<Self>) -> AnySubscriber {
        AnySubscriber(
            Arc::as_ptr(this) as usize,
            Arc::downgrade(this) as Weak<dyn Subscriber + Send + Sync>,
        )
    }

    /// Subscribes the current observer to the sources read.
    fn forward(&self) {
        if let Some(observer) = Observer::get() {
            for source in mem::take(&mut *self.0.lock().or_poisoned()) {
                source.add_subscriber(observer.clone());
                observer.add_source(source);
            }
        }
    }
}

impl ReactiveNode for ReadSources {
    fn mark_dirty(&self) {}

    fn mark_check(&self) {}

    fn mark_subscribers_check(&self) {}

    fn update_if_necessary(&self) -> bool {
        false
    }
}

impl Subscriber for ReadSources {
    fn add_source(&self, source: AnySource) {
        self.0.lock().or_poisoned().push(source);
    }

    fn clear_sources(&self, subscriber: &AnySubscriber) {
        for source in mem::take(&mut *self.0.lock().or_poisoned()) {
            source.remove_subscriber(subscriber);
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

/// The router's view of the top-level outlet. It first shows `chosen`, if its
/// view has already been chosen.
fn top_level_outlet(
    outlet: &RouteContext,
    outer_owner: &Owner,
    chosen: Option<Chosen>,
) -> AnyView {
    let child = outlet.child.clone();
    let view_fn = outlet.view_fn.clone();
    let trigger = outlet.trigger.clone();
    let chosen = Mutex::new(chosen);
    outer_owner.clone().with(|| {
        provide_context(child.clone());
        let outer_owner = outer_owner.clone();
        (move || {
            trigger.track();
            if let Some(chosen) = chosen.lock().or_poisoned().take() {
                return chosen.into_view();
            }
            let mut view_fn = view_fn.lock().or_poisoned();
            Suspend::new(view_fn(outer_owner.child()))
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
        // while the view that renders this <Outlet/> is chosen during a
        // navigation that holds the previous page, the child's view is chosen
        // too, and shown along with it
        let chosen = use_context::<ChildChoices>().and_then(|choices| {
            choices.register(child.clone(), outer_owner.clone())
        });
        move || {
            child.trigger.track();
            Renderers::track(&child.renderers);
            // a view chosen ahead is shown unless a later navigation has
            // scheduled another view for the outlet since
            if let Some(chosen) = chosen
                .as_ref()
                .and_then(|slot| slot.lock().or_poisoned().take())
                .filter(|chosen| chosen.is_current(&child))
            {
                return chosen.into_view();
            }
            // nothing new is shown until the preload of the newest view has
            // installed it: its trigger then renders the outlet again
            if child.view_installing().is_some() {
                return Suspend::new(pending::<AnyView>());
            }
            let mut view_fn = child.view_fn.lock().or_poisoned();
            Suspend::new(view_fn(outer_owner.child()))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::task::{ArcWake, waker};
    use std::{
        pin::pin,
        sync::atomic::{AtomicBool, Ordering},
        task::{Context, Waker},
    };

    #[derive(Default)]
    struct Woken(AtomicBool);

    impl ArcWake for Woken {
        fn wake_by_ref(this: &Arc<Self>) {
            this.0.store(true, Ordering::Relaxed);
        }
    }

    fn new_waker() -> (Arc<Woken>, Waker) {
        let woken = Arc::new(Woken::default());
        let waker = waker(Arc::clone(&woken));
        (woken, waker)
    }

    #[test]
    fn waiting_for_a_rendered_outlet_ends_once_nothing_renders_it() {
        let outlet = Owner::new();
        let renderers = Arc::<Renderers>::default();
        outlet.with(|| Renderers::track(&renderers));
        let (_chosen_tx, chosen) = oneshot::channel::<Option<Owner>>();
        let mut waiting =
            pin!(chosen_while_rendered(chosen, Arc::clone(&renderers)));
        let (woken, waker) = new_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(waiting.as_mut().poll(&mut cx).is_pending());

        // the <Outlet/> is disposed of, and another one renders the outlet
        outlet.cleanup();
        outlet.with(|| Renderers::track(&renderers));
        assert!(waiting.as_mut().poll(&mut cx).is_pending());

        outlet.cleanup();
        assert!(woken.0.load(Ordering::Relaxed));
        assert!(waiting.as_mut().poll(&mut cx).is_ready());
    }

    #[test]
    fn waiting_for_a_rendered_outlet_leaves_no_waker_behind() {
        let outlet = Owner::new();
        let renderers = Arc::<Renderers>::default();
        outlet.with(|| Renderers::track(&renderers));
        // the navigations that replace the outlet's view, each one waiting
        // for it in a task of its own
        for _ in 0..3 {
            let (chosen_tx, chosen) = oneshot::channel::<Option<Owner>>();
            let mut waiting =
                pin!(chosen_while_rendered(chosen, Arc::clone(&renderers)));
            let (_, waker) = new_waker();
            let mut cx = Context::from_waker(&waker);
            assert!(waiting.as_mut().poll(&mut cx).is_pending());
            _ = chosen_tx.send(None);
            assert!(waiting.as_mut().poll(&mut cx).is_ready());
        }
        let (wakers, live) = {
            let inner = renderers.0.lock().or_poisoned();
            let live = inner.waiting.iter().filter(|w| w.strong_count() > 0);
            (inner.waiting.len(), live.count())
        };
        assert!(wakers <= 1, "{wakers} wakers");
        assert_eq!(live, 0);
    }
}
