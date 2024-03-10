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
    type FallibleState = ();

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
                            // TODO!
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
        let view = view.choose(route_data);

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
    let state = Rc::new(RefCell::new(None));
    let (view, child) = route_match.into_view_and_child();
    let outlet = child
        .map(|child| get_inner_view(outlets, &owner, child))
        .unwrap_or_default();
    let fun = Rc::new(RefCell::new(Some({
        let params = params.clone();
        Box::new(move || {
            view.choose(RouteData {
                params: ArcMemo::new(move |_| params.get()),
                outlet,
            })
            .into_any()
        }) as Box<dyn FnOnce() -> AnyView<R>>
    })));

    let outlet = Outlet {
        id,
        owner,
        params,
        state,
        fun,
    };
    outlets.push_back(outlet.clone());
    outlet
}

pub struct Outlet<R>
where
    R: Renderer + 'static,
{
    id: RouteMatchId,
    owner: Owner,
    params: ArcRwSignal<Params>,
    state: Rc<RefCell<Option<OutletStateInner<R>>>>,
    fun: Rc<RefCell<Option<Box<dyn FnOnce() -> AnyView<R>>>>>,
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
            state: Rc::clone(&self.state),
            fun: Rc::clone(&self.fun),
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
            state: Default::default(),
            fun: Rc::new(RefCell::new(Some(Box::new(|| ().into_any())))),
        }
    }
}

impl<R> Render<R> for Outlet<R>
where
    R: Renderer + 'static,
{
    type State = OutletState<R>;
    type FallibleState = ();

    fn build(self) -> Self::State {
        let Outlet {
            id,
            owner,
            state,
            params,
            fun,
        } = self;
        let fun = fun
            .borrow_mut()
            .take()
            .expect("Outlet function taken before being built");
        let this = fun();
        *state.borrow_mut() = Some(OutletStateInner {
            state: this.build(),
        });
        OutletState {
            id,
            owner,
            state,
            params,
        }
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

pub struct OutletState<R>
where
    R: Renderer + 'static,
{
    id: RouteMatchId,
    owner: Owner,
    params: ArcRwSignal<Params>,
    state: Rc<RefCell<Option<OutletStateInner<R>>>>,
}

pub struct OutletStateInner<R>
where
    R: Renderer + 'static,
{
    state: AnyViewState<R>,
}

impl<R> Mountable<R> for OutletState<R>
where
    R: Renderer + 'static,
{
    fn unmount(&mut self) {
        self.state
            .borrow_mut()
            .as_mut()
            .expect("tried to access OutletState before it was built")
            .state
            .unmount();
    }

    fn mount(
        &mut self,
        parent: &<R as Renderer>::Element,
        marker: Option<&<R as Renderer>::Node>,
    ) {
        self.state
            .borrow_mut()
            .as_mut()
            .expect("tried to access OutletState before it was built")
            .state
            .mount(parent, marker);
    }

    fn insert_before_this(
        &self,
        parent: &<R as Renderer>::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool {
        self.state
            .borrow_mut()
            .as_mut()
            .expect("tried to access OutletState before it was built")
            .state
            .insert_before_this(parent, child)
    }
}

struct OutletData<R>
where
    R: Renderer,
{
    trigger: ArcRwSignal<()>,
    child: Rc<RefCell<Option<Box<dyn Any>>>>,
    fun: Box<dyn FnOnce() -> AnyView<R>>,
}

fn rebuild_nested<Match, R>(
    outer_owner: &Owner,
    prev: &mut NestedRouteState<Match, R>,
    new_match: Match,
) where
    Match: MatchInterface<R> + MatchParams,
    R: Renderer + 'static,
{
    let mut items = 1;
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
        Some(prev) => {
            let prev_id = prev.id;
            let new_id = route_match.as_id();

            if new_id == prev_id {
                // this is the same route, but the params may have changed
                // so we'll just update that params value
                prev.params.set(
                    route_match.to_params().into_iter().collect::<Params>(),
                );
                outlets.push_front(prev);
                let (_, child) = route_match.into_view_and_child();
                if let Some(child) = child {
                    // we still recurse to the children, because they may also have changed
                    rebuild_inner(items, outlets, child);
                } else {
                    outlets.truncate(*items);
                }
            } else {
                // if different routes are matched here, it means the rest of the tree is no longer
                // matched either
                outlets.truncate(*items);

                // we'll build a fresh tree instead
                // TODO check parent logic here...
                let new_outlet =
                    get_inner_view(outlets, &prev.owner, route_match);
                let fun = new_outlet.fun.borrow_mut().take().unwrap();
                let new_view = fun();
                new_view.rebuild(
                    &mut prev.state.borrow_mut().as_mut().unwrap().state,
                );
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

/*impl<View, R> NestedRouteView<View, R>
where
    R: Renderer + 'static,
{
    pub fn new<Matcher>(outer_owner: &Owner, route_match: Matcher) -> Self
    where
        Matcher: for<'a> MatchInterface<R, View = View> + 'static,
        for<'a> <Matcher as MatchInterface<R>>::View:
            ChooseView<R, Output = View>,
        for<'a> <Matcher as MatchInterface<R>>::Child: std::fmt::Debug,
        View: IntoAny<R> + 'static,
    {
        let params = ArcRwSignal::new(
            route_match
                .to_params()
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        );
        let matched = ArcRwSignal::new(route_match.as_matched().to_string());
        let id = route_match.as_id();
        let (view, child) = route_match.into_view_and_child();
        let route_data = RouteData {
            params: {
                let params = params.clone();
                ArcMemo::new(move |_| params.get())
            },
            outlet: Box::new({
                move || {
                    child
                        .map(|child| {
                            // TODO nest the next child and use real params
                            /*let params = child.to_params().into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<Params>();
                                                    let route_data = RouteData {
                                                        params: ArcMemo::new(move |_| {
                                                            params.clone()
                                                        }),
                            outlet: Box::new(|| ().into_any())
                                                    };*/
                            format!("{child:?}")
                        })
                        .into_any()
                }
            }),
        };
        NestedRouteView {
            id,
            owner: outer_owner.child(),
            params,
            matched,
            view: view.choose(route_data),
            rndr: PhantomData,
        }
    }
}*/

/*fn nested_rebuild<NewMatch, R>(
    outer_owner: &Owner,
    current: &mut NestedRouteState<
        <<NewMatch::View as ChooseView<R>>::Output as Render<R>>::State,
    >,
    new: NewMatch,
) where
    NewMatch: MatchInterface<R>,
    NewMatch::View: ChooseView<R>,
    <NewMatch::View as ChooseView<R>>::Output: Render<R> + IntoAny<R> + 'static,
    NewMatch::Child: std::fmt::Debug,
    R: Renderer + 'static,
{
    // if the new match is a different branch of the nested route tree from the current one, we can
    // just rebuild the view starting here: everything underneath it will change
    if new.as_id() != current.id {
        // TODO provide params + matched via context?
        let new_view = NestedRouteView::new(outer_owner, new);
        let prev_owner = std::mem::replace(&mut current.owner, new_view.owner);
        current.id = new_view.id;
        current.params = new_view.params;
        current.matched = new_view.matched;
        current
            .owner
            .with(|| new_view.view.rebuild(&mut current.view));

        // TODO is this the right place to drop the old Owner?
        drop(prev_owner);
    } else {
        // otherwise, we should recurse to the children of the current view, and the new match
        //nested_rebuild(current.as_child_mut(), new.as_child())
    }

    // update params, in case they're different
    // TODO
}*/

/*impl<View, R> NestedRouteView<View, R>
where
    R: Renderer + 'static,
{
    pub fn new<Matcher>(outer_owner: &Owner, route_match: Matcher) -> Self
    where
        Matcher: for<'a> MatchInterface<R, View = View> + 'static,
        for<'a> <Matcher as MatchInterface<R>>::View:
            ChooseView<R, Output = View>,
        for<'a> <Matcher as MatchInterface<R>>::Child: std::fmt::Debug,
        View: IntoAny<R> + 'static,
    {
        let params = ArcRwSignal::new(
            route_match
                .to_params()
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        );
        let matched = ArcRwSignal::new(route_match.as_matched().to_string());
        let id = route_match.as_id();
        let (view, child) = route_match.into_view_and_child();
        let route_data = RouteData {
            params: {
                let params = params.clone();
                ArcMemo::new(move |_| params.get())
            },
            outlet: Box::new({
                move || {
                    child
                        .map(|child| {
                            // TODO nest the next child and use real params
                            /*let params = child.to_params().into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<Params>();
                                                    let route_data = RouteData {
                                                        params: ArcMemo::new(move |_| {
                                                            params.clone()
                                                        }),
                            outlet: Box::new(|| ().into_any())
                                                    };*/
                            format!("{child:?}")
                        })
                        .into_any()
                }
            }),
        };
        NestedRouteView {
            id,
            owner: outer_owner.child(),
            params,
            matched,
            view: view.choose(route_data),
            rndr: PhantomData,
        }
    }
}*/

trait RouteView<R>: for<'a> MatchInterface<R>
where
    R: Renderer + 'static,
{
    type RouteViewChild: RouteView<R>;
    type RouteView: Render<R>;

    fn into_child(self) -> Option<Self::RouteViewChild>;
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
