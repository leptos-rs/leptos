use leptos_reactive::{create_memo, use_context, Memo, Scope};

use crate::{Location, Params, ParamsMap, RouteContext, RouterContext, RouterError};

pub fn use_router(cx: Scope) -> RouterContext {
    if let Some(router) = use_context::<RouterContext>(cx) {
        router
    } else {
        leptos_dom::debug_warn!("You must call use_router() within a <Router/> component");
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
    create_memo(cx, move |_| T::from_map(&route.params()))
}

pub fn use_params_map(cx: Scope) -> ParamsMap {
    let route = use_route(cx);
    route.params()
}

pub fn use_resolved_path(cx: Scope, path: impl Fn() -> String + 'static) -> Memo<Option<String>> {
    let route = use_route(cx);

    create_memo(cx, move |_| route.resolve_path(&path()).map(String::from))
}
