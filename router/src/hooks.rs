use crate::{
    Location, NavigateOptions, NavigationError, Params, ParamsError, ParamsMap,
    RouteContext, RouterContext,
};
use leptos::{create_memo, signal_prelude::*, use_context, Memo, Scope};
use std::rc::Rc;

/// Returns the current [RouterContext], containing information about the router's state.
pub fn use_router(cx: Scope) -> RouterContext {
    if let Some(router) = use_context::<RouterContext>(cx) {
        router
    } else {
        leptos::leptos_dom::debug_warn!(
            "You must call use_router() within a <Router/> component"
        );
        panic!("You must call use_router() within a <Router/> component");
    }
}

/// Returns the current [RouteContext], containing information about the matched route.
pub fn use_route(cx: Scope) -> RouteContext {
    use_context::<RouteContext>(cx).unwrap_or_else(|| use_router(cx).base())
}

/// Returns the current [Location], which contains reactive variables
pub fn use_location(cx: Scope) -> Location {
    use_router(cx).inner.location.clone()
}

/// Returns a raw key-value map of route params.
pub fn use_params_map(cx: Scope) -> Memo<ParamsMap> {
    let route = use_route(cx);
    route.params()
}

/// Returns the current route params, parsed into the given type, or an error.
pub fn use_params<T: Params>(cx: Scope) -> Memo<Result<T, ParamsError>>
where
    T: PartialEq,
{
    let route = use_route(cx);
    create_memo(cx, move |_| route.params().with(T::from_map))
}

/// Returns a raw key-value map of the URL search query.
pub fn use_query_map(cx: Scope) -> Memo<ParamsMap> {
    use_router(cx).inner.location.query
}

/// Returns the current URL search query, parsed into the given type, or an error.
pub fn use_query<T: Params>(cx: Scope) -> Memo<Result<T, ParamsError>>
where
    T: PartialEq,
{
    let router = use_router(cx);
    create_memo(cx, move |_| {
        router.inner.location.query.with(|m| T::from_map(m))
    })
}

/// Resolves the given path relative to the current route.
pub fn use_resolved_path(
    cx: Scope,
    path: impl Fn() -> String + 'static,
) -> Memo<Option<String>> {
    let route = use_route(cx);

    create_memo(cx, move |_| {
        let path = path();
        if path.starts_with('/') {
            Some(path)
        } else {
            route.resolve_path_tracked(&path).map(String::from)
        }
    })
}

/// Returns a function that can be used to navigate to a new route.
pub fn use_navigate(
    cx: Scope,
) -> impl Fn(&str, NavigateOptions) -> Result<(), NavigationError> {
    let router = use_router(cx);
    move |to, options| {
        Rc::clone(&router.inner).navigate_from_route(to, &options)
    }
}

/// Returns a signal that tells you whether you are currently navigating backwards.
pub(crate) fn use_is_back_navigation(cx: Scope) -> ReadSignal<bool> {
    let router = use_router(cx);
    router.inner.is_back.read_only()
}
