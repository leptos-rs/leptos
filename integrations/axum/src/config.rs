//! Provides a builder and implementation for wholesale configuration of [`axum::Router`].

use crate::{generate_route_list, ErrorHandler, LeptosRoutes};
#[cfg(feature = "default")]
use crate::{site_pkg_dir_service, site_pkg_dir_service_route_path};
use axum::{extract::FromRef, Router};
use leptos::{config::LeptosOptions, IntoView};

pub(crate) mod traits {
    //! Provides the trait for [`RouterConfiguration`].
    use super::*;

    /// This trait is the means to provide different kinds of configuration to the different sets of
    /// generics that may be configured for the underlying concrete type.
    ///
    /// This trait is sealed and cannot be implemented for callers as this exists to encapsulate internal
    /// implementation details.
    #[diagnostic::on_unimplemented(
        label = "incomplete `RouterConfiguration`",
        note = "ensure that `.app()`, `.shell()`, and `.state()` are provided \
                with the required values"
    )]
    pub trait RouterConfiguration<S>: crate::private::Sealed
    where
        S: Clone + Send + Sync + 'static,
        LeptosOptions: FromRef<S>,
    {
        /// Apply the configuration onto the [`Router`].
        fn apply(self, router: Router<S>) -> Router<()>;
    }
}

/// A configuration builder to setup a Leptos app onto a [`axum::Router`].
#[derive(Clone)]
pub struct RouterConfiguration<APP, CX = (), SH = (), S = LeptosOptions> {
    pub(super) app_fn: Option<APP>,
    pub(super) shell: Option<SH>,
    pub(super) state: Option<S>,
    pub(super) extra_cx: Option<CX>,
    #[cfg(feature = "default")]
    pub(super) serve_site_pkg: bool,
    pub(super) error_handler: bool,
}

impl<APP> Default for RouterConfiguration<APP, (), (), LeptosOptions> {
    fn default() -> Self {
        Self {
            app_fn: None,
            shell: None,
            state: None,
            extra_cx: None,
            #[cfg(feature = "default")]
            serve_site_pkg: false,
            error_handler: false,
        }
    }
}

impl<APP> RouterConfiguration<APP> {
    /// Create a new configuration with all toggles set to the recommended value.
    ///
    /// This typically enables the site pkg handler and the error handler for the fallback.
    pub fn new() -> Self {
        Self {
            app_fn: None,
            shell: None,
            state: None,
            extra_cx: None,
            #[cfg(feature = "default")]
            serve_site_pkg: true,
            error_handler: true,
        }
    }
}

impl<APP, CX, SH, S> RouterConfiguration<APP, CX, SH, S> {
    /// Provide the app to the configuration.
    pub fn app<IV>(mut self, app: APP) -> Self
    where
        APP: Fn() -> IV + Clone + Send + Sync + 'static,
        IV: IntoView + 'static,
    {
        self.app_fn = Some(app);
        self
    }

    /// Toggle for the site pkg `ServeDir` service.
    #[cfg(feature = "default")]
    pub fn serve_site_pkg(mut self, v: bool) -> Self {
        self.serve_site_pkg = v;
        self
    }

    /// Toggle for the fallback error handler
    pub fn error_handler(mut self, v: bool) -> Self {
        self.error_handler = v;
        self
    }

    /// Configure a new shell with a different type for state; this will reset the state parameter.
    pub fn shell<SH2, S2, IV>(
        self,
        shell: SH2,
    ) -> RouterConfiguration<APP, CX, SH2, S2>
    where
        SH2: Fn(S2) -> IV + Clone + Send + Sync + 'static,
        S2: Clone + Send + Sync + 'static,
        LeptosOptions: FromRef<S2>,
        IV: IntoView + 'static,
    {
        RouterConfiguration {
            app_fn: self.app_fn,
            shell: Some(shell),
            state: None,
            extra_cx: self.extra_cx,
            #[cfg(feature = "default")]
            serve_site_pkg: self.serve_site_pkg,
            error_handler: self.error_handler,
        }
    }

    /*
    /// Configure a new shell that takes the same existing state.
    pub fn shell<SH2, IV>(self, shell: SH2) -> RouterConfiguration<APP, CX, SH2, S>
    where
        SH2: Fn(S) -> IV + Clone + Send + Sync + 'static,
        IV: IntoView + 'static,
    {
        RouterConfiguration {
            app_fn: self.app_fn,
            shell: Some(shell),
            state: self.state,
            extra_cx: self.extra_cx,
        }
    }
    */

    /// Provide the additional context to set up Leptos routes with
    pub fn with_context<CX2>(
        self,
        extra_cx: CX2,
    ) -> RouterConfiguration<APP, CX2, SH, S>
    where
        CX2: Fn() + 'static + Clone + Send + Sync,
    {
        RouterConfiguration {
            app_fn: self.app_fn,
            shell: self.shell,
            state: self.state,
            extra_cx: Some(extra_cx),
            #[cfg(feature = "default")]
            serve_site_pkg: self.serve_site_pkg,
            error_handler: self.error_handler,
        }
    }

    /// Provide the state
    pub fn state<S2, IV>(
        self,
        state: S2,
    ) -> RouterConfiguration<APP, CX, SH, S2>
    where
        SH: Fn(S2) -> IV + Clone + Send + Sync + 'static,
        S2: Clone + Send + Sync + 'static,
        LeptosOptions: FromRef<S2>,
        IV: IntoView + 'static,
    {
        RouterConfiguration {
            app_fn: self.app_fn,
            shell: self.shell,
            state: Some(state),
            extra_cx: self.extra_cx,
            #[cfg(feature = "default")]
            serve_site_pkg: self.serve_site_pkg,
            error_handler: self.error_handler,
        }
    }
}

impl<APP, CX, SH, S, IV1, IV2> traits::RouterConfiguration<S>
    for RouterConfiguration<APP, CX, SH, S>
where
    APP: Fn() -> IV1 + Clone + Copy + Send + Sync + 'static,
    CX: Fn() + Clone + Copy + Send + Sync + 'static,
    SH: Fn(S) -> IV2 + Clone + Copy + Send + Sync + 'static,
    S: Clone + Send + Sync + 'static,
    LeptosOptions: FromRef<S>,
    IV1: IntoView + 'static,
    IV2: IntoView + 'static,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", fields(error), skip_all)
    )]
    fn apply(self, router: Router<S>) -> Router<()> {
        let app = self.app_fn.expect("an `App` should have been configured");
        let shell = self.shell.expect("a `shell` should have been configured");
        let state = self
            .state
            .as_ref()
            .expect("a `state` should have been configured");
        let extra_cx = self
            .extra_cx
            .expect("an `extra_cx` should have been configured");

        let routes = generate_route_list(app);
        let router =
            router.leptos_routes_with_context(state, routes, extra_cx, {
                let state = state.clone();
                move || shell(state.clone())
            });

        self.apply_common(router)
    }
}

impl<APP, SH, S, IV1, IV2> traits::RouterConfiguration<S>
    for RouterConfiguration<APP, (), SH, S>
where
    APP: Fn() -> IV1 + Clone + Copy + Send + Sync + 'static,
    SH: Fn(S) -> IV2 + Clone + Copy + Send + Sync + 'static,
    S: Clone + Send + Sync + 'static,
    LeptosOptions: FromRef<S>,
    IV1: IntoView + 'static,
    IV2: IntoView + 'static,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", fields(error), skip_all)
    )]
    fn apply(self, router: Router<S>) -> Router<()> {
        let app = self.app_fn.expect("an `App` should have been configured");
        let shell = self.shell.expect("a `shell` should have been configured");
        let state = self
            .state
            .as_ref()
            .expect("a `state` should have been configured");

        let routes = generate_route_list(app);
        let router = router.leptos_routes(state, routes, {
            let state = state.clone();
            move || shell(state.clone())
        });

        self.apply_common(router)
    }
}

impl<APP, CX, SH, S, IV> RouterConfiguration<APP, CX, SH, S>
where
    SH: Fn(S) -> IV + Clone + Copy + Send + Sync + 'static,
    S: Clone + Send + Sync + 'static,
    LeptosOptions: FromRef<S>,
    IV: IntoView + 'static,
{
    fn apply_common(self, router: Router<S>) -> Router<()> {
        let shell = self.shell.expect("a `shell` should have been configured");
        let state = self.state.expect("a `state` should have been configured");

        let error_handler = self
            .error_handler
            .then(|| ErrorHandler::new(shell, state.clone()));

        #[cfg(feature = "default")]
        let router = if self.serve_site_pkg {
            let leptos_options = LeptosOptions::from_ref(&state);
            if let Some(error_handler) = error_handler.clone() {
                router.route_service(
                    &site_pkg_dir_service_route_path(&leptos_options),
                    site_pkg_dir_service(&leptos_options)
                        .fallback(error_handler),
                )
            } else {
                router.route_service(
                    &site_pkg_dir_service_route_path(&leptos_options),
                    site_pkg_dir_service(&leptos_options),
                )
            }
        } else {
            router
        };

        let router = if let Some(error_handler) = error_handler {
            router.fallback_service(error_handler)
        } else {
            router
        };

        router.with_state(state)
    }
}
