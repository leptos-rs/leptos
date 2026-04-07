//! Provides a builder and implementation for wholesale configuration of [`axum::Router`].

use crate::{ErrorHandler, LeptosRoutes, generate_route_list};
#[cfg(feature = "default")]
use crate::{
    LeptosContextLayer, site_pkg_dir_service, site_pkg_dir_service_route_path,
};
use axum::{Router, extract::FromRef};
use leptos::{IntoView, config::LeptosOptions};
#[cfg(feature = "default")]
use std::borrow::Cow;
#[cfg(feature = "default")]
use tower::builder::ServiceBuilder;

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
        fn apply<S2>(self, router: Router<S>) -> Router<S2>;
    }
}

/// The possible modes for serving resources for a [`RouterConfiguration`].
#[cfg(feature = "default")]
#[derive(Clone, Default)]
enum ResourceMode {
    /// Disables the serving of site pkg dir.
    #[default]
    Disable,
    /// Use the files found on the filesystem at runtime.
    Filesystem,
    /// Build the compiled site pkg dir into the server binary.
    BuiltIn,
}

/// The possible modes for serving of the assets for a [`RouterConfiguration`].
///
/// Assets are files copied to `LEPTOS_SITE_ROOT` if `LEPTOS_ASSETS_DIR` is configured.
#[cfg(feature = "default")]
#[derive(Clone, Default)]
enum AssetMode {
    /// Disables the serving of assets.
    #[default]
    Disable,
    /// Serves the assets directory by using the [`ServeDir`] service created by [`site_pkg_dir_service`].
    /// If the provided path is `"/"`, it will become part of the fallback service, otherwise a new router
    /// will be created to serve this.
    ///
    /// [`ServeDir`]: tower_http::services::ServeDir
    ServeDir(Cow<'static, str>),
}

/// A configuration builder that simplifies the set up of a Leptos application onto an Axum router.
///
/// This builder is used in conjunction with [`LeptosRoutes::leptos_route_configure`], please refer to it for
/// a practical example.
///
/// Note that an incomplete configuration should lead to a compilation error rather than a runtime error due
/// to the trait bounds.  The required fields are `app`, `shell`, and `state`.
#[derive(Clone)]
pub struct RouterConfiguration<APP, CX = fn(), SH = (), S = ()> {
    app_fn: Option<APP>,
    shell: Option<SH>,
    state: Option<S>,
    extra_cx: Option<CX>,
    #[cfg(feature = "default")]
    site_pkg_mode: ResourceMode,
    #[cfg(feature = "default")]
    favicon_mode: ResourceMode,
    #[cfg(feature = "default")]
    serve_asset: AssetMode,
    // TODO favicon embed?
    error_handler: bool,
}

/// Create a new configuration with all toggles disabled.
impl<APP> Default for RouterConfiguration<APP> {
    fn default() -> Self {
        Self {
            app_fn: None,
            shell: None,
            state: None,
            extra_cx: None,
            #[cfg(feature = "default")]
            site_pkg_mode: ResourceMode::default(),
            #[cfg(feature = "default")]
            favicon_mode: ResourceMode::default(),
            #[cfg(feature = "default")]
            serve_asset: AssetMode::default(),
            error_handler: false,
        }
    }
}

impl<APP> RouterConfiguration<APP> {
    /// Create a new configuration with all toggles set to the recommended value.
    ///
    /// This enables route to the site pkg [`ServeDir`] service and the [`ErrorHandler`] service as the
    /// fallback handler, along with a route to `/favicon.ico` under assets; refer to [`.site_pkg_mode`]
    /// and [`.error_handler`] for further details.
    ///
    /// Use of `RouterConfiguration::default()` disables these by default.
    ///
    /// [`ServeDir`]: tower_http::services::ServeDir
    /// [`.site_pkg_mode`]: RouterConfiguration::site_pkg_mode
    /// [`.error_handler`]: RouterConfiguration::error_handler
    pub fn new() -> Self {
        Self {
            app_fn: None,
            shell: None,
            state: None,
            extra_cx: None,
            #[cfg(feature = "default")]
            site_pkg_mode: ResourceMode::Filesystem,
            #[cfg(feature = "default")]
            favicon_mode: ResourceMode::Filesystem,
            #[cfg(feature = "default")]
            serve_asset: AssetMode::Disable,
            error_handler: true,
        }
    }

    /// Create a new configuration with all toggles set to the recommended value.
    ///
    /// This enables route to the site pkg [`ServeDir`] service and the default fallback being the assets
    /// `ServeDir` service with an [`ErrorHandler`] service as the ultimate fallback handler.  This should
    /// fully replicate the `file_and_error_handler_with_context` fallback handler.
    ///
    /// Refer to [`.serve_asset`], [`.site_pkg_mode`], and [`.error_handler`] for further details.
    ///
    /// Use of `RouterConfiguration::default()` disables these by default.
    ///
    /// [`ServeDir`]: tower_http::services::ServeDir
    /// [`.serve_asset`]: RouterConfiguration::serve_asset
    /// [`.site_pkg_mode`]: RouterConfiguration::site_pkg_mode
    /// [`.error_handler`]: RouterConfiguration::error_handler
    pub fn new_with_assets() -> Self {
        Self {
            app_fn: None,
            shell: None,
            state: None,
            extra_cx: None,
            #[cfg(feature = "default")]
            site_pkg_mode: ResourceMode::Filesystem,
            // TODO verify how this value may conflict with the setting defined in `serve_asset` as it may
            // remain in `"/"` but also be configured to something else.
            #[cfg(feature = "default")]
            favicon_mode: ResourceMode::Filesystem,
            #[cfg(feature = "default")]
            serve_asset: AssetMode::ServeDir("/".into()),
            error_handler: true,
        }
    }
}

impl<APP, CX, SH, S> RouterConfiguration<APP, CX, SH, S> {
    /// Provide the `App` to the configuration.  This is required.
    pub fn app<IV>(mut self, app: APP) -> Self
    where
        APP: Fn() -> IV + Clone + Send + Sync + 'static,
        IV: IntoView + 'static,
    {
        self.app_fn = Some(app);
        self
    }

    /// Toggle for the fallback error handler; set to `true` to enable.
    ///
    /// When enabled, the [`ErrorHandler`] service will be applied as the fallback service, so that access to
    /// locations that do not have an active route will instead be rendered using the `shell`, which typically
    /// will render the 404 Not Found page generated by the underlying application.
    pub fn error_handler(mut self, v: bool) -> Self {
        self.error_handler = v;
        self
    }

    /// Configure a new shell function.  This is required.
    ///
    /// Ensure that the argument that will be passed to this function be supplied with [`.state`] to this
    /// builder, and somewhere within its returned view should contain the `App` component set up with this
    /// builder with [`.app`].
    ///
    /// [`.app`]: RouterConfiguration::app
    /// [`.state`]: RouterConfiguration::state
    pub fn shell<SH2, S2, IV>(
        self,
        shell: SH2,
    ) -> RouterConfiguration<APP, CX, SH2, S>
    where
        SH2: Fn(S2) -> IV + Clone + Send + Sync + 'static,
        S2: Clone + Send + Sync + 'static,
        LeptosOptions: FromRef<S2>,
        IV: IntoView + 'static,
    {
        RouterConfiguration {
            app_fn: self.app_fn,
            shell: Some(shell),
            state: self.state,
            extra_cx: self.extra_cx,
            #[cfg(feature = "default")]
            site_pkg_mode: self.site_pkg_mode,
            #[cfg(feature = "default")]
            favicon_mode: self.favicon_mode,
            #[cfg(feature = "default")]
            serve_asset: self.serve_asset,
            error_handler: self.error_handler,
        }
    }

    /// Provide an additional context to set up Leptos routes with.  This is optional.
    ///
    /// The provided closure will be applied to all underlying services.
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
            site_pkg_mode: self.site_pkg_mode,
            #[cfg(feature = "default")]
            favicon_mode: self.favicon_mode,
            #[cfg(feature = "default")]
            serve_asset: self.serve_asset,
            error_handler: self.error_handler,
        }
    }

    /// Provide the state.  This is required.
    ///
    /// This must be a value of the same type as the singular argument that will be passed to [`.shell`].
    ///
    /// [`.shell`]: RouterConfiguration::shell
    pub fn state<S2>(self, state: S2) -> RouterConfiguration<APP, CX, SH, S2>
    where
        S2: Clone + Send + Sync + 'static,
        LeptosOptions: FromRef<S2>,
    {
        RouterConfiguration {
            app_fn: self.app_fn,
            shell: self.shell,
            state: Some(state),
            extra_cx: self.extra_cx,
            #[cfg(feature = "default")]
            site_pkg_mode: self.site_pkg_mode,
            #[cfg(feature = "default")]
            favicon_mode: self.favicon_mode,
            #[cfg(feature = "default")]
            serve_asset: self.serve_asset,
            error_handler: self.error_handler,
        }
    }
}

#[cfg(feature = "default")]
impl<APP, CX, SH, S> RouterConfiguration<APP, CX, SH, S> {
    /// Configure the [`AssetMode`] to seve the assets with.
    ///
    /// When not disabled, the underlying `LeptosOptions` will be referenced along the configured mode to
    /// provide the relevant route or configure the appropriate fallback service to serve the assets.
    fn serve_asset(mut self, v: AssetMode) -> Self {
        self.serve_asset = v;
        self
    }

    /// Configure the base route for the `ServeDir` service that will provide the files found in
    /// `LEPTOS_SITE_ROOT` defined at runtime.
    ///
    /// If the provided path is `"/"`, the fallback service will be used instead, in conjunction with the
    /// [`ErrorHandler`] service if it is also availabled.  Otherwise [`Router::route_service`] will be used
    /// to set this service up.
    pub fn enable_fs_leptos_site_root(
        self,
        path: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.serve_asset(AssetMode::ServeDir(path.into()))
    }

    /// Disable the routing of `LEPTOS_SITE_ROOT`.
    pub fn disable_leptos_site_root(self) -> Self {
        self.serve_asset(AssetMode::Disable)
    }

    /// Configure the [`ResourceMode`] to seve the site pkg with.
    ///
    /// When not disabled, the underlying `LeptosOptions` will be referenced along the configured mode to
    /// provide the relevant routes to serve the JS/WASM bundle such that the application will be activated
    /// on the client.
    fn site_pkg_mode(mut self, v: ResourceMode) -> Self {
        self.site_pkg_mode = v;
        self
    }

    /// Enable the routing of files in the `LEPTOS_SITE_PKG` subdirectory within `LEPTOS_SITE_ROOT` by the
    /// [`ServeDir`] service set up at runtime on the relevant path on the filesystem.
    ///
    /// This is used to serve the JS/WASM bundle such that the application will be activated on the client.
    pub fn enable_fs_site_pkg(self) -> Self {
        self.site_pkg_mode(ResourceMode::Filesystem)
    }

    /// Compile the files in the `LEPTOS_SITE_PKG` subdirectory within `LEPTOS_SITE_ROOT` found during compile
    /// time.  At runtime the routes will be created to serve these builtin files.
    ///
    /// This is used to serve the JS/WASM bundle such that the application will be activated on the client.
    pub fn enable_builtin_site_pkg(self) -> Self {
        self.site_pkg_mode(ResourceMode::BuiltIn)
    }

    /// Disables the routing of `LEPTOS_SITE_PKG` files.
    pub fn disable_site_pkg(self) -> Self {
        self.site_pkg_mode(ResourceMode::Disable)
    }

    /// Configure how the `favicon.ico` is served.
    fn favicon_mode(mut self, v: ResourceMode) -> Self {
        self.favicon_mode = v;
        self
    }

    /// Enable the routing of `favicon.ico` in the `LEPTOS_SITE_PKG` on the filesystem.
    pub fn enable_fs_favicon(self) -> Self {
        self.favicon_mode(ResourceMode::Filesystem)
    }

    /// Enable the routing of `favicon.ico` in the `LEPTOS_SITE_PKG` by building it into the target binary.
    pub fn enable_builtin_favicon(self) -> Self {
        self.favicon_mode(ResourceMode::BuiltIn)
    }

    /// Disables the routing of `favicon.ico`
    pub fn disable_favicon(self) -> Self {
        self.favicon_mode(ResourceMode::Disable)
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
    fn apply<S2>(self, router: Router<S>) -> Router<S2> {
        let app = self.app_fn.expect("an `App` should have been configured");
        let shell = self.shell.expect("a `shell` should have been configured");
        let state = self.state.expect("a `state` should have been configured");
        let extra_cx = self.extra_cx;

        let routes = generate_route_list(app);
        let router = if let Some(extra_cx) = extra_cx {
            router.leptos_routes_with_context(&state, routes, extra_cx, {
                let state = state.clone();
                move || shell(state.clone())
            })
        } else {
            router.leptos_routes(&state, routes, {
                let state = state.clone();
                move || shell(state.clone())
            })
        };

        let error_handler = self.error_handler.then(|| {
            ErrorHandler::new_with_option_context(
                extra_cx,
                shell,
                state.clone(),
            )
        });

        #[cfg(feature = "default")]
        let router = match self.site_pkg_mode {
            ResourceMode::Disable => router,
            ResourceMode::Filesystem => {
                let builder = ServiceBuilder::new().option_layer(
                    extra_cx.map(LeptosContextLayer::new_with_context),
                );
                let leptos_options = LeptosOptions::from_ref(&state);
                if let Some(error_handler) = error_handler.clone() {
                    router.route_service(
                        &site_pkg_dir_service_route_path(&leptos_options),
                        builder.service(
                            site_pkg_dir_service(&leptos_options)
                                .fallback(error_handler),
                        ),
                    )
                } else {
                    router.route_service(
                        &site_pkg_dir_service_route_path(&leptos_options),
                        builder.service(site_pkg_dir_service(&leptos_options)),
                    )
                }
            }
            ResourceMode::BuiltIn => {
                todo!()
            }
        };

        #[cfg(feature = "default")]
        let router = match self.favicon_mode {
            ResourceMode::Disable => router,
            ResourceMode::Filesystem => {
                let builder = ServiceBuilder::new().option_layer(
                    extra_cx.map(LeptosContextLayer::new_with_context),
                );
                let leptos_options = LeptosOptions::from_ref(&state);
                if let Some(error_handler) = error_handler.clone() {
                    router.route_service(
                        "/favicon.ico",
                        builder.service(
                            site_pkg_dir_service(&leptos_options)
                                .fallback(error_handler),
                        ),
                    )
                } else {
                    router.route_service(
                        "/favicon.ico",
                        builder.service(site_pkg_dir_service(&leptos_options)),
                    )
                }
            }
            ResourceMode::BuiltIn => {
                todo!()
            }
        };

        #[cfg(feature = "default")]
        let router = if let Some(error_handler) = error_handler {
            let leptos_options = LeptosOptions::from_ref(&state);
            // While the one set up for `site_pkg_mode` may be used, it might not be configured and so
            // reusing that clone may be problematic; much easier to create one just for here; maybe refactor
            // this later when implementation is more settled.
            let builder = ServiceBuilder::new().option_layer(
                extra_cx.map(LeptosContextLayer::new_with_context),
            );
            match self.serve_asset {
                AssetMode::ServeDir(path) if path == "/" => router
                    .fallback_service(
                        builder.service(
                            site_pkg_dir_service(&leptos_options)
                                .fallback(error_handler),
                        ),
                    ),
                AssetMode::ServeDir(path) => router
                    .nest(
                        &path,
                        Router::new().route_service(
                            "/{*path}",
                            builder.service(
                                site_pkg_dir_service(&leptos_options)
                                    .fallback(error_handler.clone()),
                            ),
                        ),
                    )
                    .fallback_service(error_handler),
                AssetMode::Disable => router.fallback_service(error_handler),
            }
        } else {
            let leptos_options = LeptosOptions::from_ref(&state);
            let builder = ServiceBuilder::new().option_layer(
                extra_cx.map(LeptosContextLayer::new_with_context),
            );
            match self.serve_asset {
                AssetMode::ServeDir(path) if path == "/" => router
                    .fallback_service(
                        builder.service(site_pkg_dir_service(&leptos_options)),
                    ),
                AssetMode::ServeDir(path) => router.nest(
                    &path,
                    Router::new().route_service(
                        "/{*path}",
                        builder.service(site_pkg_dir_service(&leptos_options)),
                    ),
                ),
                AssetMode::Disable => router,
            }
        };

        #[cfg(not(feature = "default"))]
        let router = if let Some(error_handler) = error_handler {
            router.fallback_service(error_handler)
        } else {
            router
        };

        router.with_state(state)
    }
}
