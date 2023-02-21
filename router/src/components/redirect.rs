use crate::{use_navigate, use_resolved_path, NavigateOptions};
use leptos::{
    component, provide_context, signal_prelude::*, use_context, IntoView, Scope,
};
use std::rc::Rc;

/// Redirects the user to a new URL, whether on the client side or on the server
/// side. If rendered on the server, this sets a `302` status code and sets a `Location`
/// header. If rendered in the browser, it uses client-side navigation to redirect.
/// In either case, it resolves the route relative to the current route. (To use
/// an absolute path, prefix it with `/`).
///
/// **Note**: Support for server-side redirects is provided by the server framework
/// integrations (`leptos_actix`, `leptos_axum`, and `leptos_viz`). If you’re not using one of those
/// integrations, you should manually provide a way of redirecting on the server
/// using [provide_server_redirect].
#[component]
pub fn Redirect<P>(
    cx: Scope,
    /// The relative path to which the user should be redirected.
    path: P,
    /// Navigation options to be used on the client side.
    #[prop(optional)]
    options: Option<NavigateOptions>,
) -> impl IntoView
where
    P: std::fmt::Display + 'static,
{
    // resolve relative path
    let path = use_resolved_path(cx, move || path.to_string());
    let path = path.get().unwrap_or_else(|| "/".to_string());

    // redirect on the server
    if let Some(redirect_fn) = use_context::<ServerRedirectFunction>(cx) {
        (redirect_fn.f)(&path);
    }

    // redirect on the client
    let navigate = use_navigate(cx);
    navigate(&path, options.unwrap_or_default())
}

/// Wrapping type for a function provided as context to allow for
/// server-side redirects. See [provide_server_redirect]
/// and [Redirect].
#[derive(Clone)]
pub struct ServerRedirectFunction {
    f: Rc<dyn Fn(&str)>,
}

impl std::fmt::Debug for ServerRedirectFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerRedirectFunction").finish()
    }
}

/// Provides a function that can be used to redirect the user to another
/// absolute path, on the server. This should set a `302` status code and an
/// appropriate `Location` header.
pub fn provide_server_redirect(cx: Scope, handler: impl Fn(&str) + 'static) {
    provide_context(
        cx,
        ServerRedirectFunction {
            f: Rc::new(handler),
        },
    )
}
