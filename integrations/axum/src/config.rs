//! Provides a builder and implementation for wholesale configuration of [`axum::Router`].

#[cfg(any(feature = "default", feature = "embed"))]
use crate::LeptosContextLayer;
#[cfg(feature = "embed")]
use crate::service::EmbeddedSiteRoot;
use crate::{ErrorHandler, LeptosRoutes, generate_route_list};
#[cfg(feature = "default")]
use crate::{serve_site_pkg_service, serve_site_root_service};
use axum::{Router, extract::FromRef};
#[cfg(any(feature = "default", feature = "embed"))]
use axum::{
    body::Body,
    extract::Request,
    http::Uri,
    response::{IntoResponse, Response},
};
use leptos::{IntoView, config::LeptosOptions};
#[cfg(feature = "embed")]
use rust_embed::{EmbeddedFile, RustEmbed};
#[cfg(any(feature = "default", feature = "embed"))]
use std::sync::Arc;
#[cfg(feature = "embed")]
use std::{borrow::Cow, path::Component};
#[cfg(any(feature = "default", feature = "embed"))]
use std::{convert::Infallible, path::Path};
#[cfg(any(feature = "default", feature = "embed"))]
use tower::{Service, builder::ServiceBuilder};
#[cfg(any(feature = "default", feature = "embed"))]
use tower_http::services::fs::{Backend, DefaultServeDirFallback, ServeDir};

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
#[derive(Clone, Default)]
enum ResourceMode {
    /// Disables this resource.
    #[default]
    Disable,
    /// Use the files found on the filesystem at runtime.
    #[cfg(feature = "default")]
    Filesystem,
    /// Serve embedded files.
    #[cfg(feature = "embed")]
    Embed,
}

#[cfg(any(feature = "default", feature = "embed"))]
pub(crate) enum StaticRoute<'a> {
    Mount(&'a str),
    Exact(&'a str),
    Fallback,
}

#[cfg(any(feature = "default", feature = "embed"))]
impl StaticRoute<'_> {
    pub(crate) fn mount<S, B, F, CX>(
        self,
        router: Router<S>,
        service: ServeDir<DefaultServeDirFallback, B>,
        fallback: Option<F>,
        extra_cx: Option<CX>,
    ) -> Router<S>
    where
        S: Clone + Send + Sync + 'static,
        B: Backend,
        F: Service<Request<Body>, Response = Response, Error = Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        F::Future: Send + 'static,
        CX: Fn() + Clone + Send + Sync + 'static,
    {
        // Axum strips the mount prefix for file lookup. Include it when
        // ServeDir redirects to the trailing-slash URL.
        let (service, prefix) = match self {
            Self::Mount(base) if base != "/" => {
                let prefix = base.trim_end_matches('/');
                (service.redirect_path_prefix(prefix), Some(prefix))
            }
            _ => (service, None),
        };
        let builder = ServiceBuilder::new()
            .option_layer(extra_cx.map(LeptosContextLayer::new_with_context));
        if let Some(fallback) = fallback {
            // ServeDir needs the stripped path. The fallback renders the
            // request in the parent router, so restore this mount's prefix.
            let prefix = prefix.map(Arc::<str>::from);
            let fallback = ServiceBuilder::new()
                .map_request(move |mut req: Request| {
                    if let Some(prefix) = &prefix
                        && let Some(path) = req.uri().path_and_query()
                        && let Ok(path) = format!("{prefix}{path}").parse()
                    {
                        let mut parts = req.uri().clone().into_parts();
                        parts.path_and_query = Some(path);
                        if let Ok(uri) = Uri::from_parts(parts) {
                            *req.uri_mut() = uri;
                        }
                    }
                    req
                })
                .service(fallback);
            self.route(router, builder.service(service.fallback(fallback)))
        } else {
            self.route(router, builder.service(service))
        }
    }

    fn route<S, T>(self, router: Router<S>, service: T) -> Router<S>
    where
        S: Clone + Send + Sync + 'static,
        T: Service<Request<Body>, Error = Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        T::Response: IntoResponse,
        T::Future: Send + 'static,
    {
        match self {
            Self::Mount("/") => router.route_service("/{*path}", service),
            Self::Mount(base) => router.nest(
                base.trim_end_matches('/'),
                Router::new().route_service("/{*path}", service),
            ),
            Self::Exact(path) => router.route_service(path, service),
            Self::Fallback => router.fallback_service(service),
        }
    }
}

#[derive(Copy, Clone)]
pub struct NoEmbedSiteRoot;

#[cfg(feature = "embed")]
impl RustEmbed for NoEmbedSiteRoot {
    fn get(_: &str) -> Option<EmbeddedFile> {
        None
    }

    fn iter() -> impl Iterator<Item = Cow<'static, str>> + 'static {
        [].into_iter()
    }
}

/// A configuration builder that simplifies the set up of a Leptos application onto an Axum router.
///
/// This builder is used in conjunction with [`LeptosRoutes::leptos_route_configure`], please refer to it for
/// a practical example.
///
/// Note that an incomplete configuration should lead to a compilation error rather than a runtime error due
/// to the trait bounds.  The required fields are `app`, `shell`, and `state`.
/// When both mounts match, the longer URL path takes precedence.
/// The pkg mount wins when the URL paths are equal.
#[derive(Clone)]
pub struct RouterConfiguration<
    APP,
    CX = fn(),
    SH = (),
    S = (),
    SR = NoEmbedSiteRoot,
    SP = NoEmbedSiteRoot,
> {
    app_fn: Option<APP>,
    shell: Option<SH>,
    state: Option<S>,
    extra_cx: Option<CX>,
    site_pkg_mode: ResourceMode,
    favicon_mode: ResourceMode,
    site_root_mode: ResourceMode,
    error_handler: bool,
    site_root: SR,
    site_pkg: Option<SP>,
}

/// Create a new configuration with all options disabled.
impl<APP> Default for RouterConfiguration<APP> {
    fn default() -> Self {
        Self {
            app_fn: None,
            shell: None,
            state: None,
            extra_cx: None,
            site_pkg_mode: ResourceMode::Disable,
            favicon_mode: ResourceMode::Disable,
            site_root_mode: ResourceMode::Disable,
            error_handler: false,
            site_root: NoEmbedSiteRoot,
            site_pkg: None,
        }
    }
}

impl<APP> RouterConfiguration<APP> {
    /// Create a new configuration base with the commonly recommended values.
    ///
    /// With default features enabled, this serves the pkg directory at the
    /// pkg URL and `/favicon.ico` from the site root. It also installs the
    /// [`ErrorHandler`] fallback service.
    ///
    /// Without default features, only the error fallback is enabled.
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

            #[cfg(not(feature = "default"))]
            site_pkg_mode: ResourceMode::Disable,
            #[cfg(not(feature = "default"))]
            favicon_mode: ResourceMode::Disable,

            site_root_mode: ResourceMode::Disable,
            error_handler: true,

            site_root: NoEmbedSiteRoot,
            site_pkg: None,
        }
    }

    /// Like [`Self::new`], with the site root mounted at its configured URL.
    #[cfg(feature = "default")]
    pub fn new_with_assets() -> Self {
        Self {
            app_fn: None,
            shell: None,
            state: None,
            extra_cx: None,
            site_pkg_mode: ResourceMode::Filesystem,
            favicon_mode: ResourceMode::Filesystem,
            site_root_mode: ResourceMode::Filesystem,
            error_handler: true,
            site_root: NoEmbedSiteRoot,
            site_pkg: None,
        }
    }
}

// Future work may include the following:
//
// - Leptos specific derive macro to set up the `RustEmbed`.
#[cfg(feature = "embed")]
impl<APP, SR> RouterConfiguration<APP, fn(), (), (), SR>
where
    SR: RustEmbed,
{
    /// The embedded counterpart to [`RouterConfiguration::new()`].
    ///
    /// Serves the pkg from the embedded site root when the pkg directory is
    /// inside `site_root`. For a separate pkg embed, use
    /// [`Self::enable_embed_site_pkg`]. This also serves `/favicon.ico` from
    /// the site root embed and installs the [`ErrorHandler`] fallback.
    ///
    /// Please note that `site_root` must be a [`RustEmbed`] implementation derived from [`Embed`], and it
    /// should be constructed like so within the target application:
    ///
    /// ```rust
    /// use leptos_axum::{
    ///     RouterConfiguration,
    ///     rust_embed::{self, Embed},
    /// };
    ///
    /// #[derive(Clone, Copy, Embed)]
    /// #[folder = "$LEPTOS_SITE_ROOT"]
    /// #[prefix = "/"]
    /// # #[allow_missing = true]
    /// struct SiteRoot;
    ///
    /// # let (app, shell) = (|| (), |_| ());
    /// # let state = leptos::config::LeptosOptions::builder().output_name("test").build();
    /// // Usage of the above derive may be like so:
    /// let config = RouterConfiguration::embed(SiteRoot)
    ///     .app(app)
    ///     .shell(shell)
    ///     .state(state);
    /// ```
    ///
    /// Also note that `RustEmbed` requires that the files are available before they may be included.  The
    /// underlying build tooling (e.g. `cargo-leptos`) may need to be invoked in a manner to ensure that the
    /// frontend is compiled before the server, otherwise the resulting server binary may lack the required
    /// data for the serving of the frontend client.
    ///
    /// Another thing to note is that the configuration of the `RustEmbed` should have the folder defined
    /// with `LEPTOS_SITE_ROOT` as the base, in order to cater to the most general case where data beyond the
    /// site package may be included; this has the unfortunate consequence of data included with the site root
    /// that might never be routed.  For example, if routing to `favicon.ico` is disabled, that file will
    /// still be embedded.  This may be refined in a future version.
    ///
    /// Refer to [`.enable_embed_site_pkg`] and [`.error_handler`] for additional details.
    ///
    /// [`Embed`]: rust_embed::Embed
    /// [`RustEmbed`]: rust_embed::RustEmbed
    /// [`ServeDir`]: tower_http::services::ServeDir
    /// [`.enable_embed_site_pkg`]: RouterConfiguration::enable_embed_site_pkg
    /// [`.error_handler`]: RouterConfiguration::error_handler
    pub fn embed(site_root: SR) -> Self {
        Self {
            app_fn: None,
            shell: None,
            state: None,
            extra_cx: None,
            site_pkg_mode: ResourceMode::Embed,
            favicon_mode: ResourceMode::Embed,
            site_root_mode: ResourceMode::Disable,
            error_handler: true,
            site_root,
            site_pkg: None,
        }
    }

    /// The embedded counterpart to [`RouterConfiguration::new_with_assets()`].
    ///
    /// Like [`Self::embed`], with the embedded site root mounted at its URL.
    ///
    /// Please note that `site_root` must be a [`RustEmbed`] implementation derived from [`Embed`], and it
    /// must be constructed like so within the target application:
    ///
    /// ```rust
    /// use leptos_axum::{
    ///     rust_embed::{self, Embed},
    ///     RouterConfiguration,
    /// };
    ///
    /// #[derive(Clone, Copy, Embed)]
    /// #[folder = "$LEPTOS_SITE_ROOT"]
    /// #[prefix = "/"]
    /// # #[allow_missing = true]
    /// struct SiteRoot;
    ///
    /// # let (app, shell) = (|| (), |_| ());
    /// # let state = leptos::config::LeptosOptions::builder().output_name("test").build();
    /// // Usage of the above derive may be like so:
    /// let config = RouterConfiguration::embed(SiteRoot)
    ///     .app(app)
    ///     .shell(shell)
    ///     .state(state);
    /// ```
    ///
    /// Also note that `RustEmbed` requires that the files are available before they may be included.  The
    /// underlying build tooling (e.g. `cargo-leptos`) may need to be invoked in a manner to ensure that the
    /// frontend is compiled before the server, otherwise the resulting server binary may lack the required
    /// data for the serving of the frontend client.
    ///
    /// Refer to [`.enable_embed_leptos_site_root`], [`.enable_fs_site_pkg`], and [`.error_handler`] for
    /// additional details.
    ///
    /// [`Embed`]: rust_embed::Embed
    /// [`RustEmbed`]: rust_embed::RustEmbed
    /// [`ServeDir`]: tower_http::services::ServeDir
    /// [`.enable_fs_site_pkg`]: RouterConfiguration::enable_fs_site_pkg
    /// [`.enable_embed_leptos_site_root`]: RouterConfiguration::enable_embed_leptos_site_root
    /// [`.error_handler`]: RouterConfiguration::error_handler
    pub fn embed_with_assets(site_root: SR) -> Self {
        Self {
            app_fn: None,
            shell: None,
            state: None,
            extra_cx: None,
            site_pkg_mode: ResourceMode::Embed,
            favicon_mode: ResourceMode::Embed,
            site_root_mode: ResourceMode::Embed,
            error_handler: true,
            site_root,
            site_pkg: None,
        }
    }
}

impl<APP, CX, SH, S, SR, SP> RouterConfiguration<APP, CX, SH, S, SR, SP> {
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
    ) -> RouterConfiguration<APP, CX, SH2, S, SR, SP>
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
            site_pkg_mode: self.site_pkg_mode,
            favicon_mode: self.favicon_mode,
            site_root_mode: self.site_root_mode,
            error_handler: self.error_handler,
            site_root: self.site_root,
            site_pkg: self.site_pkg,
        }
    }

    /// Provide an additional context to set up Leptos routes with.  This is optional.
    ///
    /// The provided closure will be applied to all underlying services.
    pub fn with_context<CX2>(
        self,
        extra_cx: CX2,
    ) -> RouterConfiguration<APP, CX2, SH, S, SR, SP>
    where
        CX2: Fn() + 'static + Clone + Send + Sync,
    {
        RouterConfiguration {
            app_fn: self.app_fn,
            shell: self.shell,
            state: self.state,
            extra_cx: Some(extra_cx),
            site_pkg_mode: self.site_pkg_mode,
            favicon_mode: self.favicon_mode,
            site_root_mode: self.site_root_mode,
            error_handler: self.error_handler,
            site_root: self.site_root,
            site_pkg: self.site_pkg,
        }
    }

    /// Provide the state.  This is required.
    ///
    /// This must be a value of the same type as the singular argument that will be passed to [`.shell`].
    ///
    /// [`.shell`]: RouterConfiguration::shell
    pub fn state<S2>(
        self,
        state: S2,
    ) -> RouterConfiguration<APP, CX, SH, S2, SR, SP>
    where
        S2: Clone + Send + Sync + 'static,
        LeptosOptions: FromRef<S2>,
    {
        RouterConfiguration {
            app_fn: self.app_fn,
            shell: self.shell,
            state: Some(state),
            extra_cx: self.extra_cx,
            site_pkg_mode: self.site_pkg_mode,
            favicon_mode: self.favicon_mode,
            site_root_mode: self.site_root_mode,
            error_handler: self.error_handler,
            site_root: self.site_root,
            site_pkg: self.site_pkg,
        }
    }
}

impl<APP, CX, SH, S, SR, SP> RouterConfiguration<APP, CX, SH, S, SR, SP> {
    /// Configure the [`ResourceMode`] to serve the site pkg with.
    ///
    /// When not disabled, the underlying `LeptosOptions` will be referenced along the configured mode to
    /// provide the relevant routes to serve the JS/WASM bundle such that the application will be activated
    /// on the client.
    fn site_pkg_mode(mut self, v: ResourceMode) -> Self {
        self.site_pkg_mode = v;
        self
    }

    /// Configure how the `favicon.ico` is served.
    fn favicon_mode(mut self, v: ResourceMode) -> Self {
        self.favicon_mode = v;
        self
    }

    /// Disables the site root mount.
    pub fn disable_leptos_site_root(mut self) -> Self {
        self.site_root_mode = ResourceMode::Disable;
        self
    }

    /// Disables the pkg mount.
    pub fn disable_site_pkg(self) -> Self {
        self.site_pkg_mode(ResourceMode::Disable)
    }

    /// Disables setting up of the routing of `favicon.ico`.
    pub fn disable_favicon(self) -> Self {
        self.favicon_mode(ResourceMode::Disable)
    }
}

#[cfg(feature = "default")]
impl<APP, CX, SH, S, SR, SP> RouterConfiguration<APP, CX, SH, S, SR, SP> {
    /// Mounts the site root at [`LeptosOptions::site_root_route_base`].
    /// The site root is the fallback service when its URL is `/`.
    pub fn enable_fs_leptos_site_root(mut self) -> Self {
        self.site_root_mode = ResourceMode::Filesystem;
        self
    }

    /// Serves [`LeptosOptions::site_pkg_path`] at
    /// [`LeptosOptions::site_pkg_route_base`] using a filesystem [`ServeDir`].
    ///
    /// [`ServeDir`]: tower_http::services::ServeDir
    pub fn enable_fs_site_pkg(self) -> Self {
        self.site_pkg_mode(ResourceMode::Filesystem)
    }

    /// Enable the routing of `favicon.ico` in the `LEPTOS_SITE_ROOT` on the filesystem.
    pub fn enable_fs_favicon(self) -> Self {
        self.favicon_mode(ResourceMode::Filesystem)
    }
}

#[cfg(feature = "embed")]
impl<APP, CX, SH, S, SR, SP> RouterConfiguration<APP, CX, SH, S, SR, SP>
where
    SR: Clone + Copy + Send + Sync + RustEmbed + 'static,
{
    /// Mounts the embedded site root at [`LeptosOptions::site_root_route_base`].
    /// The site root is the fallback service when its URL is `/`.
    /// A separate pkg embed stays configured.
    pub fn enable_embed_leptos_site_root<SR2>(
        self,
        site_root: SR2,
    ) -> RouterConfiguration<APP, CX, SH, S, SR2, SP>
    where
        SR2: Clone + Copy + Send + Sync + RustEmbed + 'static,
    {
        RouterConfiguration {
            app_fn: self.app_fn,
            shell: self.shell,
            state: self.state,
            extra_cx: self.extra_cx,
            site_pkg_mode: self.site_pkg_mode,
            favicon_mode: self.favicon_mode,
            site_root_mode: ResourceMode::Embed,
            error_handler: self.error_handler,
            site_root,
            site_pkg: self.site_pkg,
        }
    }

    /// Mounts an embed of the pkg directory at [`LeptosOptions::site_pkg_route_base`].
    /// The embed uses the pkg directory as its folder and `/` as its prefix.
    /// Other assets can still be served by the site root mount.
    /// Use `$LEPTOS_SITE_PKG_ROOT` as the folder when `site-pkg-root` is
    /// set because `cargo-leptos` passes that variable only when the key
    /// is set.
    ///
    /// ```rust
    /// use leptos_axum::{
    ///     RouterConfiguration,
    ///     rust_embed::{self, Embed},
    /// };
    ///
    /// #[derive(Clone, Copy, Embed)]
    /// #[folder = "$LEPTOS_SITE_ROOT/pkg"]
    /// #[prefix = "/"]
    /// # #[allow_missing = true]
    /// struct SitePkg;
    ///
    /// # let (app, shell) = (|| (), |_| ());
    /// # let state = leptos::config::LeptosOptions::builder().output_name("test").build();
    /// let config = RouterConfiguration::default()
    ///     .enable_embed_site_pkg(SitePkg)
    ///     .app(app)
    ///     .shell(shell)
    ///     .state(state);
    /// ```
    pub fn enable_embed_site_pkg<SP2>(
        self,
        site_pkg: SP2,
    ) -> RouterConfiguration<APP, CX, SH, S, SR, SP2>
    where
        SP2: Clone + Copy + Send + Sync + RustEmbed + 'static,
    {
        RouterConfiguration {
            app_fn: self.app_fn,
            shell: self.shell,
            state: self.state,
            extra_cx: self.extra_cx,
            site_pkg_mode: ResourceMode::Embed,
            favicon_mode: self.favicon_mode,
            site_root_mode: self.site_root_mode,
            error_handler: self.error_handler,
            site_root: self.site_root,
            site_pkg: Some(site_pkg),
        }
    }

    /// Enable the routing of `favicon.ico` in the embedded site root.
    pub fn enable_embed_favicon(self) -> Self {
        self.favicon_mode(ResourceMode::Embed)
    }
}

#[cfg(feature = "embed")]
pub trait SiteRootBound:
    Clone + Copy + Send + Sync + RustEmbed + 'static
{
}
#[cfg(feature = "embed")]
impl<T> SiteRootBound for T where
    T: Clone + Copy + Send + Sync + RustEmbed + 'static
{
}

#[cfg(not(feature = "embed"))]
pub trait SiteRootBound: Clone + Copy + Send + Sync + 'static {}
#[cfg(not(feature = "embed"))]
impl<T> SiteRootBound for T where T: Clone + Copy + Send + Sync + 'static {}

impl<APP, CX, SH, S, SR, SP, IV1, IV2> traits::RouterConfiguration<S>
    for RouterConfiguration<APP, CX, SH, S, SR, SP>
where
    APP: Fn() -> IV1 + Clone + Copy + Send + Sync + 'static,
    CX: Fn() + Clone + Copy + Send + Sync + 'static,
    SH: Fn(S) -> IV2 + Clone + Copy + Send + Sync + 'static,
    S: Clone + Send + Sync + 'static,
    // Can't do the following yet because of <https://github.com/rust-lang/rust/issues/115590>.
    // #[cfg(feature = "embed")] SR: Clone + Copy + Send + Sync + RustEmbed + 'static,
    // #[cfg(not(feature = "embed"))] SR: Clone + Copy + Send + Sync + 'static,
    // Hence we have this surrogate trait with the bounds as its supertraits which has been defined
    // conditionally above.
    SR: SiteRootBound,
    SP: SiteRootBound,
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

        #[cfg(any(feature = "default", feature = "embed"))]
        let leptos_options = LeptosOptions::from_ref(&state);

        let router = if let Some(handler) = error_handler.clone() {
            router.fallback_service(handler)
        } else {
            router
        };

        #[cfg(any(feature = "default", feature = "embed"))]
        let router = {
            let pkg_base = leptos_options.site_pkg_route_base();
            let pkg_route = StaticRoute::Mount(&pkg_base);
            let (router, pkg_mounted) = match self.site_pkg_mode {
                ResourceMode::Disable => (router, false),
                #[cfg(feature = "default")]
                ResourceMode::Filesystem => (
                    pkg_route.mount(
                        router,
                        serve_site_pkg_service(&leptos_options),
                        error_handler.clone(),
                        extra_cx,
                    ),
                    true,
                ),
                #[cfg(feature = "embed")]
                ResourceMode::Embed => {
                    if let Some(site_pkg) = self.site_pkg {
                        let router = pkg_route.mount(
                            router,
                            ServeDir::with_backend(
                                "/",
                                EmbeddedSiteRoot::new(site_pkg),
                            ),
                            error_handler.clone(),
                            extra_cx,
                        );
                        (router, true)
                    } else {
                        let path = leptos_options.site_pkg_path();
                        match path.strip_prefix(&*leptos_options.site_root) {
                            Ok(relative)
                                if !relative.components().any(|part| {
                                    part == Component::ParentDir
                                }) =>
                            {
                                let router = pkg_route.mount(
                                    router,
                                    ServeDir::with_backend(
                                        Path::new("/").join(relative),
                                        EmbeddedSiteRoot::new(self.site_root),
                                    ),
                                    error_handler.clone(),
                                    extra_cx,
                                );
                                (router, true)
                            }
                            _ => {
                                leptos::logging::error!(
                                    "Cannot mount pkg directory {} from \
                                     embedded site root {}. Embed the pkg \
                                     with enable_embed_site_pkg.",
                                    path.display(),
                                    leptos_options.site_root,
                                );
                                (router, false)
                            }
                        }
                    }
                }
            };

            let favicon_route = StaticRoute::Exact("/favicon.ico");
            let router = match self.favicon_mode {
                ResourceMode::Disable => router,
                #[cfg(feature = "default")]
                ResourceMode::Filesystem => favicon_route.mount(
                    router,
                    serve_site_root_service(&leptos_options),
                    error_handler.clone(),
                    extra_cx,
                ),
                #[cfg(feature = "embed")]
                ResourceMode::Embed => favicon_route.mount(
                    router,
                    ServeDir::with_backend(
                        "/",
                        EmbeddedSiteRoot::new(self.site_root),
                    ),
                    error_handler.clone(),
                    extra_cx,
                ),
            };

            let root_base = leptos_options.site_root_route_base();
            let same_mount_url = pkg_mounted
                && root_base == pkg_base
                && !matches!(self.site_root_mode, ResourceMode::Disable);
            if same_mount_url
                && leptos_options.site_pkg_path()
                    != Path::new(&*leptos_options.site_root)
            {
                leptos::logging::error!(
                    "The site root and pkg use the same URL ({root_base}). \
                     The pkg mount takes precedence."
                );
            }

            let root_route = if root_base == "/" {
                StaticRoute::Fallback
            } else {
                StaticRoute::Mount(&root_base)
            };
            match self.site_root_mode {
                // Two nests at the same path conflict. At /, the site root
                // remains the router's fallback service.
                _ if same_mount_url && root_base != "/" => router,
                ResourceMode::Disable => router,
                #[cfg(feature = "default")]
                ResourceMode::Filesystem => root_route.mount(
                    router,
                    serve_site_root_service(&leptos_options),
                    error_handler,
                    extra_cx,
                ),
                #[cfg(feature = "embed")]
                ResourceMode::Embed => root_route.mount(
                    router,
                    ServeDir::with_backend(
                        "/",
                        EmbeddedSiteRoot::new(self.site_root),
                    ),
                    error_handler,
                    extra_cx,
                ),
            }
        };

        router.with_state(state)
    }
}
