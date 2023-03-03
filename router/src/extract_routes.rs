use crate::{Branch, RouterIntegrationContext, ServerIntegration, SsrMode};
use leptos::*;
use std::{cell::RefCell, rc::Rc};

/// Context to contain all possible routes.
#[derive(Clone, Default, Debug)]
pub struct PossibleBranchContext(pub(crate) Rc<RefCell<Vec<Branch>>>);

/// Generates a list of all routes this application could possibly serve. This returns the raw routes in the leptos_router
/// format. Odds are you want `generate_route_list()` from either the actix, axum, or viz integrations if you want
/// to work with their router
pub fn generate_route_list_inner<IV>(
    app_fn: impl FnOnce(Scope) -> IV + 'static,
) -> Vec<(String, SsrMode)>
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
                let pattern =
                    branch.routes.last().map(|route| route.pattern.clone());
                pattern.map(|pattern| (pattern, mode))
            })
            .collect()
    })
}
