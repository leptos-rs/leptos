use std::{any::Any, fmt::Debug, future::Future, rc::Rc};

use leptos::*;
use serde_lite::{Deserialize, Serialize};

use crate::{use_location, use_params_map, use_route, ParamsMap, PinnedFuture, Url};

// SSR and CSR both do the work in their own environment and return it as a resource
#[cfg(not(feature = "hydrate"))]
pub fn use_loader<T>(cx: Scope) -> Resource<(ParamsMap, Url), T>
where
    T: Clone + Debug + Serialize + Deserialize + 'static,
{
    let route = use_route(cx);
    let params = use_params_map(cx);
    let loader = route.loader().clone().unwrap_or_else(|| {
        debug_warn!(
            "use_loader() called on a route without a loader: {:?}",
            route.path()
        );
        panic!()
    });

    let location = use_location(cx);
    let route = use_route(cx);
    let url = move || Url {
        origin: String::default(), // don't care what the origin is for this purpose
        pathname: route.path().into(), // only use this route path, not all matched routes
        search: location.search.get(), // reload when any of query string changes
        hash: String::default(),   // hash is only client-side, shouldn't refire
    };

    let loader = loader.data.clone();

    create_resource(
        cx,
        move || (params.get(), url()),
        move |(params, url)| {
            let loader = loader.clone();
            async move {
                let any_data = (loader.clone())(cx, params, url).await;
                any_data
                    .as_any()
                    .downcast_ref::<T>()
                    .cloned()
                    .unwrap_or_else(|| {
                        panic!(
                            "use_loader() could not downcast to {:?}",
                            std::any::type_name::<T>(),
                        )
                    })
            }
        },
    )
}

// In hydration mode, only run the loader on the server
#[cfg(feature = "hydrate")]
pub fn use_loader<T>(cx: Scope) -> Resource<(ParamsMap, Url), T>
where
    T: Clone + Debug + Serialize + Deserialize + 'static,
{
    use wasm_bindgen::{JsCast, UnwrapThrowExt};

    use crate::use_query_map;

    let route = use_route(cx);
    let params = use_params_map(cx);

    let location = use_location(cx);
    let route = use_route(cx);
    let url = move || Url {
        origin: String::default(), // don't care what the origin is for this purpose
        pathname: route.path().into(), // only use this route path, not all matched routes
        search: location.search.get(), // reload when any of query string changes
        hash: String::default(),   // hash is only client-side, shouldn't refire
    };

    create_resource(
        cx,
        move || (params.get(), url()),
        move |(params, url)| async move {
            let route = use_route(cx);
            let query = use_query_map(cx);

            let mut opts = web_sys::RequestInit::new();
            opts.method("GET");
            let url = format!("{}{}", route.path(), query.get().to_query_string());

            let request = web_sys::Request::new_with_str_and_init(&url, &opts).unwrap_throw();
            request
                .headers()
                .set("Accept", "application/json")
                .unwrap_throw();

            let window = web_sys::window().unwrap_throw();
            let resp_value =
                wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
                    .await
                    .unwrap_throw();
            let resp = resp_value.unchecked_into::<web_sys::Response>();
            let text = wasm_bindgen_futures::JsFuture::from(resp.text().unwrap_throw())
                .await
                .unwrap_throw()
                .as_string()
                .unwrap_throw();
            //let decoded = window.atob(&text).unwrap_throw();
            //bincode::deserialize(&decoded.as_bytes()).unwrap_throw()
            //serde_json::from_str(&text.as_string().unwrap_throw()).unwrap_throw()
            let intermediate =
                serde_json::from_str(&text).expect_throw("couldn't deserialize loader data");
            T::deserialize(&intermediate).expect_throw(
                "couldn't deserialize loader data from serde-lite intermediate format",
            )
        },
    )
}

pub trait AnySerialize {
    fn serialize(&self) -> Option<String>;

    fn as_any(&self) -> &dyn Any;
}

impl<T> AnySerialize for T
where
    T: Any + Serialize + 'static,
{
    fn serialize(&self) -> Option<String> {
        let intermediate = self.serialize().ok()?;
        serde_json::to_string(&intermediate).ok()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone)]
pub struct Loader {
    #[allow(clippy::type_complexity)]
    #[cfg(not(feature = "hydrate"))]
    pub(crate) data: Rc<dyn Fn(Scope, ParamsMap, Url) -> PinnedFuture<Box<dyn AnySerialize>>>,
}

impl Loader {
    #[cfg(not(feature = "hydrate"))]
    pub fn call_loader(&self, cx: Scope) -> PinnedFuture<Box<dyn AnySerialize>> {
        let route = use_route(cx);
        let params = use_params_map(cx).get();
        let location = use_location(cx);
        let url = Url {
            origin: String::default(), // don't care what the origin is for this purpose
            pathname: route.path().into(), // only use this route path, not all matched routes
            search: location.search.get(), // reload when any of query string changes
            hash: String::default(),   // hash is only client-side, shouldn't refire
        };
        (self.data)(cx, params, url)
    }
}

impl<F, Fu, T> From<F> for Loader
where
    F: Fn(Scope, ParamsMap, Url) -> Fu + 'static,
    Fu: Future<Output = T> + 'static,
    T: Any + Serialize + 'static,
{
    #[cfg(not(feature = "hydrate"))]
    fn from(f: F) -> Self {
        let wrapped_fn = move |cx, params, url| {
            let res = f(cx, params, url);
            Box::pin(async move { Box::new(res.await) as Box<dyn AnySerialize> })
                as PinnedFuture<Box<dyn AnySerialize>>
        };

        Self {
            data: Rc::new(wrapped_fn),
        }
    }

    #[cfg(feature = "hydrate")]
    fn from(f: F) -> Self {
        Self {}
    }
}

impl std::fmt::Debug for Loader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Loader").finish()
    }
}

#[cfg(feature = "ssr")]
pub async fn loader_to_json(view: impl Fn(Scope) -> String + 'static) -> Option<String> {
    let (data, _, disposer) = run_scope_undisposed(move |cx| async move {
        let _shell = view(cx);

        let mut route = use_context::<crate::RouteContext>(cx)?;
        // get the innermost route matched by this path
        while let Some(child) = route.child() {
            route = child;
        }
        let data = route
            .loader()
            .as_ref()
            .map(|loader| loader.call_loader(cx))?;

        data.await.serialize()
    });
    let data = data.await;
    disposer.dispose();
    data
}
