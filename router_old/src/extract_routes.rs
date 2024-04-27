mod test_extract_routes;

use crate::{
    provide_server_redirect, Branch, Method, RouterIntegrationContext,
    ServerIntegration, SsrMode, StaticDataMap, StaticMode, StaticParamsMap,
    StaticPath,
};
use leptos::*;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

/// Context to contain all possible routes.
#[derive(Clone, Default, Debug)]
pub struct PossibleBranchContext(pub(crate) Rc<RefCell<Vec<Branch>>>);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
/// A route that this application can serve.
pub struct RouteListing {
    path: String,
    leptos_path: String,
    mode: SsrMode,
    methods: HashSet<Method>,
    static_mode: Option<StaticMode>,
}

impl RouteListing {
    /// Create a route listing from its parts.
    pub fn new(
        path: impl ToString,
        leptos_path: impl ToString,
        mode: SsrMode,
        methods: impl IntoIterator<Item = Method>,
        static_mode: Option<StaticMode>,
    ) -> Self {
        Self {
            path: path.to_string(),
            leptos_path: leptos_path.to_string(),
            mode,
            methods: methods.into_iter().collect(),
            static_mode,
        }
    }

    /// The path this route handles.
    ///
    /// This should be formatted for whichever web server integegration is being used. (ex: leptos-actix.)
    /// When returned from leptos-router, it matches `self.leptos_path()`.  
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The leptos-formatted path this route handles.
    pub fn leptos_path(&self) -> &str {
        &self.leptos_path
    }

    /// The rendering mode for this path.
    pub fn mode(&self) -> SsrMode {
        self.mode
    }

    /// The HTTP request methods this path can handle.
    pub fn methods(&self) -> impl Iterator<Item = Method> + '_ {
        self.methods.iter().copied()
    }

    /// Whether this route is statically rendered.
    #[inline(always)]
    pub fn static_mode(&self) -> Option<StaticMode> {
        self.static_mode
    }

    /// Build a route statically, will return `Ok(true)` on success or `Ok(false)` when the route
    /// is not marked as statically rendered. All route parameters to use when resolving all paths
    /// to render should be passed in the `params` argument.
    pub async fn build_static<IV>(
        &self,
        options: &LeptosOptions,
        app_fn: impl Fn() -> IV + Send + 'static + Clone,
        additional_context: impl Fn() + Send + 'static + Clone,
        params: &StaticParamsMap,
    ) -> Result<bool, std::io::Error>
    where
        IV: IntoView + 'static,
    {
        match self.static_mode {
            None => Ok(false),
            Some(_) => {
                let mut path = StaticPath::new(&self.leptos_path);
                path.add_params(params);
                for path in path.into_paths() {
                    path.write(
                        options,
                        app_fn.clone(),
                        additional_context.clone(),
                    )
                    .await?;
                }
                Ok(true)
            }
        }
    }
}

/// Generates a list of all routes this application could possibly serve. This returns the raw routes in the leptos_router
/// format. Odds are you want `generate_route_list()` from either the [`actix`] or [`axum`] integrations if you want
/// to work with their router.
///
/// [`actix`]: <https://docs.rs/actix/>
/// [`axum`]: <https://docs.rs/axum/>
pub fn generate_route_list_inner<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
) -> (Vec<RouteListing>, StaticDataMap)
where
    IV: IntoView + 'static,
{
    generate_route_list_inner_with_context(app_fn, || {})
}
/// Generates a list of all routes this application could possibly serve. This returns the raw routes in the leptos_router
/// format. Odds are you want `generate_route_list()` from either the [`actix`] or [`axum`] integrations if you want
/// to work with their router.
///
/// [`actix`]: <https://docs.rs/actix/>
/// [`axum`]: <https://docs.rs/axum/>
pub fn generate_route_list_inner_with_context<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
    additional_context: impl Fn() + 'static + Clone,
) -> (Vec<RouteListing>, StaticDataMap)
where
    IV: IntoView + 'static,
{
    let runtime = create_runtime();

    let branches = get_branches(app_fn, additional_context);
    let branches = branches.0.borrow();

    let mut static_data_map: StaticDataMap = HashMap::new();
    let routes = branches
        .iter()
        .flat_map(|branch| {
            let mode = branch
                .routes
                .iter()
                .map(|route| route.key.ssr_mode)
                .max()
                .unwrap_or_default();
            let methods = branch
                .routes
                .iter()
                .flat_map(|route| route.key.methods)
                .copied()
                .collect::<HashSet<_>>();
            let route = branch
                .routes
                .last()
                .map(|route| (route.key.static_mode, route.pattern.clone()));
            for route in branch.routes.iter() {
                static_data_map.insert(
                    route.pattern.to_string(),
                    route.key.static_params.clone(),
                );
            }
            route.map(|(static_mode, path)| RouteListing {
                leptos_path: path.clone(),
                path,
                mode,
                methods: methods.clone(),
                static_mode,
            })
        })
        .collect::<Vec<_>>();

    runtime.dispose();
    (routes, static_data_map)
}

fn get_branches<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
    additional_context: impl Fn() + 'static + Clone,
) -> PossibleBranchContext
where
    IV: IntoView + 'static,
{
    let integration = ServerIntegration {
        path: "http://leptos.rs/".to_string(),
    };

    provide_context(RouterIntegrationContext::new(integration));
    let branches = PossibleBranchContext::default();
    provide_context(branches.clone());
    // Suppress startup warning about using <Redirect/> without ServerRedirectFunction:
    provide_server_redirect(|_str| ());

    additional_context();

    leptos::suppress_resource_load(true);
    _ = app_fn().into_view();
    leptos::suppress_resource_load(false);

    branches
}
