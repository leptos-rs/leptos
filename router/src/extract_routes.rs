use crate::{
    Branch, Method, RouterIntegrationContext, ServerIntegration, SsrMode,
};
use leptos::*;
use std::{cell::RefCell, collections::HashSet, rc::Rc};

/// Context to contain all possible routes.
#[derive(Clone, Default, Debug)]
pub struct PossibleBranchContext(pub(crate) Rc<RefCell<Vec<Branch>>>);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
/// A route that this application can serve.
pub struct RouteListing {
    path: String,
    mode: SsrMode,
    methods: HashSet<Method>,
}

impl RouteListing {
    /// Create a route listing from its parts.
    pub fn new(
        path: impl ToString,
        mode: SsrMode,
        methods: impl IntoIterator<Item = Method>,
    ) -> Self {
        Self {
            path: path.to_string(),
            mode,
            methods: methods.into_iter().collect(),
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
}

/// Generates a list of all routes this application could possibly serve. This returns the raw routes in the leptos_router
/// format. Odds are you want `generate_route_list()` from either the actix, axum, or viz integrations if you want
/// to work with their router
pub fn generate_route_list_inner<IV>(
    app_fn: impl FnOnce(Scope) -> IV + 'static,
) -> Vec<RouteListing>
where
    IV: IntoView + 'static,
{
    let runtime = create_runtime();
    let routes = run_scope(runtime, move |cx| {
        let integration = ServerIntegration {
            path: "http://leptos.rs/".to_string(),
        };

        provide_context(cx, RouterIntegrationContext::new(integration));
        let branches = PossibleBranchContext::default();
        provide_context(cx, branches.clone());

        leptos::suppress_resource_load(true);
        _ = app_fn(cx).into_view(cx);
        leptos::suppress_resource_load(false);

        let branches = branches.0.borrow();
        branches
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
                let pattern =
                    branch.routes.last().map(|route| route.pattern.clone());
                pattern.map(|path| RouteListing {
                    path,
                    mode,
                    methods: methods.clone(),
                })
            })
            .collect()
    });
    runtime.dispose();
    routes
}
