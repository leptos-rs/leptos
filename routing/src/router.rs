use crate::{
    generate_route_list::RouteList,
    location::Location,
    matching::{
        MatchInterface, MatchNestedRoutes, PossibleRouteMatch, RouteMatchId,
        Routes,
    },
    ChooseView, MatchParams, Params,
};
use core::marker::PhantomData;
use either_of::*;
use once_cell::unsync::Lazy;
use reactive_graph::{
    computed::{ArcMemo, Memo},
    effect::RenderEffect,
    owner::Owner,
    signal::ArcRwSignal,
    traits::{Get, Read, Set, Track},
};
use std::{
    any::Any, borrow::Cow, cell::RefCell, collections::VecDeque, rc::Rc,
};
use tachys::{
    html::attribute::Attribute,
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

#[derive(Debug)]
pub struct Router<Rndr, Loc, Children, FallbackFn> {
    base: Option<Cow<'static, str>>,
    location: Loc,
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
        location: Loc,
        routes: Routes<Children, Rndr>,
        fallback: FallbackFn,
    ) -> Router<Rndr, Loc, Children, FallbackFn> {
        Self {
            base: None,
            location,
            routes,
            fallback,
        }
    }

    pub fn new_with_base(
        base: impl Into<Cow<'static, str>>,
        location: Loc,
        routes: Routes<Children, Rndr>,
        fallback: FallbackFn,
    ) -> Router<Rndr, Loc, Children, FallbackFn> {
        Self {
            base: Some(base.into()),
            location,
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

pub struct RouteData<R>
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
    //for<'a> <Children::Match<'a> as MatchInterface<Rndr, View = View>>,
    /*View: Render<Rndr> + IntoAny<Rndr> + 'static,
    View::State: 'static,*/
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
        self.location.init(self.base);
        let url = self.location.as_url().clone();
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

    let inner = Rc::new(RefCell::new(OutletStateInner {
        state: Lazy::new({
            let params = params.clone();
            let owner = owner.clone();
            Box::new(move || {
                owner
                    .with(|| {
                        view.choose(RouteData {
                            params: ArcMemo::new(move |_| params.get()),
                            outlet,
                        })
                    })
                    .into_any()
                    .build()
            })
        }),
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
            inner: Rc::new(RefCell::new(OutletStateInner {
                state: Lazy::new(Box::new(|| ().into_any().build())),
            })),
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

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        todo!()
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        todo!()
    }
}

#[derive(Debug)]
pub struct OutletStateInner<R>
where
    R: Renderer + 'static,
{
    state: Lazy<AnyViewState<R>, Box<dyn FnOnce() -> AnyViewState<R>>>,
}

impl<R> Default for OutletStateInner<R>
where
    R: Renderer + 'static,
{
    fn default() -> Self {
        Self {
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
    Fallback::State: 'static,
    Rndr: Renderer + 'static,
    Children::Match: std::fmt::Debug,
    <Children::Match as MatchInterface<Rndr>>::Child: std::fmt::Debug,
{
    // TODO probably pick a max length here
    const MIN_LENGTH: usize = Fallback::MIN_LENGTH;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        todo!()
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self::State {
        todo!()
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
