use crate::{
    generate_route_list::RouteList,
    location::{Location, RequestUrl},
    matching::{
        MatchInterface, MatchNestedRoutes, PossibleRouteMatch, RouteMatchId,
        Routes,
    },
    ChooseView, MatchParams, Method, Params, PathSegment, RouteListing,
    SsrMode,
};
use core::marker::PhantomData;
use either_of::*;
use once_cell::unsync::Lazy;
use reactive_graph::{
    computed::{ArcMemo, Memo},
    effect::RenderEffect,
    owner::{use_context, Owner},
    signal::ArcRwSignal,
    traits::{Get, Read, Set, Track},
};
use std::{
    any::Any,
    borrow::Cow,
    cell::{Cell, RefCell},
    collections::VecDeque,
    fmt::Debug,
    iter,
    rc::Rc,
};
use tachys::{
    html::attribute::Attribute,
    hydration::Cursor,
    renderer::{dom::Dom, Renderer},
    ssr::StreamBuilder,
    view::{
        add_attr::AddAnyAttr,
        any_view::{AnyView, AnyViewState, IntoAny},
        either::EitherState,
        Mountable, Position, PositionState, Render, RenderHtml,
    },
};

#[derive(Debug)]
pub struct Router<Rndr, Loc, Children, FallbackFn> {
    base: Option<Cow<'static, str>>,
    location: PhantomData<Loc>,
    pub routes: Routes<Children, Rndr>,
    fallback: FallbackFn,
}

impl<Rndr, Loc, Children, FallbackFn, Fallback>
    Router<Rndr, Loc, Children, FallbackFn>
where
    Loc: Location,
    Rndr: Renderer,
    FallbackFn: Fn() -> Fallback,
{
    pub fn new(
        routes: Routes<Children, Rndr>,
        fallback: FallbackFn,
    ) -> Router<Rndr, Loc, Children, FallbackFn> {
        Self {
            base: None,
            location: PhantomData,
            routes,
            fallback,
        }
    }

    pub fn new_with_base(
        base: impl Into<Cow<'static, str>>,
        routes: Routes<Children, Rndr>,
        fallback: FallbackFn,
    ) -> Router<Rndr, Loc, Children, FallbackFn> {
        Self {
            base: Some(base.into()),
            location: PhantomData,
            routes,
            fallback,
        }
    }
}

impl<Rndr, Loc, Children, FallbackFn, Fallback>
    Router<Rndr, Loc, Children, FallbackFn>
where
    FallbackFn: Fn() -> Fallback,
    Rndr: Renderer,
{
    pub fn fallback(&self) -> Fallback {
        (self.fallback)()
    }
}

pub struct RouteData<R = Dom>
where
    R: Renderer + 'static,
{
    pub params: ArcMemo<Params>,
    pub outlet: Outlet<R>,
}

impl<Rndr, Loc, FallbackFn, Fallback, Children> Render<Rndr>
    for Router<Rndr, Loc, Children, FallbackFn>
where
    Loc: Location,
    FallbackFn: Fn() -> Fallback + 'static,
    Fallback: Render<Rndr>,
    Children: MatchNestedRoutes<Rndr> + 'static,
    Fallback::State: 'static,
    Rndr: Renderer + 'static,
    Children::Match: std::fmt::Debug,
    <Children::Match as MatchInterface<Rndr>>::Child: std::fmt::Debug,
{
    type State = RenderEffect<
        EitherState<
            <NestedRouteView<Children::Match, Rndr> as Render<Rndr>>::State,
            <Fallback as Render<Rndr>>::State,
            Rndr,
        >,
    >;
    type FallibleState = (); // TODO

    fn build(self) -> Self::State {
        let location = Loc::new().unwrap(); // TODO
        location.init(self.base);
        let url = location.as_url().clone();
        let path = ArcMemo::new({
            let url = url.clone();
            move |_| url.read().path().to_string()
        });
        let search_params = ArcMemo::new({
            let url = url.clone();
            move |_| url.read().search_params().clone()
        });
        let outer_owner =
            Owner::current().expect("creating Router, but no Owner was found");

        RenderEffect::new(move |prev: Option<EitherState<_, _, _>>| {
            let path = path.read();
            let new_match = self.routes.match_route(&path);

            if let Some(mut prev) = prev {
                if let Some(new_match) = new_match {
                    match &mut prev.state {
                        Either::Left(prev) => {
                            rebuild_nested(&outer_owner, prev, new_match);
                        }
                        Either::Right(_) => {
                            Either::<_, Fallback>::Left(NestedRouteView::new(
                                &outer_owner,
                                new_match,
                            ))
                            .rebuild(&mut prev);
                        }
                    }
                } else {
                    Either::<NestedRouteView<Children::Match, Rndr>, _>::Right(
                        (self.fallback)(),
                    )
                    .rebuild(&mut prev);
                }
                prev
            } else {
                match new_match {
                    Some(matched) => Either::Left(NestedRouteView::new(
                        &outer_owner,
                        matched,
                    )),
                    _ => Either::Right((self.fallback)()),
                }
                .build()
            }
        })
    }

    fn rebuild(self, state: &mut Self::State) {}

    fn try_build(self) -> tachys::error::Result<Self::FallibleState> {
        todo!()
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> tachys::error::Result<()> {
        todo!()
    }
}

impl<Rndr, Loc, FallbackFn, Fallback, Children> RenderHtml<Rndr>
    for Router<Rndr, Loc, Children, FallbackFn>
where
    Loc: Location,
    FallbackFn: Fn() -> Fallback + 'static,
    Fallback: RenderHtml<Rndr>,
    Children: MatchNestedRoutes<Rndr> + 'static,
    Children::View: RenderHtml<Rndr>,
    /*View: Render<Rndr> + IntoAny<Rndr> + 'static,
    View::State: 'static,*/
    Fallback: RenderHtml<Rndr>,
    Fallback::State: 'static,
    Rndr: Renderer + 'static,
    Children::Match: std::fmt::Debug,
    <Children::Match as MatchInterface<Rndr>>::Child: std::fmt::Debug,
{
    // TODO probably pick a max length here
    const MIN_LENGTH: usize = Children::View::MIN_LENGTH;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        // if this is being run on the server for the first time, generating all possible routes
        if RouteList::is_generating() {
            // add routes
            let (base, routes) = self.routes.generate_routes();
            let mut routes = routes
                .into_iter()
                .map(|segments| {
                    let path = base
                        .into_iter()
                        .flat_map(|base| {
                            iter::once(PathSegment::Static(
                                base.to_string().into(),
                            ))
                        })
                        .chain(segments)
                        .collect::<Vec<_>>();
                    // TODO add non-defaults for mode, etc.
                    RouteListing::new(
                        path,
                        SsrMode::OutOfOrder,
                        [Method::Get],
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
            let outer_owner = Owner::current()
                .expect("creating Router, but no Owner was found");
            let url = use_context::<RequestUrl>()
                .expect("could not find request URL in context");
            // TODO base
            let url =
                RequestUrl::parse(url.as_ref()).expect("could not parse URL");
            // TODO query params
            let new_match = self.routes.match_route(url.path());
            match new_match {
                Some(matched) => {
                    Either::Left(NestedRouteView::new(&outer_owner, matched))
                }
                _ => Either::Right((self.fallback)()),
            }
            .to_html_with_buf(buf, position)
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        let outer_owner =
            Owner::current().expect("creating Router, but no Owner was found");
        let url = use_context::<RequestUrl>()
            .expect("could not find request URL in context");
        // TODO base
        let url = RequestUrl::parse(url.as_ref()).expect("could not parse URL");
        // TODO query params
        let new_match = self.routes.match_route(url.path());
        match new_match {
            Some(matched) => {
                Either::Left(NestedRouteView::new(&outer_owner, matched))
            }
            _ => Either::Right((self.fallback)()),
        }
        .to_html_async_with_buf::<OUT_OF_ORDER>(buf, position)
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self::State {
        let location = Loc::new().unwrap(); // TODO
        location.init(self.base);
        let url = location.as_url().clone();
        let path = ArcMemo::new({
            let url = url.clone();
            move |_| url.read().path().to_string()
        });
        let search_params = ArcMemo::new({
            let url = url.clone();
            move |_| url.read().search_params().clone()
        });
        let outer_owner =
            Owner::current().expect("creating Router, but no Owner was found");

        let cursor = cursor.clone();
        let position = position.clone();
        RenderEffect::new(move |prev: Option<EitherState<_, _, _>>| {
            let path = path.read();
            let new_match = self.routes.match_route(&path);

            if let Some(mut prev) = prev {
                if let Some(new_match) = new_match {
                    match &mut prev.state {
                        Either::Left(prev) => {
                            rebuild_nested(&outer_owner, prev, new_match);
                        }
                        Either::Right(_) => {
                            Either::<_, Fallback>::Left(NestedRouteView::new(
                                &outer_owner,
                                new_match,
                            ))
                            .rebuild(&mut prev);
                        }
                    }
                } else {
                    Either::<NestedRouteView<Children::Match, Rndr>, _>::Right(
                        (self.fallback)(),
                    )
                    .rebuild(&mut prev);
                }
                prev
            } else {
                match new_match {
                    Some(matched) => {
                        Either::Left(NestedRouteView::new_hydrate(
                            &outer_owner,
                            matched,
                            &cursor,
                            &position,
                        ))
                    }
                    _ => Either::Right((self.fallback)()),
                }
                .hydrate::<true>(&cursor, &position)
            }
        })
    }
}

pub struct NestedRouteView<Matcher, R>
where
    Matcher: MatchInterface<R>,
    R: Renderer + 'static,
{
    id: RouteMatchId,
    owner: Owner,
    params: ArcRwSignal<Params>,
    outlets: VecDeque<Outlet<R>>,
    view: Matcher::View,
    ty: PhantomData<(Matcher, R)>,
}

impl<Matcher, Rndr> NestedRouteView<Matcher, Rndr>
where
    Matcher: MatchInterface<Rndr> + MatchParams,
    Matcher::Child: 'static,
    Matcher::View: 'static,
    Rndr: Renderer + 'static,
{
    pub fn new(outer_owner: &Owner, route_match: Matcher) -> Self {
        // keep track of all outlets, for diffing
        let mut outlets = VecDeque::new();

        // build this view
        let owner = outer_owner.child();
        let id = route_match.as_id();
        let params =
            ArcRwSignal::new(route_match.to_params().into_iter().collect());
        let (view, child) = route_match.into_view_and_child();

        let outlet = child
            .map(|child| get_inner_view(&mut outlets, &owner, child))
            .unwrap_or_default();

        let route_data = RouteData {
            params: ArcMemo::new({
                let params = params.clone();
                move |_| params.get()
            }),
            outlet,
        };
        let view = owner.with(|| view.choose(route_data));

        Self {
            id,
            owner,
            params,
            outlets,
            view,
            ty: PhantomData,
        }
    }

    pub fn new_hydrate(
        outer_owner: &Owner,
        route_match: Matcher,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self {
        // keep track of all outlets, for diffing
        let mut outlets = VecDeque::new();

        // build this view
        let owner = outer_owner.child();
        let id = route_match.as_id();
        let params =
            ArcRwSignal::new(route_match.to_params().into_iter().collect());
        let (view, child) = route_match.into_view_and_child();

        let outlet = child
            .map(|child| {
                get_inner_view_hydrate(
                    &mut outlets,
                    &owner,
                    child,
                    cursor,
                    position,
                )
            })
            .unwrap_or_default();

        let route_data = RouteData {
            params: ArcMemo::new({
                let params = params.clone();
                move |_| params.get()
            }),
            outlet,
        };
        let view = owner.with(|| view.choose(route_data));

        Self {
            id,
            owner,
            params,
            outlets,
            view,
            ty: PhantomData,
        }
    }
}

pub struct NestedRouteState<Matcher, Rndr>
where
    Matcher: MatchInterface<Rndr>,
    Rndr: Renderer + 'static,
{
    id: RouteMatchId,
    owner: Owner,
    params: ArcRwSignal<Params>,
    view: <Matcher::View as Render<Rndr>>::State,
    outlets: VecDeque<Outlet<Rndr>>,
}

fn get_inner_view<Match, R>(
    outlets: &mut VecDeque<Outlet<R>>,
    parent: &Owner,
    route_match: Match,
) -> Outlet<R>
where
    Match: MatchInterface<R> + MatchParams,
    R: Renderer + 'static,
{
    let owner = parent.child();
    let id = route_match.as_id();
    let params =
        ArcRwSignal::new(route_match.to_params().into_iter().collect());
    let (view, child) = route_match.into_view_and_child();
    let outlet = child
        .map(|child| get_inner_view(outlets, &owner, child))
        .unwrap_or_default();

    let view = Rc::new(Lazy::new({
        let owner = owner.clone();
        let params = params.clone();
        Box::new(move || {
            RefCell::new(Some(
                owner
                    .with(|| {
                        view.choose(RouteData {
                            params: ArcMemo::new(move |_| params.get()),
                            outlet,
                        })
                    })
                    .into_any(),
            ))
        }) as Box<dyn FnOnce() -> RefCell<Option<AnyView<R>>>>
    }));
    let inner = Rc::new(RefCell::new(OutletStateInner {
        html_len: {
            let view = Rc::clone(&view);
            Box::new(move || view.borrow().html_len())
        },
        view: Rc::clone(&view),
        state: Lazy::new(Box::new(move || view.take().unwrap().build())),
    }));

    let outlet = Outlet {
        id,
        owner,
        params,
        inner,
    };
    outlets.push_back(outlet.clone());
    outlet
}

fn get_inner_view_hydrate<Match, R>(
    outlets: &mut VecDeque<Outlet<R>>,
    parent: &Owner,
    route_match: Match,
    cursor: &Cursor<R>,
    position: &PositionState,
) -> Outlet<R>
where
    Match: MatchInterface<R> + MatchParams,
    R: Renderer + 'static,
{
    let owner = parent.child();
    let id = route_match.as_id();
    let params =
        ArcRwSignal::new(route_match.to_params().into_iter().collect());
    let (view, child) = route_match.into_view_and_child();
    let outlet = child
        .map(|child| get_inner_view(outlets, &owner, child))
        .unwrap_or_default();

    let view = Rc::new(Lazy::new({
        let owner = owner.clone();
        let params = params.clone();
        Box::new(move || {
            RefCell::new(Some(
                owner
                    .with(|| {
                        view.choose(RouteData {
                            params: ArcMemo::new(move |_| params.get()),
                            outlet,
                        })
                    })
                    .into_any(),
            ))
        }) as Box<dyn FnOnce() -> RefCell<Option<AnyView<R>>>>
    }));
    let inner = Rc::new(RefCell::new(OutletStateInner {
        html_len: Box::new({
            let view = Rc::clone(&view);
            move || view.borrow().html_len()
        }),
        view: Rc::clone(&view),
        state: Lazy::new(Box::new({
            let cursor = cursor.clone();
            let position = position.clone();
            move || view.take().unwrap().hydrate::<true>(&cursor, &position)
        })),
    }));

    let outlet = Outlet {
        id,
        owner,
        params,
        inner,
    };
    outlets.push_back(outlet.clone());
    outlet
}

#[derive(Debug)]
pub struct Outlet<R>
where
    R: Renderer + 'static,
{
    id: RouteMatchId,
    owner: Owner,
    params: ArcRwSignal<Params>,
    inner: Rc<RefCell<OutletStateInner<R>>>,
}

impl<R> Clone for Outlet<R>
where
    R: Renderer + 'static,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            owner: self.owner.clone(),
            params: self.params.clone(),
            inner: Rc::clone(&self.inner),
        }
    }
}

impl<R> Default for Outlet<R>
where
    R: Renderer + 'static,
{
    fn default() -> Self {
        Self {
            id: RouteMatchId(0),
            owner: Owner::current().unwrap(),
            params: ArcRwSignal::new(Params::new()),
            inner: Default::default(),
        }
    }
}

impl<R> Render<R> for Outlet<R>
where
    R: Renderer + 'static,
{
    type State = Outlet<R>;
    type FallibleState = ();

    fn build(self) -> Self::State {
        self
    }

    fn rebuild(self, state: &mut Self::State) {
        todo!()
    }

    fn try_build(self) -> tachys::error::Result<Self::FallibleState> {
        todo!()
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> tachys::error::Result<()> {
        todo!()
    }
}

impl<R> RenderHtml<R> for Outlet<R>
where
    R: Renderer + 'static,
{
    const MIN_LENGTH: usize = 0; // TODO

    fn html_len(&self) -> usize {
        (self.inner.borrow().html_len)()
    }

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        let view = self.inner.borrow().view.take().unwrap();
        view.to_html_with_buf(buf, position);
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        let view = self.inner.borrow().view.take().unwrap();
        view.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        let view = self.inner.borrow().view.take().unwrap();
        let state = view.hydrate::<FROM_SERVER>(cursor, position);
        self
    }
}

pub struct OutletStateInner<R>
where
    R: Renderer + 'static,
{
    html_len: Box<dyn Fn() -> usize>,
    view: Rc<
        Lazy<
            RefCell<Option<AnyView<R>>>,
            Box<dyn FnOnce() -> RefCell<Option<AnyView<R>>>>,
        >,
    >,
    state: Lazy<AnyViewState<R>, Box<dyn FnOnce() -> AnyViewState<R>>>,
}

impl<R: Renderer> Debug for OutletStateInner<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutletStateInner").finish_non_exhaustive()
    }
}

impl<R> Default for OutletStateInner<R>
where
    R: Renderer + 'static,
{
    fn default() -> Self {
        let view =
            Rc::new(Lazy::new(Box::new(|| RefCell::new(Some(().into_any())))
                as Box<dyn FnOnce() -> RefCell<Option<AnyView<R>>>>));
        Self {
            html_len: Box::new(|| 0),
            view,
            state: Lazy::new(Box::new(|| ().into_any().build())),
        }
    }
}

impl<R> Mountable<R> for Outlet<R>
where
    R: Renderer + 'static,
{
    fn unmount(&mut self) {
        self.inner.borrow_mut().state.unmount();
    }

    fn mount(
        &mut self,
        parent: &<R as Renderer>::Element,
        marker: Option<&<R as Renderer>::Node>,
    ) {
        self.inner.borrow_mut().state.mount(parent, marker);
    }

    fn insert_before_this(
        &self,
        parent: &<R as Renderer>::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool {
        self.inner
            .borrow_mut()
            .state
            .insert_before_this(parent, child)
    }
}

fn rebuild_nested<Match, R>(
    outer_owner: &Owner,
    prev: &mut NestedRouteState<Match, R>,
    new_match: Match,
) where
    Match: MatchInterface<R> + MatchParams + std::fmt::Debug,
    R: Renderer + 'static,
{
    let mut items = 0;
    let NestedRouteState {
        id,
        owner,
        params,
        view,
        outlets,
    } = prev;

    if new_match.as_id() == *id {
        params.set(new_match.to_params().into_iter().collect::<Params>());
        let (_, child) = new_match.into_view_and_child();
        if let Some(child) = child {
            rebuild_inner(&mut items, outlets, child);
        } else {
            outlets.truncate(items);
        }
    } else {
        let new = NestedRouteView::new(outer_owner, new_match);
        new.rebuild(prev);
    }
}

fn rebuild_inner<Match, R>(
    items: &mut usize,
    outlets: &mut VecDeque<Outlet<R>>,
    route_match: Match,
) where
    Match: MatchInterface<R> + MatchParams,
    R: Renderer + 'static,
{
    *items += 1;

    match outlets.pop_front() {
        None => todo!(),
        Some(mut prev) => {
            let prev_id = prev.id;
            let new_id = route_match.as_id();

            // we'll always update the params to the new params
            prev.params
                .set(route_match.to_params().into_iter().collect::<Params>());

            if new_id == prev_id {
                outlets.push_front(prev);
                let (_, child) = route_match.into_view_and_child();
                if let Some(child) = child {
                    // we still recurse to the children, because they may also have changed
                    rebuild_inner(items, outlets, child);
                } else {
                    outlets.truncate(*items);
                }
            } else {
                // we'll be updating the previous outlet before pushing it back onto the stack
                // update the ID to the ID of the new route
                prev.id = new_id;
                outlets.push_front(prev.clone());

                // if different routes are matched here, it means the rest of the tree is no longer
                // matched either
                outlets.truncate(*items);

                // we'll build a fresh tree instead
                let (view, child) = route_match.into_view_and_child();

                // first, let's add all the outlets that would be created by children
                let outlet = child
                    .map(|child| get_inner_view(outlets, &prev.owner, child))
                    .unwrap_or_default();

                // now, let's update the previou route at this point in the tree
                let mut prev_state = prev.inner.borrow_mut();
                let new_view = prev.owner.with_cleanup(|| {
                    view.choose(RouteData {
                        params: ArcMemo::new({
                            let params = prev.params.clone();
                            move |_| params.get()
                        }),
                        outlet,
                    })
                });

                new_view.into_any().rebuild(&mut prev_state.state);
            }
        }
    }
}

impl<Matcher, R> Render<R> for NestedRouteView<Matcher, R>
where
    Matcher: MatchInterface<R>,
    Matcher::View: Sized + 'static,
    R: Renderer + 'static,
{
    type State = NestedRouteState<Matcher, R>;
    type FallibleState = ();

    fn build(self) -> Self::State {
        let NestedRouteView {
            id,
            owner,
            params,
            outlets,
            view,
            ty,
        } = self;
        NestedRouteState {
            id,
            owner,
            outlets,
            params,
            view: view.build(),
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        let NestedRouteView {
            id,
            owner,
            params,
            outlets,
            view,
            ty,
        } = self;
        state.id = id;
        state.owner = owner;
        state.params = params;
        state.outlets = outlets;
        view.rebuild(&mut state.view);
    }

    fn try_build(self) -> tachys::error::Result<Self::FallibleState> {
        todo!()
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> tachys::error::Result<()> {
        todo!()
    }
}

impl<Matcher, R> RenderHtml<R> for NestedRouteView<Matcher, R>
where
    Matcher: MatchInterface<R>,
    Matcher::View: Sized + 'static,
    R: Renderer + 'static,
{
    const MIN_LENGTH: usize = Matcher::View::MIN_LENGTH;

    fn html_len(&self) -> usize {
        self.view.html_len()
    }

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        buf.reserve(self.html_len());
        self.view.to_html_with_buf(buf, position);
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        buf.reserve(self.html_len());
        self.view
            .to_html_async_with_buf::<OUT_OF_ORDER>(buf, position)
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        let NestedRouteView {
            id,
            owner,
            params,
            outlets,
            view,
            ty,
        } = self;
        NestedRouteState {
            id,
            owner,
            outlets,
            params,
            view: view.hydrate::<FROM_SERVER>(cursor, position),
        }
    }
}

impl<Matcher, R> Mountable<R> for NestedRouteState<Matcher, R>
where
    Matcher: MatchInterface<R>,
    R: Renderer + 'static,
{
    fn unmount(&mut self) {
        self.view.unmount();
    }

    fn mount(&mut self, parent: &R::Element, marker: Option<&R::Node>) {
        self.view.mount(parent, marker);
    }

    fn insert_before_this(
        &self,
        parent: &R::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool {
        self.view.insert_before_this(parent, child)
    }
}

impl<Rndr, Loc, FallbackFn, Fallback, Children, View> AddAnyAttr<Rndr>
    for Router<Rndr, Loc, Children, FallbackFn>
where
    Loc: Location,
    FallbackFn: Fn() -> Fallback,
    Fallback: Render<Rndr>,
    Children: MatchNestedRoutes<Rndr>,
     <<Children as MatchNestedRoutes<Rndr>>::Match as MatchInterface<
        Rndr,
    >>::View: ChooseView<Rndr, Output = View>,
    Rndr: Renderer + 'static,
    Router<Rndr, Loc, Children, FallbackFn>: RenderHtml<Rndr>,
{
    type Output<SomeNewAttr: Attribute<Rndr>> = Self;

    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Rndr>,
    {
        self
    }

    fn add_any_attr_by_ref<NewAttr: Attribute<Rndr>>(
        self,
        attr: &NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Rndr>,
    {
        self
    }
}

#[derive(Debug)]
pub struct FlatRouter<Rndr, Loc, Children, FallbackFn> {
    base: Option<Cow<'static, str>>,
    location: PhantomData<Loc>,
    pub routes: Routes<Children, Rndr>,
    fallback: FallbackFn,
}

impl<Rndr, Loc, Children, FallbackFn, Fallback>
    FlatRouter<Rndr, Loc, Children, FallbackFn>
where
    Loc: Location,
    Rndr: Renderer,
    FallbackFn: Fn() -> Fallback,
{
    pub fn new(
        routes: Routes<Children, Rndr>,
        fallback: FallbackFn,
    ) -> FlatRouter<Rndr, Loc, Children, FallbackFn> {
        Self {
            base: None,
            location: PhantomData,
            routes,
            fallback,
        }
    }

    pub fn new_with_base(
        base: impl Into<Cow<'static, str>>,
        routes: Routes<Children, Rndr>,
        fallback: FallbackFn,
    ) -> FlatRouter<Rndr, Loc, Children, FallbackFn> {
        Self {
            base: Some(base.into()),
            location: PhantomData,
            routes,
            fallback,
        }
    }
}
impl<Rndr, Loc, FallbackFn, Fallback, Children> Render<Rndr>
    for FlatRouter<Rndr, Loc, Children, FallbackFn>
where
    Loc: Location,
    FallbackFn: Fn() -> Fallback + 'static,
    Fallback: Render<Rndr>,
    Children: MatchNestedRoutes<Rndr> + 'static,
    Fallback::State: 'static,
    Rndr: Renderer + 'static,
{
    type State =
        RenderEffect<
            EitherState<
                <<Children::Match as MatchInterface<Rndr>>::View as Render<
                    Rndr,
                >>::State,
                <Fallback as Render<Rndr>>::State,
                Rndr,
            >,
        >;
    type FallibleState = Self::State;

    fn build(self) -> Self::State {
        let location = Loc::new().unwrap(); // TODO
        location.init(self.base);
        let url = location.as_url().clone();
        let path = ArcMemo::new({
            let url = url.clone();
            move |_| url.read().path().to_string()
        });
        let search_params = ArcMemo::new({
            let url = url.clone();
            move |_| url.read().search_params().clone()
        });
        let outer_owner =
            Owner::current().expect("creating Router, but no Owner was found");

        RenderEffect::new(move |prev: Option<EitherState<_, _, _>>| {
            let path = path.read();
            let new_match = self.routes.match_route(&path);

            if let Some(mut prev) = prev {
                if let Some(new_match) = new_match {
                    let params = ArcRwSignal::new(
                        new_match.to_params().into_iter().collect(),
                    );
                    #[allow(unused)]
                    let (view, child) = new_match.into_view_and_child();
                    #[cfg(debug_assertions)]
                    if child.is_some() {
                        panic!(
                            "FlatRouter should not be used with a route that \
                             has a child."
                        );
                    }

                    let route_data = RouteData {
                        params: ArcMemo::new({
                            let params = params.clone();
                            move |_| params.get()
                        }),
                        outlet: Default::default(),
                    };
                    let view = outer_owner.with(|| view.choose(route_data));
                    Either::Left::<_, Fallback>(view).rebuild(&mut prev);
                } else {
                    Either::<<Children::Match as MatchInterface<Rndr>>::View, _>::Right((self.fallback)()).rebuild(&mut prev);
                }
                prev
            } else {
                match new_match {
                    Some(matched) => {
                        let params = ArcRwSignal::new(
                            matched.to_params().into_iter().collect(),
                        );
                        #[allow(unused)]
                        let (view, child) = matched.into_view_and_child();
                        #[cfg(debug_assertions)]
                        if child.is_some() {
                            panic!(
                                "FlatRouter should not be used with a route \
                                 that has a child."
                            );
                        }

                        let route_data = RouteData {
                            params: ArcMemo::new({
                                let params = params.clone();
                                move |_| params.get()
                            }),
                            outlet: Default::default(),
                        };
                        let view = outer_owner.with(|| view.choose(route_data));
                        Either::Left(view)
                    }
                    _ => Either::Right((self.fallback)()),
                }
                .build()
            }
        })
    }

    fn rebuild(self, state: &mut Self::State) {}

    fn try_build(self) -> tachys::error::Result<Self::FallibleState> {
        todo!()
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> tachys::error::Result<()> {
        todo!()
    }
}

impl<Rndr, Loc, FallbackFn, Fallback, Children> RenderHtml<Rndr>
    for FlatRouter<Rndr, Loc, Children, FallbackFn>
where
    Loc: Location,
    FallbackFn: Fn() -> Fallback + 'static,
    Fallback: RenderHtml<Rndr>,
    Children: MatchNestedRoutes<Rndr> + 'static,
    Fallback::State: 'static,
    Rndr: Renderer + 'static,
{
    const MIN_LENGTH: usize =
        <Children::Match as MatchInterface<Rndr>>::View::MIN_LENGTH;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        // if this is being run on the server for the first time, generating all possible routes
        if RouteList::is_generating() {
            // add routes
            let (base, routes) = self.routes.generate_routes();
            let mut routes = routes
                .into_iter()
                .map(|segments| {
                    let path = base
                        .into_iter()
                        .flat_map(|base| {
                            iter::once(PathSegment::Static(
                                base.to_string().into(),
                            ))
                        })
                        .chain(segments)
                        .collect::<Vec<_>>();
                    // TODO add non-defaults for mode, etc.
                    RouteListing::new(
                        path,
                        SsrMode::OutOfOrder,
                        [Method::Get],
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
            let outer_owner = Owner::current()
                .expect("creating Router, but no Owner was found");
            let url = use_context::<RequestUrl>()
                .expect("could not find request URL in context");
            // TODO base
            let url =
                RequestUrl::parse(url.as_ref()).expect("could not parse URL");
            // TODO query params
            match self.routes.match_route(url.path()) {
                Some(new_match) => {
                    let params = ArcRwSignal::new(
                        new_match.to_params().into_iter().collect(),
                    );
                    #[allow(unused)]
                    let (view, child) = new_match.into_view_and_child();
                    #[cfg(debug_assertions)]
                    if child.is_some() {
                        panic!(
                            "FlatRouter should not be used with a route that \
                             has a child."
                        );
                    }

                    let route_data = RouteData {
                        params: ArcMemo::new({
                            let params = params.clone();
                            move |_| params.get()
                        }),
                        outlet: Default::default(),
                    };
                    let view = outer_owner.with(|| view.choose(route_data));
                    Either::Left(view)
                }
                None => Either::Right((self.fallback)()),
            }
            .to_html_with_buf(buf, position)
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        let outer_owner =
            Owner::current().expect("creating Router, but no Owner was found");
        let url = use_context::<RequestUrl>()
            .expect("could not find request URL in context");
        // TODO base
        let url = RequestUrl::parse(url.as_ref()).expect("could not parse URL");
        // TODO query params
        match self.routes.match_route(url.path()) {
            Some(new_match) => {
                let params = ArcRwSignal::new(
                    new_match.to_params().into_iter().collect(),
                );
                #[allow(unused)]
                let (view, child) = new_match.into_view_and_child();
                #[cfg(debug_assertions)]
                if child.is_some() {
                    panic!(
                        "FlatRouter should not be used with a route that has \
                         a child."
                    );
                }

                let route_data = RouteData {
                    params: ArcMemo::new({
                        let params = params.clone();
                        move |_| params.get()
                    }),
                    outlet: Default::default(),
                };
                let view = outer_owner.with(|| view.choose(route_data));
                Either::Left(view)
            }
            None => Either::Right((self.fallback)()),
        }
        .to_html_async_with_buf::<OUT_OF_ORDER>(buf, position)
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self::State {
        let location = Loc::new().unwrap(); // TODO
        location.init(self.base);
        let url = location.as_url().clone();
        let path = ArcMemo::new({
            let url = url.clone();
            move |_| url.read().path().to_string()
        });
        let search_params = ArcMemo::new({
            let url = url.clone();
            move |_| url.read().search_params().clone()
        });
        let outer_owner =
            Owner::current().expect("creating Router, but no Owner was found");
        let cursor = cursor.clone();
        let position = position.clone();

        RenderEffect::new(move |prev: Option<EitherState<_, _, _>>| {
            let path = path.read();
            let new_match = self.routes.match_route(&path);

            if let Some(mut prev) = prev {
                if let Some(new_match) = new_match {
                    let params = ArcRwSignal::new(
                        new_match.to_params().into_iter().collect(),
                    );
                    #[allow(unused)]
                    let (view, child) = new_match.into_view_and_child();
                    #[cfg(debug_assertions)]
                    if child.is_some() {
                        panic!(
                            "FlatRouter should not be used with a route that \
                             has a child."
                        );
                    }

                    let route_data = RouteData {
                        params: ArcMemo::new({
                            let params = params.clone();
                            move |_| params.get()
                        }),
                        outlet: Default::default(),
                    };
                    let view = outer_owner.with(|| view.choose(route_data));
                    Either::Left::<_, Fallback>(view).rebuild(&mut prev);
                } else {
                    Either::<<Children::Match as MatchInterface<Rndr>>::View, _>::Right((self.fallback)()).rebuild(&mut prev);
                }
                prev
            } else {
                match new_match {
                    Some(matched) => {
                        let params = ArcRwSignal::new(
                            matched.to_params().into_iter().collect(),
                        );
                        #[allow(unused)]
                        let (view, child) = matched.into_view_and_child();
                        #[cfg(debug_assertions)]
                        if child.is_some() {
                            panic!(
                                "FlatRouter should not be used with a route \
                                 that has a child."
                            );
                        }

                        let route_data = RouteData {
                            params: ArcMemo::new({
                                let params = params.clone();
                                move |_| params.get()
                            }),
                            outlet: Default::default(),
                        };
                        let view = outer_owner.with(|| view.choose(route_data));
                        Either::Left(view)
                    }
                    _ => Either::Right((self.fallback)()),
                }
                .hydrate::<FROM_SERVER>(&cursor, &position)
            }
        })
    }
}
