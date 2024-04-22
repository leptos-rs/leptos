use crate::{
    location::{BrowserUrl, Location, LocationProvider, State, Url},
    MatchNestedRoutes, NestedRoute, NestedRoutesView, Routes,
};
use leptos::{
    children::{ToChildren, TypedChildren},
    component, IntoView,
};
use reactive_graph::{
    computed::ArcMemo,
    owner::{provide_context, use_context, Owner},
    signal::{ArcRwSignal, RwSignal},
    traits::Read,
};
use std::{borrow::Cow, marker::PhantomData};
use tachys::renderer::{dom::Dom, Renderer};

#[derive(Debug)]
pub struct RouteChildren<Children>(Children);

impl<Children> RouteChildren<Children> {
    pub fn into_inner(self) -> Children {
        self.0
    }
}

impl<F, Children> ToChildren<F> for RouteChildren<Children>
where
    F: FnOnce() -> Children,
{
    fn to_children(f: F) -> Self {
        RouteChildren(f())
    }
}

#[component]
pub fn Router<Chil>(
    /// The base URL for the router. Defaults to `""`.
    #[prop(optional, into)]
    base: Option<Cow<'static, str>>,

    // TODO these prop
    ///// A fallback that should be shown if no route is matched.
    //#[prop(optional)]
    //fallback: Option<fn() -> View>,
    ///// A signal that will be set while the navigation process is underway.
    //#[prop(optional, into)]
    //set_is_routing: Option<SignalSetter<bool>>,
    ///// How trailing slashes should be handled in [`Route`] paths.
    //#[prop(optional)]
    //trailing_slash: TrailingSlash,
    /// The `<Router/>` should usually wrap your whole page. It can contain
    /// any elements, and should include a [`Routes`](crate::Routes) component somewhere
    /// to define and display [`Route`](crate::Route)s.
    children: TypedChildren<Chil>,
    /// A unique identifier for this router, allowing you to mount multiple Leptos apps with
    /// different routes from the same server.
    #[prop(optional)]
    id: usize,
) -> impl IntoView
where
    Chil: IntoView,
{
    let location =
        BrowserUrl::new().expect("could not access browser navigation"); // TODO options here
    location.init(base.clone());
    let url = location.as_url().clone();
    provide_context(url.clone());
    provide_context(Location::new(
        url.read_only().into(),
        // TODO state
        RwSignal::new(State::new(None)).read_only(),
    ));

    let children = children.into_inner();
    children()
}

/*
#[component]
pub fn FlatRouter<Children, FallbackFn, Fallback>(
    #[prop(optional, into)] base: Option<Cow<'static, str>>,
    fallback: FallbackFn,
    children: RouteChildren<Children>,
) -> FlatRouter<Dom, BrowserUrl, Children, FallbackFn>
where
    FallbackFn: Fn() -> Fallback,
{
    let children = Routes::new(children.into_inner());
    if let Some(base) = base {
        FlatRouter::new_with_base(base, children, fallback)
    } else {
        FlatRouter::new(children, fallback)
    }
}*/

#[component]
pub fn Routes<Defs, FallbackFn, Fallback>(
    #[prop(optional, into)] base: Option<Cow<'static, str>>,
    fallback: FallbackFn,
    children: RouteChildren<Defs>,
) -> impl IntoView
where
    Defs: MatchNestedRoutes<Dom> + Clone + Send + 'static,
    FallbackFn: Fn() -> Fallback + Send + 'static,
    Fallback: IntoView + 'static,
{
    let url = use_context::<ArcRwSignal<Url>>()
        .expect("<Routes> should be used inside a <Router> component");
    let routes = Routes::new(children.into_inner());
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
    move || NestedRoutesView {
        routes: routes.clone(),
        outer_owner: outer_owner.clone(),
        url: url.clone(),
        path: path.clone(),
        search_params: search_params.clone(),
        base: base.clone(), // TODO is this necessary?
        fallback: fallback(),
        rndr: PhantomData,
    }
}

#[component]
pub fn Route<Segments, View, ViewFn>(
    path: Segments,
    view: ViewFn,
) -> NestedRoute<Segments, (), (), ViewFn, Dom>
where
    ViewFn: Fn() -> View,
{
    NestedRoute::new(path, view)
}

#[component]
pub fn ParentRoute<Segments, View, Children, ViewFn>(
    path: Segments,
    view: ViewFn,
    children: RouteChildren<Children>,
) -> NestedRoute<Segments, Children, (), ViewFn, Dom>
where
    ViewFn: Fn() -> View,
{
    let children = children.into_inner();
    NestedRoute::new(path, view).child(children)
}
