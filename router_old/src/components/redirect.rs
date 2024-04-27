use crate::{use_navigate, use_resolved_path, NavigateOptions};
use leptos::{
    component, provide_context, signal_prelude::*, use_context, IntoView,
};
use std::rc::Rc;

/// Redirects the user to a new URL, whether on the client side or on the server
/// side. If rendered on the server, this sets a `302` status code and sets a `Location`
/// header. If rendered in the browser, it uses client-side navigation to redirect.
/// In either case, it resolves the route relative to the current route. (To use
/// an absolute path, prefix it with `/`).
///
/// **Note**: Support for server-side redirects is provided by the server framework
/// integrations ([`leptos_actix`] and [`leptos_axum`]. If you’re not using one of those
/// integrations, you should manually provide a way of redirecting on the server
/// using [`provide_server_redirect`].
///
/// [`leptos_actix`]: <https://docs.rs/leptos_actix/>
/// [`leptos_axum`]: <https://docs.rs/leptos_axum/>
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
#[component]
pub fn Redirect<P>(
    /// The relative path to which the user should be redirected.
    path: P,
    /// Navigation options to be used on the client side.
    #[prop(optional)]
    #[allow(unused)]
    options: Option<NavigateOptions>,
) -> impl IntoView
where
    P: core::fmt::Display + 'static,
{
    // resolve relative path
    let path = use_resolved_path(move || path.to_string());
    let path = path.get_untracked().unwrap_or_else(|| "/".to_string());

    // redirect on the server
    if let Some(redirect_fn) = use_context::<ServerRedirectFunction>() {
        (redirect_fn.f)(&path);
    }
    // redirect on the client
    else {
        #[allow(unused)]
        let navigate = use_navigate();
        #[cfg(any(feature = "csr", feature = "hydrate"))]
        navigate(&path, options.unwrap_or_default());
        #[cfg(not(any(feature = "csr", feature = "hydrate")))]
        {
            leptos::logging::debug_warn!(
                "<Redirect/> is trying to redirect without \
                 `ServerRedirectFunction` being provided. (If you’re getting \
                 this on initial server start-up, it’s okay to ignore. It \
                 just means that your root route is a redirect.)"
            );
        }
    }
}

/// Wrapping type for a function provided as context to allow for
/// server-side redirects. See [`provide_server_redirect`]
/// and [`Redirect`].
#[derive(Clone)]
pub struct ServerRedirectFunction {
    f: Rc<dyn Fn(&str)>,
}

impl core::fmt::Debug for ServerRedirectFunction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ServerRedirectFunction").finish()
    }
}

/// Provides a function that can be used to redirect the user to another
/// absolute path, on the server. This should set a `302` status code and an
/// appropriate `Location` header.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
pub fn provide_server_redirect(handler: impl Fn(&str) + 'static) {
    provide_context(ServerRedirectFunction {
        f: Rc::new(handler),
    })
}
