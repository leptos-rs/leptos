use crate::{
    generate_route_list::RouteList,
    location::Location,
    matching::{
        MatchInterface, MatchNestedRoutes, PossibleRouteMatch, RouteMatchId,
        Routes,
    },
    ChooseView, Params,
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
    /*View: Render<Rndr> + IntoAny<Rndr> + 'static,
    View::State: 'static,*/
    Fallback::State: 'static,
    Rndr: Renderer + 'static,
{
    type State =
        RenderEffect<EitherState<(), <Fallback as Render<Rndr>>::State, Rndr>>;
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
                            //nested_rebuild(&outer_owner, prev, new_match);
                        }
                        Either::Right(_) => {
                            /*Either::<_, Fallback>::Left(NestedRouteView::new(
                                &outer_owner,
                                new_match,
                            ))
                            .rebuild(&mut prev);*/
                        }
                    }
                } else {
                    /*Either::<NestedRouteView<View, Rndr>, _>::Right((self
                        .fallback)(
                    ))
                    .rebuild(&mut prev);*/
                }
                prev
            } else {
                match new_match {
                    Some(matched) =>
                    /*Either::Left(NestedRouteView::new(
                        &outer_owner,
                        matched,
                    ))*/
                    {
                        Either::Left(())
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

pub struct NestedRouteView<View, R>
where
    R: Renderer,
{
    id: RouteMatchId,
    owner: Owner,
    params: ArcRwSignal<Params>,
    matched: ArcRwSignal<String>,
    view: View,
    rndr: PhantomData<R>,
}

impl<View, R> NestedRouteView<View, R>
where
    R: Renderer + 'static,
{
    pub fn new<Matcher>(outer_owner: &Owner, route_match: Matcher) -> Self
    where
        Matcher: for<'a> MatchInterface<'a, R, View = View> + 'static,
        for<'a> <Matcher as MatchInterface<'a, R>>::View:
            ChooseView<R, Output = View>,
        for<'a> <Matcher as MatchInterface<'a, R>>::Child: std::fmt::Debug,
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
}

impl<R, View> Render<R> for NestedRouteView<View, R>
where
    View: Render<R>,
    R: Renderer,
{
    type State = NestedRouteState<View::State>;
    type FallibleState = ();

    fn build(self) -> Self::State {
        let NestedRouteView {
            id,
            owner,
            params,
            matched,
            view,
            rndr,
        } = self;
        NestedRouteState {
            id,
            owner,
            params,
            matched,
            view: view.build(),
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

pub struct NestedRouteState<ViewState> {
    id: RouteMatchId,
    owner: Owner,
    params: ArcRwSignal<Params>,
    matched: ArcRwSignal<String>,
    view: ViewState,
}

impl<ViewState, R> Mountable<R> for NestedRouteState<ViewState>
where
    ViewState: Mountable<R>,
    R: Renderer,
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

trait RouteView<R>: for<'a> MatchInterface<'a, R>
where
    R: Renderer,
{
    type RouteViewChild: RouteView<R>;
    type RouteView: Render<R>;

    fn into_child(self) -> Option<Self::RouteViewChild>;
}

impl<Rndr, Loc, FallbackFn, Fallback, Children, View> RenderHtml<Rndr>
    for Router<Rndr, Loc, Children, FallbackFn>
where
    Loc: Location,
    FallbackFn: Fn() -> Fallback + 'static,
    Fallback: RenderHtml<Rndr>,
    Children: MatchNestedRoutes<Rndr> + 'static,
    for<'a> <<Children as MatchNestedRoutes<Rndr>>::Match<'a> as MatchInterface<
        'a,
        Rndr,
    >>::View: ChooseView<Rndr, Output = View>,
    for<'a> <<Children as MatchNestedRoutes<Rndr>>::Match<'a> as MatchInterface<
        'a,
        Rndr,
    >>::Child: std::fmt::Debug,
    View: Render<Rndr> + IntoAny<Rndr> + 'static,
    View::State: 'static,
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
    for<'a> <<Children as MatchNestedRoutes<Rndr>>::Match<'a> as MatchInterface<
        'a,
        Rndr,
    >>::View: ChooseView<Rndr, Output = View>,
    Rndr: Renderer,
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

macro_rules! tuples {
    ($either:ident => $($ty:ident),*) => {
        paste::paste! {
            impl<Rndr, $($ty, [<Fn $ty>],)*> ChooseView<Rndr> for $either<$([<Fn $ty>],)*>
            where
                Rndr: Renderer,
                $([<Fn $ty>]: Fn(RouteData<Rndr>) -> $ty,)*
                $($ty: Render<Rndr>,)*
            {
                type Output = $either<$($ty,)*>;

                fn choose(self, route_data: RouteData<Rndr>) -> Self::Output {
                    match self {
                        $($either::$ty(f) => $either::$ty(f(route_data)),)*
                    }
                }
            }
        }
    }
}

tuples!(EitherOf3 => A, B, C);
tuples!(EitherOf4 => A, B, C, D);
tuples!(EitherOf5 => A, B, C, D, E);
tuples!(EitherOf6 => A, B, C, D, E, F);
tuples!(EitherOf7 => A, B, C, D, E, F, G);
tuples!(EitherOf8 => A, B, C, D, E, F, G, H);
tuples!(EitherOf9 => A, B, C, D, E, F, G, H, I);
tuples!(EitherOf10 => A, B, C, D, E, F, G, H, I, J);
tuples!(EitherOf11 => A, B, C, D, E, F, G, H, I, J, K);
tuples!(EitherOf12 => A, B, C, D, E, F, G, H, I, J, K, L);
tuples!(EitherOf13 => A, B, C, D, E, F, G, H, I, J, K, L, M);
tuples!(EitherOf14 => A, B, C, D, E, F, G, H, I, J, K, L, M, N);
tuples!(EitherOf15 => A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
tuples!(EitherOf16 => A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
/*
impl<Rndr, Loc, Fal, Children> RenderHtml<Rndr>
    for Router<Rndr, Loc, Children, Fal>
where
    Self: FallbackOrViewHtml,
    Rndr: Renderer,
    Loc: Location,
    Children: PossibleRouteMatch,
    <Self as FallbackOrView>::Output: RenderHtml<Rndr>,
    Rndr::Element: Clone,
    Rndr::Node: Clone,
{
    const MIN_LENGTH: usize = <Self as FallbackOrViewHtml>::MIN_LENGTH;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        if RouteList::is_generating() {
            let routes = RouteList::default();
            RouteList::register(routes);
        } else {
            self.fallback_or_view().1.to_html_with_buf(buf, position);
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        if RouteList::is_generating() {
            let routes = RouteList::default();
            RouteList::register(routes);
        } else {
            self.fallback_or_view()
                .1
                .to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self::State {
        self.fallback_or_view()
            .1
            .hydrate::<FROM_SERVER>(cursor, position)
    }
}

pub trait FallbackOrView {
    type Output;

    fn fallback_or_view(&self) -> (&'static str, Self::Output);

    fn generate_route_list(&self, route_list: &mut RouteList);
}

pub trait FallbackOrViewHtml: FallbackOrView {
    const MIN_LENGTH: usize;
}

impl<Rndr, Loc, FallbackFn, Fal> FallbackOrView
    for Router<Rndr, Loc, (), FallbackFn>
where
    Rndr: Renderer,
    Loc: Location,
    FallbackFn: Fn() -> Fal,
    Fal: Render<Rndr>,
{
    type Output = Fal;

    fn fallback_or_view(&self) -> (&'static str, Self::Output) {
        ("Fal", (self.fallback)())
    }

    fn generate_route_list(&self, _route_list: &mut RouteList) {}
}

impl<Rndr, Loc, FallbackFn, Fal> FallbackOrViewHtml
    for Router<Rndr, Loc, (), FallbackFn>
where
    Rndr: Renderer,
    Loc: Location,
    FallbackFn: Fn() -> Fal,
    Fal: RenderHtml<Rndr>,
    Rndr::Element: Clone,
    Rndr::Node: Clone,
{
    const MIN_LENGTH: usize = Fal::MIN_LENGTH;
}

impl<Rndr, Loc, FallbackFn, Fal, APat, AViewFn, AView, AChildren> FallbackOrView
    for Router<
        Rndr,
        Loc,
        RouteDefinition<Rndr, APat, AViewFn, AChildren>,
        FallbackFn,
    >
where
    Rndr: Renderer,
    Loc: Location,
    APat: RouteMatch,
    AViewFn: Fn(MatchedRoute) -> AView,
    AView: Render<Rndr>,
    FallbackFn: Fn() -> Fal,
    Fal: Render<Rndr>,
{
    type Output = Either<Fal, AView>;

    fn fallback_or_view(&self) -> (&'static str, Self::Output) {
        match self.location.try_to_url() {
            Ok(url) => {
                if self.routes.path.matches(&url.pathname) {
                    let PartialPathMatch {
                        params,
                        matched,
                        remaining,
                    } = self.routes.path.test(&url.pathname).unwrap();
                    if remaining.is_empty() {
                        let matched = MatchedRoute {
                            params,
                            matched,
                            search_params: url.search_params.clone(),
                        };
                        return (
                            "Route",
                            Either::Right(self.routes.view(matched)),
                        );
                    }
                }
                ("Fal", Either::Left(self.fallback()))
            }
            Err(e) => {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!(
                        "Error converting location into URL: {e:?}"
                    );
                }
                ("Fal", Either::Left(self.fallback()))
            }
        }
    }

    fn generate_route_list(&self, route_list: &mut RouteList) {
        let mut path = Vec::new();
        self.routes.path.generate_path(&mut path);
        route_list.push(RouteListing::from_path(path));
    }
}

impl<Rndr, Loc, FallbackFn, Fal, APat, AViewFn, AView, AChildren>
    FallbackOrViewHtml
    for Router<
        Rndr,
        Loc,
        RouteDefinition<Rndr, APat, AViewFn, AChildren>,
        FallbackFn,
    >
where
    Rndr: Renderer,
    Loc: Location,
    APat: RouteMatch,
    AViewFn: Fn(MatchedRoute) -> AView,
    AView: RenderHtml<Rndr>,
    FallbackFn: Fn() -> Fal,
    Fal: RenderHtml<Rndr>,
    Rndr::Element: Clone,
    Rndr::Node: Clone,
{
    const MIN_LENGTH: usize = if Fal::MIN_LENGTH < AView::MIN_LENGTH {
        Fal::MIN_LENGTH
    } else {
        AView::MIN_LENGTH
    };
}*/
