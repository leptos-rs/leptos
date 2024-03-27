use crate::{
    location::BrowserUrl, matching, router, FlatRouter, NestedRoute, RouteData,
    Router, Routes,
};
use leptos::{children::ToChildren, component};
use std::borrow::Cow;
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
}

#[component]
pub fn Router<Children, FallbackFn, Fallback>(
    #[prop(optional, into)] base: Option<Cow<'static, str>>,
    fallback: FallbackFn,
    children: RouteChildren<Children>,
) -> Router<Dom, BrowserUrl, Children, FallbackFn>
where
    FallbackFn: Fn() -> Fallback,
{
    let children = Routes::new(children.into_inner());
    if let Some(base) = base {
        Router::new_with_base(base, children, fallback)
    } else {
        Router::new(children, fallback)
    }
}

#[component]
pub fn Route<Segments, View, ViewFn>(
    path: Segments,
    view: ViewFn,
) -> NestedRoute<Segments, (), (), ViewFn, Dom>
where
    ViewFn: Fn(RouteData<Dom>) -> View,
{
    NestedRoute::new(path, view)
}
