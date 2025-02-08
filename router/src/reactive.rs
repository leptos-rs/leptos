use crate::{
    location::Location,
    matching::Params,
    route::MatchedRoute,
    router::{FallbackOrView, Router},
    static_render::StaticDataMap,
    PathSegment, RouteList, RouteListing, SsrMode,
};
use reactive_graph::{
    memo::Memo,
    signal::ArcRwSignal,
    traits::{SignalGet, SignalSet, SignalWith, Track},
    untrack, Owner,
};
use std::{marker::PhantomData, mem};
use tachydom::{
    hydration::Cursor,
    log,
    renderer::Renderer,
    ssr::StreamBuilder,
    view::{Mountable, Position, PositionState, Render, RenderHtml},
};

#[allow(non_snake_case)]
pub fn ReactiveRouter<Rndr, Loc, DefFn, Defs, FallbackFn, Fallback>(
    mut location: Loc,
    routes: DefFn,
    fallback: FallbackFn,
) -> impl RenderHtml
where
    DefFn: Fn() -> Defs + 'static,
    Defs: 'static,
    Loc: Location + Clone + 'static,
    Rndr: Renderer + 'static,
    Rndr::Element: Clone,
    Rndr::Node: Clone,
    FallbackFn: Fn() -> Fallback + Clone + 'static,
    Fallback: Render + 'static,
    Router<Rndr, Loc, Defs, FallbackFn>: FallbackOrView,
    <Router<Rndr, Loc, Defs, FallbackFn> as FallbackOrView>::Output: RenderHtml,
{
    // create a reactive URL signal that will drive the router view
    let url = ArcRwSignal::new(location.try_to_url().unwrap_or_default());

    // initialize the location service with a router hook that will update
    // this URL signal
    location.set_navigation_hook({
        let url = url.clone();
        move |new_url| {
            tachydom::log(&format!("setting url to {new_url:?}"));
            url.set(new_url)
        }
    });
    location.init();

    // return a reactive router that will update if and only if the URL signal changes
    let owner = Owner::current().unwrap();
    move || {
        url.track();
        ReactiveRouterInner {
            owner: owner.clone(),
            inner: Router::new(location.clone(), routes(), fallback.clone()),
            fal: PhantomData,
        }
    }
}

struct ReactiveRouterInner<Rndr, Loc, Defs, FallbackFn, Fallback>
where
    Rndr: Renderer,
{
    owner: Owner,
    inner: Router<Rndr, Loc, Defs, FallbackFn>,
    fal: PhantomData<Fallback>,
}

impl<Rndr, Loc, Defs, FallbackFn, Fallback> Render
    for ReactiveRouterInner<Rndr, Loc, Defs, FallbackFn, Fallback>
where
    Loc: Location,
    Rndr: Renderer,
    Rndr::Element: Clone,
    Rndr::Node: Clone,
    FallbackFn: Fn() -> Fallback,
    Fallback: Render,
    Router<Rndr, Loc, Defs, FallbackFn>: FallbackOrView,
    <Router<Rndr, Loc, Defs, FallbackFn> as FallbackOrView>::Output: Render,
{
    type State =
        ReactiveRouterInnerState<Rndr, Loc, Defs, FallbackFn, Fallback>;

    fn build(self) -> Self::State {
        let (prev_id, inner) = self.inner.fallback_or_view();
        let owner = self.owner.with(Owner::new);
        ReactiveRouterInnerState {
            inner: owner.with(|| inner.build()),
            owner,
            prev_id,
            fal: PhantomData,
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        let (new_id, view) = self.inner.fallback_or_view();
        if new_id != state.prev_id {
            state.owner = self.owner.with(Owner::new)
            // previous root is dropped here -- TODO check if that's correct or should wait
        };
        state.owner.with(|| view.rebuild(&mut state.inner));
    }
}

impl<Rndr, Loc, Defs, FallbackFn, Fallback> RenderHtml
    for ReactiveRouterInner<Rndr, Loc, Defs, FallbackFn, Fallback>
where
    Loc: Location,
    Rndr: Renderer,
    Rndr::Element: Clone,
    Rndr::Node: Clone,
    FallbackFn: Fn() -> Fallback,
    Fallback: Render,
    Router<Rndr, Loc, Defs, FallbackFn>: FallbackOrView,
    <Router<Rndr, Loc, Defs, FallbackFn> as FallbackOrView>::Output: RenderHtml,
{
    const MIN_LENGTH: usize = <<Router<Rndr, Loc, Defs, FallbackFn> as FallbackOrView>::Output as RenderHtml>::MIN_LENGTH;

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        // if this is being run on the server for the first time, generating all possible routes
        if RouteList::is_generating() {
            let mut routes = RouteList::new();

            // add routes
            self.inner.generate_route_list(&mut routes);

            // add fallback
            routes.push(RouteListing::from_path([PathSegment::Static(
                "".into(),
            )]));

            RouteList::register(routes);
        } else {
            let (id, view) = self.inner.fallback_or_view();
            view.to_html_with_buf(buf, position, escape)
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
        self.inner
            .fallback_or_view()
            .1
            .to_html_async_with_buf::<OUT_OF_ORDER>(buf, position, escape)
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        let (prev_id, inner) = self.inner.fallback_or_view();
        let owner = self.owner.with(Owner::new);
        ReactiveRouterInnerState {
            inner: owner
                .with(|| inner.hydrate::<FROM_SERVER>(cursor, position)),
            owner,
            prev_id,
            fal: PhantomData,
        }
    }
}

struct ReactiveRouterInnerState<Rndr, Loc, Defs, FallbackFn, Fallback>
where
    Router<Rndr, Loc, Defs, FallbackFn>: FallbackOrView,
    <Router<Rndr, Loc, Defs, FallbackFn> as FallbackOrView>::Output: Render,
    Rndr: Renderer,
{
    owner: Owner,
    prev_id: &'static str,
    inner: <<Router<Rndr, Loc, Defs, FallbackFn> as FallbackOrView>::Output as Render>::State,
    fal: PhantomData<Fallback>,
}

impl<Rndr, Loc, Defs, FallbackFn, Fallback> Mountable
    for ReactiveRouterInnerState<Rndr, Loc, Defs, FallbackFn, Fallback>
where
    Router<Rndr, Loc, Defs, FallbackFn>: FallbackOrView,
    <Router<Rndr, Loc, Defs, FallbackFn> as FallbackOrView>::Output: Render,
    Rndr: Renderer,
{
    fn unmount(&mut self) {
        self.inner.unmount();
    }

    fn mount(
        &mut self,
        parent: &leptos::tachys::renderer::types::Element,
        marker: Option<&leptos::tachys::renderer::types::Node>,
    ) {
        self.inner.mount(parent, marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.inner.insert_before_this(child)
    }
}

pub struct ReactiveMatchedRoute {
    pub(crate) search_params: ArcRwSignal<Params<String>>,
    pub(crate) params: ArcRwSignal<Params<&'static str>>,
    pub(crate) matched: ArcRwSignal<String>,
}

impl ReactiveMatchedRoute {
    pub fn param(&self, key: &str) -> Memo<Option<String>> {
        let params = self.params.clone();
        let key = key.to_owned();
        Memo::new(move |_| {
            params.with(|p| {
                p.iter().find(|n| n.0 == key).map(|(_, v)| v.to_string())
            })
        })
    }

    pub fn search(&self, key: &str) -> Memo<Option<String>> {
        let params = self.search_params.clone();
        let key = key.to_owned();
        Memo::new(move |_| {
            params.with(|p| {
                p.iter().find(|n| n.0 == key).map(|(_, v)| v.to_string())
            })
        })
    }
}

pub fn reactive_route<ViewFn, View>(
    view_fn: ViewFn,
) -> impl Fn(MatchedRoute) -> ReactiveRoute<ViewFn, View>
where
    ViewFn: Fn(&ReactiveMatchedRoute) -> View + Clone,
    View: Render,
    Rndr: Renderer,
{
    move |matched| ReactiveRoute {
        view_fn: view_fn.clone(),
        matched,
        ty: PhantomData,
    }
}

pub struct ReactiveRoute<ViewFn, View>
where
    ViewFn: Fn(&ReactiveMatchedRoute) -> View,
    View: Render,
    Rndr: Renderer,
{
    view_fn: ViewFn,
    matched: MatchedRoute,
    ty: PhantomData,
}

impl<ViewFn, View> Render for ReactiveRoute<ViewFn, View>
where
    ViewFn: Fn(&ReactiveMatchedRoute) -> View,
    View: Render,
    Rndr: Renderer,
{
    type State = ReactiveRouteState<View::State>;

    fn build(self) -> Self::State {
        let MatchedRoute {
            search_params,
            params,
            matched,
        } = self.matched;
        let matched = ReactiveMatchedRoute {
            search_params: ArcRwSignal::new(search_params),
            params: ArcRwSignal::new(params),
            matched: ArcRwSignal::new(matched),
        };
        let view_state = untrack(|| (self.view_fn)(&matched).build());
        ReactiveRouteState {
            matched,
            view_state,
        }
    }

    fn rebuild(mut self, state: &mut Self::State) {
        let ReactiveRouteState { matched, .. } = state;
        matched
            .search_params
            .set(mem::take(&mut self.matched.search_params));
        matched.params.set(mem::take(&mut self.matched.params));
        matched.matched.set(mem::take(&mut self.matched.matched));
    }
}

impl<ViewFn, View> RenderHtml for ReactiveRoute<ViewFn, View>
where
    ViewFn: Fn(&ReactiveMatchedRoute) -> View,
    View: RenderHtml,
    Rndr: Renderer,
    Rndr::Node: Clone,
    Rndr::Element: Clone,
{
    const MIN_LENGTH: usize = 0;

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        let MatchedRoute {
            search_params,
            params,
            matched,
        } = self.matched;
        let matched = ReactiveMatchedRoute {
            search_params: ArcRwSignal::new(search_params),
            params: ArcRwSignal::new(params),
            matched: ArcRwSignal::new(matched),
        };
        untrack(|| {
            (self.view_fn)(&matched).to_html_with_buf(buf, position, escape)
        });
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
        let MatchedRoute {
            search_params,
            params,
            matched,
        } = self.matched;
        let matched = ReactiveMatchedRoute {
            search_params: ArcRwSignal::new(search_params),
            params: ArcRwSignal::new(params),
            matched: ArcRwSignal::new(matched),
        };
        untrack(|| {
            (self.view_fn)(&matched)
                .to_html_async_with_buf::<OUT_OF_ORDER>(buf, position, escape)
        });
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        let MatchedRoute {
            search_params,
            params,
            matched,
        } = self.matched;
        let matched = ReactiveMatchedRoute {
            search_params: ArcRwSignal::new(search_params),
            params: ArcRwSignal::new(params),
            matched: ArcRwSignal::new(matched),
        };
        let view_state = untrack(|| {
            (self.view_fn)(&matched).hydrate::<FROM_SERVER>(cursor, position)
        });
        ReactiveRouteState {
            matched,
            view_state,
        }
    }
}

pub struct ReactiveRouteState<State> {
    view_state: State,
    matched: ReactiveMatchedRoute,
}

impl<State> Drop for ReactiveRouteState<State> {
    fn drop(&mut self) {
        log("dropping ReactiveRouteState");
    }
}

impl<T> Mountable for ReactiveRouteState<T>
where
    T: Mountable,
{
    fn unmount(&mut self) {
        self.view_state.unmount();
    }

    fn mount(
        &mut self,
        parent: &<R as Renderer>::Element,
        marker: Option<&<R as Renderer>::Node>,
    ) {
        self.view_state.mount(parent, marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.view_state.insert_before_this(child)
    }
}
