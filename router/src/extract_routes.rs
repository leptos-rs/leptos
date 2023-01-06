use leptos::*;
use std::{cell::RefCell, rc::Rc};

use crate::{Branch, RouterIntegrationContext, ServerIntegration};

/// Context to contain all possible routes.
#[derive(Clone, Default, Debug)]
pub struct PossibleBranchContext(pub(crate) Rc<RefCell<Vec<Branch>>>);

/// Generates a list of all routes this application could possibly serve.
#[cfg(feature = "ssr")]
pub fn generate_route_list<IV>(app_fn: impl FnOnce(Scope) -> IV + 'static) -> Vec<String>
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

        let _ = app_fn(cx).into_view(cx);

        let branches = branches.0.borrow();
        branches
            .iter()
            .flat_map(|branch| branch.routes.last().map(|route| route.pattern.clone()))
            .collect()
    })
}
