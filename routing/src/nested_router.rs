use crate::{
    location::{Location, Url},
    matching::Routes,
    ChooseView, MatchInterface, MatchNestedRoutes, MatchParams, Params,
    RouteMatchId,
};
use either_of::Either;
use reactive_graph::{
    computed::ArcMemo,
    owner::{provide_context, Owner},
    signal::{ArcRwSignal, ArcTrigger},
    traits::{Read, Trigger},
};
use std::{
    borrow::Cow,
    cell::RefCell,
    collections::VecDeque,
    marker::PhantomData,
    rc::Rc,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};
use tachys::{
    renderer::{dom::Dom, Renderer},
    view::{
        any_view::{AnyView, AnyViewState, IntoAny},
        either::EitherState,
        Mountable, Render,
    },
};

pub struct RouteData<R = Dom>
where
    R: Renderer + 'static,
{
    pub params: ArcMemo<Params>,
    pub outlet: Outlet<R>,
}

pub struct Outlet<R> {
    rndr: PhantomData<R>,
}

pub struct NestedRoutesView<Loc, Defs, Fal, R> {
    routes: Routes<Defs, R>,
    outer_owner: Owner,
    url: ArcRwSignal<Url>,
    path: ArcMemo<String>,
    search_params: ArcMemo<Params>,
    base: Option<Cow<'static, str>>,
    fallback: Fal,
    loc: PhantomData<Loc>,
    rndr: PhantomData<R>,
}

pub struct NestedRouteViewState<Fal, R>
where
    Fal: Render<R>,
    R: Renderer,
{
    outer_owner: Owner,
    url: ArcRwSignal<Url>,
    path: ArcMemo<String>,
    search_params: ArcMemo<Params>,
    outlets: VecDeque<OutletContext<R>>,
    view: EitherState<Fal::State, (), R>,
}

impl<Loc, Defs, Fal, R> Render<R> for NestedRoutesView<Loc, Defs, Fal, R>
where
    Loc: Location,
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

        let mut outlets = VecDeque::new();
        let new_match = routes.match_route(&path.read());
        let view = match new_match {
            None => Either::Left(fallback),
            Some(route) => {
                route.build_nested_route(&mut outlets, &outer_owner);
                Either::Right(())
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
                Either::<Fal, ()>::Left(self.fallback).rebuild(&mut state.view);
                state.outlets.clear();
            }
            Some(route) => {
                todo!()
            }
        }
    }
}

type OutletViewFn<R> = Box<dyn FnOnce() -> AnyView<R> + Send>;

pub struct OutletContext<R>
where
    R: Renderer,
{
    id: RouteMatchId,
    trigger: ArcTrigger,
    params: ArcRwSignal<Params>,
    owner: Owner,
    tx: Sender<OutletViewFn<R>>,
    rx: Arc<Mutex<Option<Receiver<OutletViewFn<R>>>>>,
}

impl<R> Clone for OutletContext<R>
where
    R: Renderer,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            trigger: self.trigger.clone(),
            params: self.params.clone(),
            owner: self.owner.clone(),
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
        outlets: &mut VecDeque<OutletContext<R>>,
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
        outlets: &mut VecDeque<OutletContext<R>>,
        parent: &Owner,
    ) {
        let owner = parent.child();
        let id = self.as_id();
        let params = ArcRwSignal::new(self.to_params().into_iter().collect());

        let (tx, rx) = std::sync::mpsc::channel();

        let outlet = OutletContext {
            id,
            trigger: ArcTrigger::new(),
            params,
            owner: owner.clone(),
            tx: tx.clone(),
            rx: Arc::new(Mutex::new(Some(rx))),
        };
        owner.with(|| provide_context(outlet.clone()));

        let (view, child) = self.into_view_and_child();
        outlet.trigger.trigger();
        tx.send(Box::new(move || view.choose(todo!()).into_any()));

        // recursively continue building the tree
        // this is important because to build the view, we need access to the outlet
        // and the outlet will be returned from building this child
        if let Some(child) = child {
            child.build_nested_route(outlets, &owner);
        }

        outlets.push_back(outlet);
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
