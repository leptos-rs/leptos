use crate::{
    Location, NavigateOptions, Params, ParamsError, ParamsMap, RouteContext,
    RouterContext,
};
use leptos::{
    request_animation_frame, signal_prelude::*, use_context, window, Oco,
};
use std::{rc::Rc, str::FromStr};

/// Constructs a signal synchronized with a specific URL query parameter.
///
/// The function creates a bidirectional sync mechanism between the state encapsulated in a signal and a URL query parameter.
/// This means that any change to the state will update the URL, and vice versa, making the function especially useful
/// for maintaining state consistency across page reloads.
///
/// The `key` argument is the unique identifier for the query parameter to be synced with the state.
/// It is important to note that only one state can be tied to a specific key at any given time.
///
/// The function operates with types that can be parsed from and formatted into strings, denoted by `T`.
/// If the parsing fails for any reason, the function treats the value as `None`.
/// The URL parameter can be cleared by setting the signal to `None`.
///
/// ```rust
/// use leptos::*;
/// use leptos_router::*;
///
/// #[component]
/// pub fn SimpleQueryCounter() -> impl IntoView {
///     let (count, set_count) = create_query_signal::<i32>("count");
///     let clear = move |_| set_count.set(None);
///     let decrement =
///         move |_| set_count.set(Some(count.get().unwrap_or(0) - 1));
///     let increment =
///         move |_| set_count.set(Some(count.get().unwrap_or(0) + 1));
///
///     view! {
///         <div>
///             <button on:click=clear>"Clear"</button>
///             <button on:click=decrement>"-1"</button>
///             <span>"Value: " {move || count.get().unwrap_or(0)} "!"</span>
///             <button on:click=increment>"+1"</button>
///         </div>
///     }
/// }
/// ```
#[track_caller]
pub fn create_query_signal<T>(
    key: impl Into<Oco<'static, str>>,
) -> (Memo<Option<T>>, SignalSetter<Option<T>>)
where
    T: FromStr + ToString + PartialEq,
{
    create_query_signal_with_options::<T>(key, NavigateOptions::default())
}

#[track_caller]
pub fn create_query_signal_with_options<T>(
    key: impl Into<Oco<'static, str>>,
    nav_options: NavigateOptions,
) -> (Memo<Option<T>>, SignalSetter<Option<T>>)
where
    T: FromStr + ToString + PartialEq,
{
    let mut key: Oco<'static, str> = key.into();
    let query_map = use_query_map();
    let navigate = use_navigate();
    let location = use_location();

    let get = create_memo({
        let key = key.clone_inplace();
        move |_| {
            query_map
                .with(|map| map.get(&key).and_then(|value| value.parse().ok()))
        }
    });

    let set = SignalSetter::map(move |value: Option<T>| {
        let mut new_query_map = query_map.get();
        match value {
            Some(value) => {
                new_query_map.insert(key.to_string(), value.to_string());
            }
            None => {
                new_query_map.remove(&key);
            }
        }
        let qs = new_query_map.to_query_string();
        let path = location.pathname.get_untracked();
        let hash = location.hash.get_untracked();
        let new_url = format!("{path}{qs}{hash}");
        navigate(&new_url, nav_options.clone());
    });

    (get, set)
}

#[track_caller]
pub(crate) fn has_router() -> bool {
    use_context::<RouterContext>().is_some()
}

/// Returns the current [`RouterContext`], containing information about the router's state.
#[track_caller]
pub fn use_router() -> RouterContext {
    if let Some(router) = use_context::<RouterContext>() {
        router
    } else {
        leptos::leptos_dom::debug_warn!(
            "You must call use_router() within a <Router/> component {:?}",
            std::panic::Location::caller()
        );
        panic!("You must call use_router() within a <Router/> component");
    }
}

/// Returns the current [`RouteContext`], containing information about the matched route.
#[track_caller]
pub fn use_route() -> RouteContext {
    use_context::<RouteContext>().unwrap_or_else(|| use_router().base())
}

/// Returns the data for the current route, which is provided by the `data` prop on `<Route/>`.
#[track_caller]
pub fn use_route_data<T: Clone + 'static>() -> Option<T> {
    let route = use_context::<RouteContext>()?;
    let data = route.inner.data.borrow();
    let data = data.clone()?;
    let downcast = data.downcast_ref::<T>().cloned();
    downcast
}

/// Returns the current [`Location`], which contains reactive variables
#[track_caller]
pub fn use_location() -> Location {
    use_router().inner.location.clone()
}

/// Returns a raw key-value map of route params.
#[track_caller]
pub fn use_params_map() -> Memo<ParamsMap> {
    let route = use_route();
    route.params()
}

/// Returns the current route params, parsed into the given type, or an error.
#[track_caller]
pub fn use_params<T>() -> Memo<Result<T, ParamsError>>
where
    T: Params + PartialEq,
{
    let route = use_route();
    create_memo(move |_| route.params().with(T::from_map))
}

/// Returns a raw key-value map of the URL search query.
#[track_caller]
pub fn use_query_map() -> Memo<ParamsMap> {
    use_router().inner.location.query
}

/// Returns the current URL search query, parsed into the given type, or an error.
#[track_caller]
pub fn use_query<T>() -> Memo<Result<T, ParamsError>>
where
    T: Params + PartialEq,
{
    let router = use_router();
    create_memo(move |_| router.inner.location.query.with(|m| T::from_map(m)))
}

/// Resolves the given path relative to the current route.
#[track_caller]
pub fn use_resolved_path(
    path: impl Fn() -> String + 'static,
) -> Memo<Option<String>> {
    let route = use_route();

    create_memo(move |_| {
        let path = path();
        if path.starts_with('/') {
            Some(path)
        } else {
            route.resolve_path_tracked(&path)
        }
    })
}

/// Returns a function that can be used to navigate to a new route.
///
/// This should only be called on the client; it does nothing during
/// server rendering.
///
/// ```rust
/// # use leptos::{request_animation_frame, create_runtime};
/// # let runtime = create_runtime();
/// # if false { // can't actually navigate, no <Router/>
/// let navigate = leptos_router::use_navigate();
/// navigate("/", Default::default());
/// # }
/// # runtime.dispose();
/// ```
#[track_caller]
pub fn use_navigate() -> impl Fn(&str, NavigateOptions) + Clone {
    let router = use_router();
    move |to, options| {
        let router = Rc::clone(&router.inner);
        let to = to.to_string();
        if cfg!(any(feature = "csr", feature = "hydrate")) {
            request_animation_frame(move || {
                #[allow(unused_variables)]
                if let Err(e) = router.navigate_from_route(&to, &options) {
                    leptos::logging::debug_warn!("use_navigate error: {e:?}");
                }
            });
        } else {
            leptos::logging::warn!(
                "The navigation function returned by `use_navigate` should \
                 not be called during server rendering."
            );
        }
    }
}

/// Returns a signal that tells you whether you are currently navigating backwards.
pub(crate) fn use_is_back_navigation() -> ReadSignal<bool> {
    let router = use_router();
    router.inner.is_back.read_only()
}

/// Resolves a redirect location to an (absolute) URL.
pub(crate) fn resolve_redirect_url(loc: &str) -> Option<web_sys::Url> {
    let origin = match window().location().origin() {
        Ok(origin) => origin,
        Err(e) => {
            leptos::logging::error!("Failed to get origin: {:#?}", e);
            return None;
        }
    };

    // TODO: Use server function's URL as base instead.
    let base = origin;

    match web_sys::Url::new_with_base(loc, &base) {
        Ok(url) => Some(url),
        Err(e) => {
            leptos::logging::error!(
                "Invalid redirect location: {}",
                e.as_string().unwrap_or_default(),
            );
            None
        }
    }
}
