use crate::{
    Branch, Method, RouterIntegrationContext, ServerIntegration, SsrMode,
};
use leptos::*;
use std::{any::Any, cell::RefCell, collections::HashSet, rc::Rc, sync::Arc};

/// Context to contain all possible routes.
#[derive(Clone, Default, Debug)]
pub struct PossibleBranchContext {
    pub(crate) ui: Rc<RefCell<Vec<Branch>>>,
    pub(crate) api: Rc<RefCell<Vec<ApiRouteListing>>>
}

#[derive(Clone, Debug)]
/// A route that this application can serve.
pub enum PossibleRouteListing {
    View(RouteListing),
    Api(ApiRouteListing)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
/// Route listing for a component-based view.
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

#[derive(Clone)]
/// Route listing for an API route.
pub struct ApiRouteListing {
    path: String,
    methods: Option<HashSet<Method>>,
    // this will be downcast by the implementation
    handler: Arc<dyn Any>
}

impl std::fmt::Debug for ApiRouteListing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiRouteListing").field("path", &self.path).field("methods", &self.methods).finish()
    }
}

impl ApiRouteListing {
    /// Create an API route listing from its parts.
    pub fn new<T: 'static>(
        path: impl ToString,
        handler: T
    ) -> Self {
        Self {
            path: path.to_string(),
            methods: None,
            handler: Arc::new(handler)
        }
    }

    /// Create an API route listing from its parts.
    pub fn new_with_methods<T: 'static>(
        path: impl ToString,
        methods: impl IntoIterator<Item = Method>,
        handler: T
    ) -> Self {
        Self {
            path: path.to_string(),
            methods: Some(methods.into_iter().collect()),
            handler: Arc::new(handler)
        }
    }

    /// The path this route handles.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The HTTP request methods this path can handle.
    pub fn methods(&self) -> impl Iterator<Item = Method> + '_ {
        self.methods.iter().flatten().copied()
    }

    /// The handler for a request at this route
    pub fn handler<T: 'static>(&self) -> Option<&T> {
        self.handler.downcast_ref::<T>()
    }
}

/// Generates a list of all routes this application could possibly serve. This returns the raw routes in the leptos_router
/// format. Odds are you want `generate_route_list()` from either the actix, axum, or viz integrations if you want
/// to work with their router
pub fn generate_route_list_inner<IV>(
    app_fn: impl FnOnce(Scope) -> IV + 'static,
) -> (Vec<RouteListing>, Vec<ApiRouteListing>)
where
    IV: IntoView + 'static,
{
    let runtime = create_runtime();
    run_scope(runtime, move |cx| {
        let integration = ServerIntegration {
            path: "http://leptos.rs/".to_string(),
        };

        provide_context(cx, RouterIntegrationContext::new(integration));
        let branches = PossibleBranchContext::default();
        provide_context(cx, branches.clone());

        leptos::suppress_resource_load(true);
        _ = app_fn(cx).into_view(cx);
        leptos::suppress_resource_load(false);

        let ui_branches = branches.ui.borrow();
        let ui = ui_branches
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
            .collect();
        let api_branches = branches.api.borrow();
        let api = api_branches
            .iter()
            .cloned()
            .collect();
        (ui, api)
    })
}
