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
    traits::{Get, Read, Track},
};
use std::borrow::Cow;
use tachys::{
    html::attribute::Attribute,
    hydration::Cursor,
    renderer::Renderer,
    ssr::StreamBuilder,
    view::{
        add_attr::AddAnyAttr,
        any_view::{AnyView, IntoAny},
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
    #[inline(always)]
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

    #[inline(always)]
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
    R: Renderer,
{
    pub params: ArcMemo<Params>,
    pub outlet: Box<dyn FnOnce() -> AnyView<R>>,
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
                            //nested_rebuild(&outer_owner, prev, new_match);
                        }
                        Either::Right(_) => {
                            Either::<_, Fallback>::Left(
                                NestedRouteView::create(
                                    &outer_owner,
                                    new_match,
                                ),
                            )
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
                    Some(matched) => Either::Left(NestedRouteView::create(
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
    matched: ArcRwSignal<String>,
    view: Matcher::View,
    //child: Option<Box<dyn FnOnce() -> NestedRouteView<Matcher::Child, R>>>,
    ty: PhantomData<(Matcher, R)>,
}

impl<Matcher, Rndr> NestedRouteView<Matcher, Rndr>
where
    Matcher: MatchInterface<Rndr> + MatchParams,
    Matcher::Child: 'static,
    Matcher::View: 'static,
    Rndr: Renderer + 'static,
{
    pub fn create(outer_owner: &Owner, route_match: Matcher) -> Self {
        let id = route_match.as_id();
        let owner = outer_owner.child();
        let params = ArcRwSignal::new(
            route_match
                .to_params()
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        );
        let matched = ArcRwSignal::new(route_match.as_matched().to_string());
        /*let (view, child) = route_match.into_view_and_child();
        let child = child.map(|child| {
            let owner = owner.clone();
            Box::new(move || NestedRouteView::create(&owner, child))
                as Box<dyn FnOnce() -> NestedRouteView<Matcher::Child, Rndr>>
        });*/
        let view = build_nested(route_match);

        Self {
            id,
            owner,
            params,
            matched,
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
    matched: ArcRwSignal<String>,
    view: <Matcher::View as Render<Rndr>>::State,
    //child: Option<Box<NestedRouteState<Matcher::Child, Rndr>>>,
}

// TODO: also build a Vec<(RouteMatchId, ArcRwSignal<Params>)>
// when we rebuild, at each level,
// if the route IDs don't match, then replace with new
// if they do match, then just update params
fn build_nested<Match, R>(route_match: Match) -> Match::View
where
    Match: MatchInterface<R>,
    R: Renderer,
{
    let (view, child) = route_match.into_view_and_child();
    let outlet = move || child.map(|child| build_nested(child)).into_any();
    let data = RouteData {
        params: { ArcMemo::new(move |_| Params::new()) },
        outlet: Box::new(outlet),
    };
    view.choose(data)
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
        NestedRouteState {
            id: self.id,
            owner: self.owner,
            params: self.params,
            matched: self.matched,
            view: self.view.build(), //child: None,
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
