use std::{any::Any, fmt::Debug, future::Future, rc::Rc};

use leptos_reactive::{create_resource, debug_warn, Memo, Resource, Scope};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{use_location, use_params_map, use_route, Location, ParamsMap, PinnedFuture, Url};

// SSR and CSR both do the work in their own environment and return it as a resource
#[cfg(not(feature = "hydrate"))]
pub fn use_loader<T>(cx: Scope) -> Resource<(ParamsMap, Url), T>
where
    T: Clone + Debug + Serialize + DeserializeOwned + 'static,
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
    let url = move || Url {
        origin: String::default(),
        pathname: location.pathname.get(),
        search: location.search.get(),
        hash: location.hash.get(),
    };

    let loader = loader.data.clone();

    create_resource(
        cx,
        move || (params.get(), url()),
        move |(params, url)| {
            let loader = loader.clone();
            async move {
                let any_data = (loader.clone())(cx, params, url).await;
                any_data.downcast_ref::<T>().cloned().unwrap_or_else(|| {
                    debug_warn!(
                        "use_loader() could not downcast to {:?}",
                        std::any::type_name::<T>(),
                    );
                    panic!()
                })
            }
        },
    )
}

// In hydration mode, only run the loader on the server
#[cfg(feature = "hydrate")]
pub fn use_loader<T>(cx: Scope) -> Resource<(ParamsMap, Url), T>
where
    T: Clone + Debug + Serialize + DeserializeOwned + 'static,
{
    use wasm_bindgen::{JsCast, UnwrapThrowExt};

    let route = use_route(cx);
    let params = use_params_map(cx);

    let location = use_location(cx);
    let url = move || Url {
        origin: String::default(),
        pathname: location.pathname.get(),
        search: location.search.get(),
        hash: location.hash.get(),
    };

    create_resource(
        cx,
        move || (params.get(), url()),
        move |(params, url)| async move {
            let route = use_route(cx);
            let mut opts = web_sys::RequestInit::new();
            opts.method("GET");
            let url = route.path();

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
            let decoded = window.atob(&text).unwrap_throw();
            bincode::deserialize(&decoded.as_bytes()).unwrap_throw()
            //serde_json::from_str(&json.as_string().unwrap_throw()).unwrap_throw()
        },
    )
}

#[derive(Clone)]
pub struct Loader {
    #[cfg(not(feature = "hydrate"))]
    pub(crate) data: Rc<dyn Fn(Scope, ParamsMap, Url) -> PinnedFuture<Box<dyn Any>>>,
}

impl<F, Fu, T> From<F> for Loader
where
    F: Fn(Scope, ParamsMap, Url) -> Fu + 'static,
    Fu: Future<Output = T> + 'static,
    T: Any + 'static,
{
    #[cfg(not(feature = "hydrate"))]
    fn from(f: F) -> Self {
        let wrapped_fn = move |cx, params, url| {
            let res = f(cx, params, url);
            Box::pin(async move { Box::new(res.await) as Box<dyn Any> })
                as PinnedFuture<Box<dyn Any>>
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
