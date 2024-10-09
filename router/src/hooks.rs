use crate::{
    components::RouterContext,
    location::{Location, Url},
    navigate::NavigateOptions,
    params::{Params, ParamsError, ParamsMap},
};
use leptos::{leptos_dom::helpers::request_animation_frame, oco::Oco};
use reactive_graph::{
    computed::{ArcMemo, Memo},
    owner::{expect_context, use_context},
    signal::{ArcRwSignal, ReadSignal},
    traits::{Get, GetUntracked, ReadUntracked, With, WriteValue},
    wrappers::write::SignalSetter,
};
use std::{
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
};

#[track_caller]
#[deprecated = "This has been renamed to `query_signal` to match Rust naming \
                conventions."]
pub fn create_query_signal<T>(
    key: impl Into<Oco<'static, str>>,
) -> (Memo<Option<T>>, SignalSetter<Option<T>>)
where
    T: FromStr + ToString + PartialEq + Send + Sync,
{
    query_signal(key)
}

#[track_caller]
#[deprecated = "This has been renamed to `query_signal_with_options` to mtch \
                Rust naming conventions."]
pub fn create_query_signal_with_options<T>(
    key: impl Into<Oco<'static, str>>,
    nav_options: NavigateOptions,
) -> (Memo<Option<T>>, SignalSetter<Option<T>>)
where
    T: FromStr + ToString + PartialEq + Send + Sync,
{
    query_signal_with_options(key, nav_options)
}

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
/// use leptos::prelude::*;
/// use leptos_router::hooks::query_signal;
///
/// #[component]
/// pub fn SimpleQueryCounter() -> impl IntoView {
///     let (count, set_count) = query_signal::<i32>("count");
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
pub fn query_signal<T>(
    key: impl Into<Oco<'static, str>>,
) -> (Memo<Option<T>>, SignalSetter<Option<T>>)
where
    T: FromStr + ToString + PartialEq + Send + Sync,
{
    query_signal_with_options::<T>(key, NavigateOptions::default())
}

#[track_caller]
pub fn query_signal_with_options<T>(
    key: impl Into<Oco<'static, str>>,
    nav_options: NavigateOptions,
) -> (Memo<Option<T>>, SignalSetter<Option<T>>)
where
    T: FromStr + ToString + PartialEq + Send + Sync,
{
    static IS_NAVIGATING: AtomicBool = AtomicBool::new(false);

    let mut key: Oco<'static, str> = key.into();
    let query_map = use_query_map();
    let navigate = use_navigate();
    let location = use_location();
    let RouterContext {
        query_mutations, ..
    } = expect_context();

    let get = Memo::new({
        let key = key.clone_inplace();
        move |_| {
            query_map.with(|map| {
                map.get_str(&key).and_then(|value| value.parse().ok())
            })
        }
    });

    let set = SignalSetter::map(move |value: Option<T>| {
        let path = location.pathname.get_untracked();
        let hash = location.hash.get_untracked();
        let qs = location.query.read_untracked().to_query_string();
        let new_url = format!("{path}{qs}{hash}");
        query_mutations
            .write_value()
            .push((key.clone(), value.as_ref().map(ToString::to_string)));

        if !IS_NAVIGATING.load(Ordering::Relaxed) {
            IS_NAVIGATING.store(true, Ordering::Relaxed);
            request_animation_frame({
                let navigate = navigate.clone();
                let nav_options = nav_options.clone();
                move || {
                    navigate(&new_url, nav_options.clone());
                    IS_NAVIGATING.store(false, Ordering::Relaxed)
                }
            })
        }
    });

    (get, set)
}

#[track_caller]
pub(crate) fn has_router() -> bool {
    use_context::<RouterContext>().is_some()
}

/*
/// Returns the current [`RouterContext`], containing information about the router's state.
#[track_caller]
pub(crate) fn use_router() -> RouterContext {
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
*/

/// Returns the current [`Location`], which contains reactive variables
#[track_caller]
pub fn use_location() -> Location {
    let RouterContext { location, .. } =
        use_context().expect("Tried to access Location outside a <Router>.");
    location
}

pub(crate) type RawParamsMap = ArcMemo<ParamsMap>;

#[track_caller]
fn use_params_raw() -> RawParamsMap {
    use_context().expect(
        "Tried to access params outside the context of a matched <Route>.",
    )
}

/// Returns a raw key-value map of route params.
#[track_caller]
pub fn use_params_map() -> Memo<ParamsMap> {
    use_params_raw().into()
}

/// Returns the current route params, parsed into the given type, or an error.
#[track_caller]
pub fn use_params<T>() -> Memo<Result<T, ParamsError>>
where
    T: Params + PartialEq + Send + Sync + 'static,
{
    // TODO this can be optimized in future to map over the signal, rather than cloning
    let params = use_params_raw();
    Memo::new(move |_| params.with(T::from_map))
}

#[track_caller]
fn use_url_raw() -> ArcRwSignal<Url> {
    use_context().unwrap_or_else(|| {
        let RouterContext { current_url, .. } = use_context().expect(
            "Tried to access reactive URL outside a <Router> component.",
        );
        current_url
    })
}

#[track_caller]
pub fn use_url() -> ReadSignal<Url> {
    use_url_raw().read_only().into()
}

/// Returns a raw key-value map of the URL search query.
#[track_caller]
pub fn use_query_map() -> Memo<ParamsMap> {
    let url = use_url_raw();
    Memo::new(move |_| url.with(|url| url.search_params().clone()))
}

/// Returns the current URL search query, parsed into the given type, or an error.
#[track_caller]
pub fn use_query<T>() -> Memo<Result<T, ParamsError>>
where
    T: Params + PartialEq + Send + Sync + 'static,
{
    let url = use_url_raw();
    Memo::new(move |_| url.with(|url| T::from_map(url.search_params())))
}

#[derive(Debug, Clone)]
pub(crate) struct Matched(pub ArcMemo<String>);

/// Resolves the given path relative to the current route.
#[track_caller]
pub(crate) fn use_resolved_path(
    path: impl Fn() -> String + Send + Sync + 'static,
) -> ArcMemo<Option<String>> {
    let router = use_context::<RouterContext>()
        .expect("called use_resolved_path outside a <Router>");
    // TODO make this work with flat routes too?
    let matched = use_context::<Matched>().map(|n| n.0);
    ArcMemo::new(move |_| {
        let path = path();
        if path.starts_with('/') {
            Some(path)
        } else {
            router
                .resolve_path(
                    &path,
                    matched.as_ref().map(|n| n.get()).as_deref(),
                )
                .map(|n| n.to_string())
        }
    })
}

/// Returns a function that can be used to navigate to a new route.
///
/// This should only be called on the client; it does nothing during
/// server rendering.
///
/// ```rust
/// # if false { // can't actually navigate, no <Router/>
/// let navigate = leptos_router::hooks::use_navigate();
/// navigate("/", Default::default());
/// # }
/// ```
#[track_caller]
pub fn use_navigate() -> impl Fn(&str, NavigateOptions) + Clone {
    let cx = use_context::<RouterContext>()
        .expect("You cannot call `use_navigate` outside a <Router>.");
    move |path: &str, options: NavigateOptions| cx.navigate(path, options)
}

/*
/// Returns a signal that tells you whether you are currently navigating backwards.
pub(crate) fn use_is_back_navigation() -> ReadSignal<bool> {
    let router = use_router();
    router.inner.is_back.read_only()
}
*/

/* TODO check how this is used in 0.6 and use it
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
*/
