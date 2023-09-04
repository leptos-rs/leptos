use crate::{
    Branch, Method, RouterIntegrationContext, ServerIntegration, SsrMode,
    StaticData, StaticParamsMap, StaticPath, StaticRenderContext,
};
use leptos::*;
use leptos_meta::MetaContext;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::Path,
    rc::Rc,
};

/// Context to contain all possible routes.
#[derive(Clone, Default, Debug)]
pub struct PossibleBranchContext(pub(crate) Rc<RefCell<Vec<Branch>>>);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
/// A route that this application can serve.
pub struct RouteListing {
    path: String,
    mode: SsrMode,
    methods: HashSet<Method>,
    static_rendered: bool,
}

impl RouteListing {
    /// Create a route listing from its parts.
    pub fn new(
        path: impl ToString,
        mode: SsrMode,
        methods: impl IntoIterator<Item = Method>,
        static_rendered: bool,
    ) -> Self {
        Self {
            path: path.to_string(),
            mode,
            methods: methods.into_iter().collect(),
            static_rendered,
        }
    }

    /// The path this route handles.
    pub fn path(&self) -> &str {
        &self.path
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
    pub fn static_rendered(&self) -> bool {
        self.static_rendered
    }
}

/// Generates a list of all routes this application could possibly serve. This returns the raw routes in the leptos_router
/// format. Odds are you want `generate_route_list()` from either the actix, axum, or viz integrations if you want
/// to work with their router
pub async fn generate_route_list_inner<IV>(
    options: LeptosOptions,
    app_fn: impl Fn() -> IV + 'static + Clone,
    additional_context: impl Fn() + 'static + Clone,
    static_context: Option<StaticRenderContext>,
) -> Vec<RouteListing>
where
    IV: IntoView + 'static,
{
    let static_context = static_context.unwrap_or_default();
    let runtime = create_runtime();

    let integration = ServerIntegration {
        path: "http://leptos.rs/".to_string(),
    };

    provide_context(RouterIntegrationContext::new(integration));
    let branches = PossibleBranchContext::default();
    provide_context(branches.clone());

    leptos::suppress_resource_load(true);
    _ = app_fn().into_view();
    leptos::suppress_resource_load(false);

    let branches = branches.0.borrow();
    let mut routes_map: HashMap<String, Option<StaticData>> = HashMap::new();
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
                .map(|route| (route.key.static_render, route.pattern.clone()));
            for route in branch.routes.iter() {
                routes_map.insert(
                    route.pattern.to_string(),
                    route.key.static_data.clone(),
                );
            }
            route.map(|(static_rendered, path)| RouteListing {
                path,
                mode,
                methods: methods.clone(),
                static_rendered,
            })
        })
        .collect::<Vec<_>>();

    let routes_map = routes_map.into_iter().collect::<Vec<_>>();
    let mut static_data: HashMap<String, StaticParamsMap> = HashMap::new();
    for (key, value) in routes_map {
        match value {
            Some(value) => {
                static_data.insert(key, value.as_ref()(&static_context).await)
            }
            None => static_data.insert(key, StaticParamsMap::default()),
        };
    }

    let static_routes = routes
        .iter()
        .filter(|route| route.static_rendered)
        .collect::<Vec<_>>();
    // TODO: maybe make this concurrent in some capacity
    for route in static_routes {
        let mut path = StaticPath::new(&route.path);
        for p in path.parents().into_iter().rev() {
            match static_data.get(p.path()) {
                Some(data) => path.add_params(data),
                None => {}
            }
        }
        match static_data.get(path.path()) {
            Some(data) => path.add_params(data),
            None => {}
        }
        // find all parent routes and resolve all static_data
        // grab each parent route's static_data and add the parameters to the path
        for path in path.into_paths() {
            let url = format!("http://leptos{}", path);
            let app = {
                let app_fn = app_fn.clone();
                move || {
                    provide_context(RouterIntegrationContext::new(
                        ServerIntegration { path: url },
                    ));
                    provide_context(MetaContext::new());
                    (app_fn)().into_view()
                }
            };
            let (stream, runtime) = leptos::ssr::render_to_stream_in_order_with_prefix_undisposed_with_context(app, move || "".into(), additional_context.clone());
            let html = leptos_integration_utils::build_async_response(
                stream, &options, runtime,
            )
            .await;
            let path =
                Path::new(&options.site_root).join(format!(".{path}.html"));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, html).unwrap();
        }
    }
    runtime.dispose();
    routes
}
