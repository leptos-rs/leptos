use crate::{
    location::{BrowserUrl, Location},
    MatchNestedRoutes, NestedRoute, NestedRoutesView, Routes,
};
use leptos::{children::ToChildren, component, IntoView};
use reactive_graph::{computed::ArcMemo, owner::Owner, traits::Read};
use std::{borrow::Cow, marker::PhantomData};
use tachys::renderer::dom::Dom;

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
pub fn Router<Defs, FallbackFn, Fallback>(
    #[prop(optional, into)] base: Option<Cow<'static, str>>,
    fallback: FallbackFn,
    children: RouteChildren<Defs>,
) -> impl IntoView
where
    Defs: MatchNestedRoutes<Dom> + Clone + Send + 'static,
    FallbackFn: Fn() -> Fallback + Send + 'static,
    Fallback: IntoView + 'static,
{
    let routes = Routes::new(children.into_inner());
    let location =
        BrowserUrl::new().expect("could not access browser navigation"); // TODO options here
    location.init(base.clone());
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
