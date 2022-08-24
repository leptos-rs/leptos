use leptos_reactive::{use_context, Scope};

use crate::{Params, ParamsMap, RouteContext, RouterContext, RouterError};

pub fn use_router(cx: Scope) -> RouterContext {
    use_context(cx).expect("You must call use_router() within a <Router/> component")
}

pub fn use_route(cx: Scope) -> RouteContext {
    use_context(cx).unwrap_or_else(|| use_router(cx).base())
}

pub fn use_params<T: Params>(cx: Scope) -> Result<T, RouterError> {
    let route = use_route(cx);
    T::from_map(route.params())
}

pub fn use_params_map(cx: Scope) -> ParamsMap {
    let route = use_route(cx);
    route.params().clone()
}
