use crate::{
    location::{Location, RequestUrl, Url},
    matching::Routes,
    params::ParamsMap,
    resolve_path::resolve_path,
    ChooseView, MatchInterface, MatchNestedRoutes, MatchParams, Method,
    PathSegment, RouteList, RouteListing, RouteMatchId,
};
use either_of::Either;
use leptos::{component, oco::Oco, IntoView};
use or_poisoned::OrPoisoned;
use reactive_graph::{
    computed::{ArcMemo, Memo},
    owner::{provide_context, use_context, Owner},
    signal::{ArcRwSignal, ArcTrigger},
    traits::{Get, Read, ReadUntracked, Set, Track, Trigger},
};
use std::{
    borrow::Cow,
    iter,
    marker::PhantomData,
    mem,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
};
use tachys::{
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

pub struct Outlet<R> {
    rndr: PhantomData<R>,
}

pub(crate) struct NestedRoutesView<Defs, Fal, R> {
    pub routes: Routes<Defs, R>,
    pub outer_owner: Owner,
    pub url: ArcRwSignal<Url>,
    pub path: ArcMemo<String>,
    pub search_params: ArcMemo<ParamsMap>,
    pub base: Option<Oco<'static, str>>,
    pub fallback: Fal,
    pub rndr: PhantomData<R>,
}

pub struct NestedRouteViewState<Fal, R>
where
    Fal: Render<R>,
    R: Renderer + 'static,
{
    outer_owner: Owner,
    url: ArcRwSignal<Url>,
    path: ArcMemo<String>,
    search_params: ArcMemo<ParamsMap>,
    outlets: Vec<RouteContext<R>>,
    view: EitherState<Fal::State, AnyViewState<R>, R>,
}

impl<Defs, Fal, R> Render<R> for NestedRoutesView<Defs, Fal, R>
where
    Defs: MatchNestedRoutes<R>,
    Fal: Render<R>,
    R: Renderer + 'static,
{
    type State = NestedRouteViewState<Fal, R>;

    fn build(self) -> Self::State {
        let NestedRoutesView {
            routes,
            outer_owner,
            url,
            path,
            search_params,
            fallback,
            base,
            ..
        } = self;

        let mut outlets = Vec::new();
        let new_match = routes.match_route(&path.read());
        let view = match new_match {
            None => Either::Left(fallback),
            Some(route) => {
                route.build_nested_route(base, &mut outlets, &outer_owner);
                outer_owner.with(|| {
                    Either::Right(
                        Outlet(OutletProps::builder().build()).into_any(),
                    )
                })
            }
        }
        .build();

        NestedRouteViewState {
            outlets,
            view,
            outer_owner,
            url,
            path,
            search_params,
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        let new_match = self.routes.match_route(&self.path.read());

        match new_match {
            None => {
                Either::<Fal, AnyView<R>>::Left(self.fallback)
                    .rebuild(&mut state.view);
                state.outlets.clear();
            }
            Some(route) => {
                route.rebuild_nested_route(
                    self.base,
                    &mut 0,
                    &mut state.outlets,
                    &self.outer_owner,
                );

                // if it was on the fallback, show the view instead
                if matches!(state.view.state, Either::Left(_)) {
                    self.outer_owner.with(|| {
                        Either::<Fal, AnyView<R>>::Right(
                            Outlet(OutletProps::builder().build()).into_any(),
                        )
                        .rebuild(&mut state.view);
                    })
                }
            }
        }
    }
}

impl<Defs, Fal, R> AddAnyAttr<R> for NestedRoutesView<Defs, Fal, R>
where
    Defs: MatchNestedRoutes<R> + Send,
    Fal: RenderHtml<R>,
    R: Renderer + 'static,
{
    type Output<SomeNewAttr: leptos::attr::Attribute<R>> =
        NestedRoutesView<Defs, Fal, R>;

    fn add_any_attr<NewAttr: leptos::attr::Attribute<R>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<R>,
    {
        todo!()
    }
}

impl<Defs, Fal, R> RenderHtml<R> for NestedRoutesView<Defs, Fal, R>
where
    Defs: MatchNestedRoutes<R> + Send,
    Fal: RenderHtml<R>,
    R: Renderer + 'static,
{
    type AsyncOutput = Self;

    const MIN_LENGTH: usize = 0; // TODO

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        // if this is being run on the server for the first time, generating all possible routes
        if RouteList::is_generating() {
            // add routes
            let (base, routes) = self.routes.generate_routes();
            let mut routes = routes
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
                url,
                path,
                search_params,
                fallback,
                base,
                ..
            } = self;

            let mut outlets = Vec::new();
            let new_match = routes.match_route(&path.read());
            let view = match new_match {
                None => Either::Left(fallback),
                Some(route) => {
                    route.build_nested_route(base, &mut outlets, &outer_owner);
                    outer_owner.with(|| {
                        Either::Right(
                            Outlet(OutletProps::builder().build()).into_any(),
                        )
                    })
                }
            };
            view.to_html_with_buf(buf, position);
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        let NestedRoutesView {
            routes,
            outer_owner,
            url,
            path,
            search_params,
            fallback,
            base,
            ..
        } = self;

        let mut outlets = Vec::new();
        let new_match = routes.match_route(&path.read());
        let view = match new_match {
            None => Either::Left(fallback),
            Some(route) => {
                route.build_nested_route(base, &mut outlets, &outer_owner);
                outer_owner.with(|| {
                    Either::Right(
                        Outlet(OutletProps::builder().build()).into_any(),
                    )
                })
            }
        };
        view.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        let NestedRoutesView {
            routes,
            outer_owner,
            url,
            path,
            search_params,
            fallback,
            base,
            ..
        } = self;

        let mut outlets = Vec::new();
        let new_match = routes.match_route(&path.read());
        let view = match new_match {
            None => Either::Left(fallback),
            Some(route) => {
                route.build_nested_route(base, &mut outlets, &outer_owner);
                outer_owner.with(|| {
                    Either::Right(
                        Outlet(OutletProps::builder().build()).into_any(),
                    )
                })
            }
        }
        .hydrate::<FROM_SERVER>(cursor, position);

        NestedRouteViewState {
            outlets,
            view,
            outer_owner,
            url,
            path,
            search_params,
        }
    }
}

type OutletViewFn<R> = Box<dyn FnOnce() -> AnyView<R> + Send>;

#[derive(Debug)]
pub(crate) struct RouteContext<R>
where
    R: Renderer,
{
    id: RouteMatchId,
    trigger: ArcTrigger,
    params: ArcRwSignal<ParamsMap>,
    owner: Owner,
    pub matched: ArcRwSignal<String>,
    base: Option<Oco<'static, str>>,
    tx: Sender<OutletViewFn<R>>,
    rx: Arc<Mutex<Option<Receiver<OutletViewFn<R>>>>>,
}

impl<R> RouteContext<R>
where
    R: Renderer + 'static,
{
    fn provide_contexts(&self) {
        provide_context(self.params.read_only());
        provide_context(self.clone());
    }
}

impl<R> Clone for RouteContext<R>
where
    R: Renderer,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            trigger: self.trigger.clone(),
            params: self.params.clone(),
            owner: self.owner.clone(),
            matched: self.matched.clone(),
            base: self.base.clone(),
            tx: self.tx.clone(),
            rx: self.rx.clone(),
        }
    }
}

trait AddNestedRoute<R>
where
    R: Renderer,
{
    fn build_nested_route(
        self,
        base: Option<Oco<'static, str>>,
        outlets: &mut Vec<RouteContext<R>>,
        parent: &Owner,
    );

    fn rebuild_nested_route(
        self,
        base: Option<Oco<'static, str>>,
        items: &mut usize,
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
        base: Option<Oco<'static, str>>,
        outlets: &mut Vec<RouteContext<R>>,
        parent: &Owner,
    ) {
        // each Outlet gets its own owner, so it can inherit context from its parent route,
        // a new owner will be constructed if a different route replaces this one in the outlet,
        // so that any signals it creates or context it provides will be cleaned up
        let owner = parent.child();

        // the params signal can be updated to allow the same outlet to update to changes in the
        // params, even if there's not a route match change
        let params = ArcRwSignal::new(self.to_params().into_iter().collect());

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
        let (tx, rx) = mpsc::channel();

        // add this outlet to the end of the outlet stack used for diffing
        let outlet = RouteContext {
            id: self.as_id(),
            trigger,
            params,
            owner: owner.clone(),
            matched: ArcRwSignal::new(self.as_matched().to_string()),
            tx: tx.clone(),
            rx: Arc::new(Mutex::new(Some(rx))),
            base: base.clone(),
        };
        outlets.push(outlet.clone());

        // send the initial view through the channel, and recurse through the children
        let (view, child) = self.into_view_and_child();

        tx.send(Box::new({
            let owner = outlet.owner.clone();
            move || owner.with(|| view.choose().into_any())
        }));

        // and share the outlet with the parent via context
        // we share it with the *parent* because the <Outlet/> is rendered in or below the parent
        // wherever it appears, <Outlet/> will look for the closest RouteContext
        parent.with(|| outlet.provide_contexts());

        // recursively continue building the tree
        // this is important because to build the view, we need access to the outlet
        // and the outlet will be returned from building this child
        if let Some(child) = child {
            child.build_nested_route(base, outlets, &owner);
        }
    }

    fn rebuild_nested_route(
        self,
        base: Option<Oco<'static, str>>,
        items: &mut usize,
        outlets: &mut Vec<RouteContext<R>>,
        parent: &Owner,
    ) {
        let current = outlets.get_mut(*items);
        match current {
            // if there's nothing currently in the routes at this point, build from here
            None => {
                self.build_nested_route(base, outlets, parent);
            }
            Some(current) => {
                // a unique ID for each route, which allows us to compare when we get new matches
                // if two IDs are the same, we do not rerender, but only update the params
                // if the IDs are different, we need to replace the remainder of the tree
                let id = self.as_id();

                // whether the route is the same or different, we always need to
                // 1) update the params (if they've changed),
                // 2) update the matched path (if it's changed),
                // 2) access the view and children

                let new_params =
                    self.to_params().into_iter().collect::<ParamsMap>();
                if current.params.read() != new_params {
                    current.params.set(new_params);
                }
                let new_match = self.as_matched();
                if &*current.matched.read() != new_match {
                    current.matched.set(new_match);
                }

                let (view, child) = self.into_view_and_child();

                // if the IDs don't match, everything below in the tree needs to be swapped:
                // 1) replace this outlet with the next view, with a new owner
                // 2) remove other outlets that are lower down in the match tree
                // 3) build the rest of the list of matched routes, rather than rebuilding,
                //    as all lower outlets needs to be replaced
                if id != current.id {
                    // update the ID of the match at this depth, so that futures rebuilds diff
                    // against the new ID, not the original one
                    current.id = id;

                    // assign a new owner, so that contexts and signals owned by the previous route
                    // in this outlet can be dropped
                    let old_owner =
                        mem::replace(&mut current.owner, parent.child());
                    let owner = current.owner.clone();

                    // send the new view, with the new owner, through the channel to the Outlet,
                    // and notify the trigger so that the reactive view inside the Outlet tracking
                    // the trigger runs again
                    current.tx.send({
                        let owner = owner.clone();
                        Box::new(move || {
                            owner.with(|| view.choose().into_any())
                        })
                    });
                    current.trigger.trigger();

                    // remove all the items lower in the tree
                    // if this match is different, all its children will also be different
                    outlets.truncate(*items + 1);

                    // if this children has matches, then rebuild the lower section of the tree
                    if let Some(child) = child {
                        let mut new_outlets = Vec::new();
                        child.build_nested_route(
                            base,
                            &mut new_outlets,
                            &owner,
                        );
                        outlets.extend(new_outlets);
                    }

                    return;
                }

                // otherwise, just keep rebuilding recursively, checking the remaining routes in
                // the list
                if let Some(child) = child {
                    let owner = current.owner.clone();
                    *items += 1;
                    child.rebuild_nested_route(base, items, outlets, &owner);
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

    fn insert_before_this(
        &self,
        parent: &R::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool {
        self.view.insert_before_this(parent, child)
    }
}

#[component]
pub fn Outlet<R>(#[prop(optional)] rndr: PhantomData<R>) -> impl RenderHtml<R>
where
    R: Renderer + 'static,
{
    _ = rndr;
    let ctx = use_context::<RouteContext<R>>()
        .expect("<Outlet/> used without RouteContext being provided.");
    let RouteContext {
        id,
        trigger,
        params,
        owner,
        tx,
        rx,
        ..
    } = ctx;
    let rx = rx.lock().or_poisoned().take().expect(
        "Tried to render <Outlet/> but could not find the view receiver. Are \
         you using the same <Outlet/> twice?",
    );
    move || {
        trigger.track();

        rx.try_recv().map(|view| view()).unwrap()
    }
}
