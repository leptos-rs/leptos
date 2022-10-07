use std::rc::Rc;

use leptos::{create_memo, use_context, Memo, Scope};

use crate::{
    Location, NavigateOptions, NavigationError, Params, ParamsMap, RouteContext, RouterContext,
    RouterError,
};

pub fn use_router(cx: Scope) -> RouterContext {
    if let Some(router) = use_context::<RouterContext>(cx) {
        router
    } else {
        leptos::leptos_dom::debug_warn!("You must call use_router() within a <Router/> component");
        panic!("You must call use_router() within a <Router/> component");
    }
}

pub fn use_route(cx: Scope) -> RouteContext {
    use_context::<RouteContext>(cx).unwrap_or_else(|| use_router(cx).base())
}

pub fn use_location(cx: Scope) -> Location {
    use_router(cx).inner.location.clone()
}

pub fn use_params<T: Params>(cx: Scope) -> Memo<Result<T, RouterError>>
where
    T: PartialEq + std::fmt::Debug + Clone,
{
    let route = use_route(cx);
    create_memo(cx, move |_| route.params().with(T::from_map))
}

pub fn use_params_map(cx: Scope) -> Memo<ParamsMap> {
    let route = use_route(cx);
    route.params()
}

pub fn use_query<T: Params>(cx: Scope) -> Memo<Result<T, RouterError>>
where
    T: PartialEq + std::fmt::Debug + Clone,
{
    let router = use_router(cx);
    create_memo(cx, move |_| {
        router.inner.location.query.with(|m| T::from_map(m))
    })
}

pub fn use_query_map(cx: Scope) -> Memo<ParamsMap> {
    use_router(cx).inner.location.query
}

pub fn use_resolved_path(cx: Scope, path: impl Fn() -> String + 'static) -> Memo<Option<String>> {
    let route = use_route(cx);

    create_memo(cx, move |_| route.resolve_path(&path()).map(String::from))
}

pub fn use_navigate(cx: Scope) -> impl Fn(&str, NavigateOptions) -> Result<(), NavigationError> {
    let router = use_router(cx);
    move |to, options| Rc::clone(&router.inner).navigate_from_route(to, &options)
}
