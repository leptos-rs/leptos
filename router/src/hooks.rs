use crate::{
    Location, NavigateOptions, NavigationError, Params, ParamsError, ParamsMap,
    RouteContext, RouterContext,
};
use leptos::{create_memo, signal_prelude::*, use_context, Memo, Scope};
use std::{borrow::Cow, rc::Rc, str::FromStr};

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
/// pub fn SimpleQueryCounter(cx: Scope) -> impl IntoView {
///     let (count, set_count) = create_query_signal::<i32>(cx, "count");
///     let clear = move |_| set_count.set(None);
///     let decrement =
///         move |_| set_count.set(Some(count.get().unwrap_or(0) - 1));
///     let increment =
///         move |_| set_count.set(Some(count.get().unwrap_or(0) + 1));
///
///     view! { cx,
///         <div>
///             <button on:click=clear>"Clear"</button>
///             <button on:click=decrement>"-1"</button>
///             <span>"Value: " {move || count.get().unwrap_or(0)} "!"</span>
///             <button on:click=increment>"+1"</button>
///         </div>
///     }
/// }
/// ```
pub fn create_query_signal<T>(
    cx: Scope,
    key: impl Into<Cow<'static, str>>,
) -> (Memo<Option<T>>, SignalSetter<Option<T>>)
where
    T: FromStr + ToString + PartialEq,
{
    let key = key.into();
    let query_map = use_query_map(cx);
    let navigate = use_navigate(cx);
    let route = use_route(cx);

    let get = create_memo(cx, {
        let key = key.clone();
        move |_| {
            query_map
                .with(|map| map.get(&key).and_then(|value| value.parse().ok()))
        }
    });

    let set = SignalSetter::map(cx, move |value: Option<T>| {
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
        let path = route.path();
        let new_url = format!("{path}{qs}");
        let _ = navigate(&new_url, NavigateOptions::default());
    });

    (get, set)
}

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
///
/// ## Panics
/// `use_navigate` can sometimes panic due to a `BorrowMut` runtime error
/// if it is called immediately during routing/rendering. In this case, you should
/// wrap it in [`request_animation_frame`](leptos::request_animation_frame)
/// to delay it until that routing process is complete.
/// ```rust
/// # use leptos::{request_animation_frame,create_scope,create_runtime};
/// # create_scope(create_runtime(), |cx| {
/// # if false { // can't actually navigate, no <Router/>
/// let navigate = leptos_router::use_navigate(cx);
/// request_animation_frame(move || {
///     _ = navigate("/", Default::default());
/// });
/// # }
/// # });
/// ```
pub fn use_navigate(
    cx: Scope,
) -> impl Fn(&str, NavigateOptions) -> Result<(), NavigationError> {
    let router = use_router(cx);
    move |to, options| {
        Rc::clone(&router.inner).navigate_from_route(to, &options)
    }
}
///
/// Returns a signal that tells you whether you are currently navigating backwards.
pub(crate) fn use_is_back_navigation(cx: Scope) -> ReadSignal<bool> {
    let router = use_router(cx);
    router.inner.is_back.read_only()
}
