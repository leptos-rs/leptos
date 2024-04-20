use crate::{
    location::{Location, Url},
    matching::Routes,
    ChooseView, MatchInterface, MatchNestedRoutes, MatchParams, Params,
    RouteMatchId,
};
use either_of::Either;
use leptos::{component, IntoView};
use or_poisoned::OrPoisoned;
use reactive_graph::{
    computed::ArcMemo,
    owner::{provide_context, use_context, Owner},
    signal::{ArcRwSignal, ArcTrigger},
    traits::{Read, Set, Track, Trigger},
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
        Mountable, Render, RenderHtml,
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
    pub search_params: ArcMemo<Params>,
    pub base: Option<Cow<'static, str>>,
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
    search_params: ArcMemo<Params>,
    outlets: VecDeque<OutletContext<R>>,
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

        let mut outlets = VecDeque::new();
        let new_match = routes.match_route(&path.read());
        let view = match new_match {
            None => Either::Left(fallback),
            Some(route) => {
                route.build_nested_route(&mut outlets, &outer_owner);
                provide_context(outlets[1].clone());
                Either::Right(Outlet(OutletProps::builder().build()).into_any())
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

        // TODO handle fallback => real view, fallback => fallback

        match new_match {
            None => {
                Either::<Fal, AnyView<R>>::Left(self.fallback)
                    .rebuild(&mut state.view);
                state.outlets.clear();
            }
            Some(route) => {
                route.rebuild_nested_route(
                    &mut 0,
                    &mut state.outlets,
                    &self.outer_owner,
                );
            }
        }
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

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut tachys::view::Position,
    ) {
        todo!()
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &tachys::hydration::Cursor<R>,
        position: &tachys::view::PositionState,
    ) -> Self::State {
        todo!()
    }
}

type OutletViewFn<R> = Box<dyn FnOnce() -> AnyView<R> + Send>;

#[derive(Debug)]
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

    fn rebuild_nested_route(
        self,
        items: &mut usize,
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
        let current_child = outlets.len();

        let (tx, rx) = std::sync::mpsc::channel();

        let outlet = OutletContext {
            id,
            trigger: ArcTrigger::new(),
            params,
            owner: parent.clone(),
            tx: tx.clone(),
            rx: Arc::new(Mutex::new(Some(rx))),
        };

        let (view, child) = self.into_view_and_child();

        // recursively continue building the tree
        // this is important because to build the view, we need access to the outlet
        // and the outlet will be returned from building this child
        if let Some(child) = child {
            child.build_nested_route(outlets, &owner);
        }

        outlet.trigger.trigger();
        tx.send(Box::new({
            let owner = owner.clone();
            let outlet = outlets.get(current_child + 0).cloned();
            let parent = parent.clone();
            move || {
                leptos::logging::log!("here");
                parent.with(|| {
                    if let Some(outlet) = outlet {
                        leptos::logging::log!(
                            "providing context on {:?}",
                            parent.debug_id()
                        );
                        provide_context(outlet);
                    } else {
                        leptos::logging::log!("nothing found");
                    }
                });
                owner.with(|| view.choose().into_any())
            }
        }));

        outlets.push_back(outlet);
    }

    fn rebuild_nested_route(
        self,
        items: &mut usize,
        outlets: &mut VecDeque<OutletContext<R>>,
        parent: &Owner,
    ) {
        let current = outlets.get_mut(*items);
        match current {
            // if there's nothing currently in the routes at this point, build from here
            None => {
                self.build_nested_route(outlets, parent);
            }
            Some(current) => {
                let id = self.as_id();
                // we always need to update the params, so go ahead and do that
                current
                    .params
                    .set(self.to_params().into_iter().collect::<Params>());
                let (view, child) = self.into_view_and_child();

                // if the IDs don't match, everything below in the tree needs to be swapped
                // 1) replace this outlet with the next view
                // 2) remove other outlets
                // 3) build down the chain
                if id != current.id {
                    let owner = current.owner.clone();
                    current.tx.send({
                        let owner = owner.clone();
                        Box::new(move || {
                            leptos::logging::log!(
                                "running Outlet view in {:?}",
                                owner.debug_id()
                            );
                            owner.with(|| view.choose().into_any())
                        })
                    });
                    current.trigger.trigger();
                    current.id = id;

                    // TODO check this offset
                    outlets.truncate(*items + 1);

                    if let Some(child) = child {
                        child.build_nested_route(outlets, &owner);
                    }

                    return;
                }

                // otherwise, just keep rebuilding recursively
                if let Some(child) = child {
                    let current = current.clone();
                    *items += 1;
                    child.rebuild_nested_route(items, outlets, &current.owner);
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
    let owner = Owner::current().unwrap();
    leptos::logging::log!("Outlet owner is {:?}", owner.debug_id());
    let ctx = use_context::<OutletContext<R>>()
        .expect("<Outlet/> used without OutletContext being provided.");
    let OutletContext {
        id,
        trigger,
        params,
        owner,
        tx,
        rx,
    } = ctx;
    let rx = rx.lock().or_poisoned().take().expect(
        "Tried to render <Outlet/> but could not find the view receiver. Are \
         you using the same <Outlet/> twice?",
    );
    move || {
        trigger.track();

        let x = rx.try_recv().map(|view| view()).unwrap();
        x
    }
}
