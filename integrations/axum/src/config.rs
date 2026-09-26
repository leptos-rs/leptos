//! Provides a builder and implementation for wholesale configuration of [`axum::Router`].

#[cfg(feature = "default")]
use crate::serve_site_root_service;
#[cfg(feature = "embed")]
use crate::service::EmbeddedSiteRoot;
use crate::{ErrorHandler, LeptosRoutes, generate_route_list};
#[cfg(any(feature = "default", feature = "embed"))]
use crate::{LeptosContextLayer, serve_site_root_service_route_path};
use axum::{Router, extract::FromRef};
use leptos::{IntoView, config::LeptosOptions};
#[cfg(feature = "embed")]
use rust_embed::{EmbeddedFile, RustEmbed};
#[cfg(any(feature = "default", feature = "embed"))]
use std::borrow::Cow;
#[cfg(any(feature = "default", feature = "embed"))]
use tower::builder::ServiceBuilder;
#[cfg(feature = "embed")]
use tower_http::services::ServeDir;

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
    /// Disables the serving of site pkg dir.
    #[default]
    Disable,
    /// Use the files found on the filesystem at runtime.
    #[cfg(feature = "default")]
    Filesystem,
    /// Build the compiled site pkg dir into the server binary.
    #[cfg(feature = "embed")]
    Embed,
}

/// The possible modes for serving of the assets for a [`RouterConfiguration`].
///
/// Assets are files copied to `LEPTOS_SITE_ROOT` if `LEPTOS_ASSETS_DIR` is configured.
#[derive(Clone, Default)]
enum AssetMode {
    /// Disables the serving of assets.
    #[default]
    Disable,
    /// Serves the assets directory using the [`ServeDir`] service created by [`serve_site_root_service`].
    /// If the provided path is `"/"`, it will become part of the fallback service, otherwise a new router
    /// will be created to serve this.
    ///
    /// [`ServeDir`]: tower_http::services::ServeDir
    #[cfg(feature = "default")]
    Filesystem(Cow<'static, str>),
    /// Serves the embedded assets using the [`ServeDir`] service with `EmbeddedSiteRoot` as the backend.
    /// If the provided path is `"/"`, it will become part of the fallback service, otherwise a new router
    /// will be created to serve this.
    ///
    /// [`ServeDir`]: tower_http::services::ServeDir
    #[cfg(feature = "embed")]
    Embed(Cow<'static, str>),
}

// The setup implementation uses these as intermediate values to define how the router will be set up with
// the relevant services at the defined routes.  While the support of multiple routes are not exposed, this
// could be handled in the future.
#[cfg(any(feature = "default", feature = "embed"))]
enum Site {
    #[cfg(feature = "default")]
    Filesystem(Cow<'static, str>),
    /// Build the compiled site pkg dir into the server binary.
    #[cfg(feature = "embed")]
    Embed(Cow<'static, str>),
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
#[derive(Clone)]
pub struct RouterConfiguration<
    APP,
    CX = fn(),
    SH = (),
    S = (),
    SR = NoEmbedSiteRoot,
> {
    app_fn: Option<APP>,
    shell: Option<SH>,
    state: Option<S>,
    extra_cx: Option<CX>,
    site_pkg_mode: ResourceMode,
    favicon_mode: ResourceMode,
    serve_asset: AssetMode,
    error_handler: bool,
    site_root: SR,
}

/// Create a new configuration with all toggles disabled.
impl<APP> Default for RouterConfiguration<APP> {
    fn default() -> Self {
        Self {
            app_fn: None,
            shell: None,
            state: None,
            extra_cx: None,
            site_pkg_mode: ResourceMode::Disable,
            favicon_mode: ResourceMode::Disable,
            serve_asset: AssetMode::Disable,
            error_handler: false,
            site_root: NoEmbedSiteRoot,
        }
    }
}

impl<APP> RouterConfiguration<APP> {
    /// Create a new configuration base with the commonly recommended values.
    ///
    /// When default features are enabled, this enables routing of the path defined by `LEPTOS_SITE_PKG` to
    /// the [`ServeDir`] service at `LEPTOS_SITE_ROOT`, with the [`Router`]'s fallback handler set to the
    /// [`ErrorHandler`] service.  A route to `/favicon.ico` is also provided to the corresponding file at
    /// `LEPTOS_SITE_ROOT`.  Refer to [`.enable_fs_site_pkg`] and [`.error_handler`] for additional details.
    ///
    /// Use of `RouterConfiguration::default()` does not have these additional routes and services enabled.
    /// Without default features enabled, this constructor is equivalent to that.
    ///
    /// [`ServeDir`]: tower_http::services::ServeDir
    /// [`.enable_fs_site_pkg`]: RouterConfiguration::enable_fs_site_pkg
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

            #[cfg(not(feature = "default"))]
            site_pkg_mode: ResourceMode::Disable,
            #[cfg(not(feature = "default"))]
            favicon_mode: ResourceMode::Disable,

            serve_asset: AssetMode::Disable,
            error_handler: true,

            site_root: NoEmbedSiteRoot,
        }
    }

    /// A configuration base that has a fallback handler to serve the entirety of `LEPTOS_SITE_PKG`.
    ///
    /// Create a new configuration to set up a [`Router`] that routes the path defined by `LEPTOS_SITE_PKG` to
    /// the [`ServeDir`] service at `LEPTOS_SITE_ROOT`, with that service also serving as the default fallback
    /// handler, with an [`ErrorHandler`] service being the ultimate fallback handler.  This should fully
    /// replicate the `file_and_error_handler_with_context` handler.
    ///
    /// Refer to [`.enable_fs_leptos_site_root`], [`.enable_fs_site_pkg`], and [`.error_handler`] for
    /// additional details.
    ///
    /// [`ServeDir`]: tower_http::services::ServeDir
    /// [`.enable_fs_leptos_site_root`]: RouterConfiguration::enable_fs_leptos_site_root
    /// [`.enable_fs_site_pkg`]: RouterConfiguration::enable_fs_site_pkg
    /// [`.error_handler`]: RouterConfiguration::error_handler
    #[cfg(feature = "default")]
    pub fn new_with_assets() -> Self {
        Self {
            app_fn: None,
            shell: None,
            state: None,
            extra_cx: None,
            site_pkg_mode: ResourceMode::Filesystem,
            // TODO verify how this value may conflict with the setting defined in `serve_asset` as it may
            // remain in `"/"` but also be configured to something else.
            favicon_mode: ResourceMode::Filesystem,
            serve_asset: AssetMode::Filesystem("/".into()),
            error_handler: true,
            site_root: NoEmbedSiteRoot,
        }
    }
}

#[cfg(feature = "embed")]
impl<APP, SR> RouterConfiguration<APP, fn(), (), (), SR>
where
    SR: RustEmbed,
{
    /// The embedded counterpart to [`RouterConfiguration::new()`].
    ///
    /// Create a new configuration with the provided `site_root` to be converted into an `EmbeddedSiteRoot`
    /// backend for [`ServeDir`], in order to enable a compiled server binary to serve the site pkg that is
    /// embedded within it.
    ///
    /// This enables the routing of the path defined by `LEPTOS_SITE_PKG` to the aforementioned `ServeDir`
    /// service, with the [`Router`]'s fallback handler set to the [`ErrorHandler`] service.  A route to
    /// `/favicon.ico` is also provided to route to the corresponding embedded resource through the `ServeDir`
    /// service.
    ///
    /// Please note that `site_root` must be a [`RustEmbed`] implementation derived from [`Embed`], and it
    /// must be constructed like so within the target application:
    ///
    /// ```rust
    /// use leptos_axum::rust_embed::{self, Embed};
    ///
    /// #[derive(Clone, Copy, Embed)]
    /// #[folder = "$LEPTOS_SITE_ROOT"]
    /// #[prefix = "/"]
    /// # #[allow_missing = true]
    /// struct SiteRoot;
    /// ```
    ///
    /// Also note that `RustEmbed` requires that the files are available before they may be included.  The
    /// underlying build tooling (e.g. `cargo-leptos`) may need to be invoked in a manner to ensure that the
    /// frontend is compiled before the server, otherwise the resulting server binary may lack the required
    /// data for the serving of the frontend client.
    ///
    /// Another thing to note is that the configuration of the `RustEmbed` must be done on `LEPTOS_SITE_ROOT`,
    /// in order to cater to the most general case where data beyond the site package may be included; this
    /// has the unfortunate consequence of data included with the site root that might never be routed.  For
    /// example, if routing to `favicon.ico` is disabled, that file will still be embedded.  This may be
    /// refined in a future version.
    ///
    /// Refer to [`.enable_embed_site_pkg`] and [`.error_handler`] for additional details.
    ///
    /// [`Embed`]: rust_embed::Embed
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
            serve_asset: AssetMode::Disable,
            error_handler: true,
            site_root,
        }
    }

    /// The embedded counterpart to [`RouterConfiguration::new_with_assets()`].
    ///
    /// Create a new configuration with the provided `site_root` to be converted into an `EmbeddedSiteRoot`
    /// backend for [`ServeDir`], in order to enable a compiled server binary to serve site root data that is
    /// embedded within it.
    ///
    /// This enables the routing of the path defined by `LEPTOS_SITE_PKG` to the aforementioned `ServeDir`
    /// service, with that also acting as the default fallback handler, with its [`ErrorHandler`] service
    /// being the ultimate fallback handler.  This replicates the `file_and_error_handler_with_context`
    /// handler but with content served being embedded into the server binary.
    ///
    /// Please note that `site_root` must be a [`RustEmbed`] implementation derived from [`Embed`], and it
    /// must be constructed like so within the target application:
    ///
    /// ```rust
    /// use leptos_axum::rust_embed::{self, Embed};
    ///
    /// #[derive(Clone, Copy, Embed)]
    /// #[folder = "$LEPTOS_SITE_ROOT"]
    /// #[prefix = "/"]
    /// # #[allow_missing = true]
    /// struct SiteRoot;
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
            serve_asset: AssetMode::Embed("/".into()),
            error_handler: true,
            site_root,
        }
    }
}

impl<APP, CX, SH, S, SR> RouterConfiguration<APP, CX, SH, S, SR> {
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
    ) -> RouterConfiguration<APP, CX, SH2, S, SR>
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
            serve_asset: self.serve_asset,
            error_handler: self.error_handler,
            site_root: self.site_root,
        }
    }

    /// Provide an additional context to set up Leptos routes with.  This is optional.
    ///
    /// The provided closure will be applied to all underlying services.
    pub fn with_context<CX2>(
        self,
        extra_cx: CX2,
    ) -> RouterConfiguration<APP, CX2, SH, S, SR>
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
            serve_asset: self.serve_asset,
            error_handler: self.error_handler,
            site_root: self.site_root,
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
    ) -> RouterConfiguration<APP, CX, SH, S2, SR>
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
            serve_asset: self.serve_asset,
            error_handler: self.error_handler,
            site_root: self.site_root,
        }
    }
}

#[cfg(feature = "default")]
impl<APP, CX, SH, S, SR> RouterConfiguration<APP, CX, SH, S, SR> {
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
    /// [`ErrorHandler`] service if it is also available.  Otherwise [`Router::route_service`] will be used
    /// to set this service up.
    ///
    /// This configuration is not meant for setting up multiple paths to multiple `ServeDir` services; if
    /// that is required, please do so on the resulting `Router`.
    pub fn enable_fs_leptos_site_root(
        self,
        path: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.serve_asset(AssetMode::Filesystem(path.into()))
    }

    /// Disable the routing of `LEPTOS_SITE_ROOT`.
    pub fn disable_leptos_site_root(self) -> Self {
        self.serve_asset(AssetMode::Disable)
    }

    /// Configure the [`ResourceMode`] to serve the site pkg with.
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
    ///
    /// [`ServeDir`]: tower_http::services::ServeDir
    #[cfg(feature = "default")]
    pub fn enable_fs_site_pkg(self) -> Self {
        self.site_pkg_mode(ResourceMode::Filesystem)
    }

    // TODO See if just the site pkg may be included to avoid embedding the whole site root, i.e. this will
    // ignore the embedding of `/favicon.ico`.  If so that should be documented here as the more advanced and
    // optimized option.
    /// Enable the routing of files in the `LEPTOS_SITE_PKG` subdirectory within the provided embedded site
    /// root, which will be converted to a [`ServeDir`] service with it as the backend to be set up at
    /// runtime.
    ///
    /// This is used to serve the JS/WASM bundle embedded in the server binary, such that the application will
    /// be activated on the client.
    ///
    /// This may be used in conjunction with the other `enable_fs` prefixed configurations, such that
    /// other additional data may be provided from the filesystem through the relevant `ServeDir` service that
    /// will be set up.
    ///
    /// For the most common use cases (i.e. where the intent is to have only one source of files be embedded),
    /// the more convenient methods to set this up may be through [`RouterConfiguration::embed`] or
    /// [`RouterConfiguration::embed_with_assets`].  For complete details, including the caveats of enabling
    /// the embedded site root, please refer to the documentation for those two constructors.
    ///
    /// This configuration will also supercede any previous [`.enable_embed_leptos_site_root`] method calls.
    /// Currently only one set of `RustEmbed` contents is supported with this configuration.  If the site root
    /// is to be of a different source from the site pkg, the embedded site root must be set up separately on
    /// the resulting `Router` with the corresponding service.
    ///
    /// [`ServeDir`]: tower_http::services::ServeDir
    /// [`.enable_embed_leptos_site_root`]: RouterConfiguration::enable_embed_leptos_site_root
    #[cfg(feature = "embed")]
    pub fn enable_embed_site_pkg<SR2>(
        self,
        site_root: SR2,
    ) -> RouterConfiguration<APP, CX, SH, S, SR2>
    where
        SR2: Clone + Copy + Send + Sync + RustEmbed + 'static,
    {
        RouterConfiguration {
            app_fn: self.app_fn,
            shell: self.shell,
            state: self.state,
            extra_cx: self.extra_cx,
            site_pkg_mode: ResourceMode::Embed,
            favicon_mode: self.favicon_mode,
            serve_asset: self.serve_asset,
            error_handler: self.error_handler,
            site_root,
        }
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
    #[cfg(feature = "default")]
    pub fn enable_fs_favicon(self) -> Self {
        self.favicon_mode(ResourceMode::Filesystem)
    }

    /// Enable the routing of `favicon.ico` in the `LEPTOS_SITE_PKG` on the embedded site root.
    #[cfg(feature = "embed")]
    pub fn enable_embed_favicon(self) -> Self {
        self.favicon_mode(ResourceMode::Embed)
    }

    /// Disables the routing of `favicon.ico`
    pub fn disable_favicon(self) -> Self {
        self.favicon_mode(ResourceMode::Disable)
    }
}

#[cfg(feature = "embed")]
impl<APP, CX, SH, S, SR> RouterConfiguration<APP, CX, SH, S, SR>
where
    SR: Clone + Copy + Send + Sync + RustEmbed + 'static,
{
    /// Configure the base route for the [`ServeDir`] service with the `EmbeddedSiteRoot` backend for serving
    /// of the embedded files.
    ///
    /// If the provided path is `"/"`, the fallback service will be used instead, in conjunction with the
    /// [`ErrorHandler`] service if it is also available.  Otherwise [`Router::route_service`] will be used
    /// to set this service up.
    ///
    /// This configuration is not meant for setting up multiple paths to multiple `ServeDir` services; if
    /// that is required, please do so on the resulting `Router`.
    ///
    /// This configuration will also supercede any previous [`.enable_embed_site_pkg`] method calls.
    /// Currently only one set of `RustEmbed` contents is supported with this configuration.  If the site root
    /// is to be of a different source from the site pkg, the embedded site root must be set up separately on
    /// the resulting `Router` with the corresponding service.
    ///
    /// [`ServeDir`]: tower_http::services::ServeDir
    /// [`.enable_embed_site_pkg`]: RouterConfiguration::enable_embed_site_pkg
    pub fn enable_embed_leptos_site_root<SR2>(
        self,
        path: impl Into<Cow<'static, str>>,
        site_root: SR2,
    ) -> RouterConfiguration<APP, CX, SH, S, SR2>
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
            serve_asset: AssetMode::Embed(path.into()),
            error_handler: self.error_handler,
            site_root,
        }
    }

    /// Set the base route for a configuration that already has a site root configured to be embedded.
    ///
    /// If the provided path is `"/"`, the fallback service will be used instead, in conjunction with the
    /// [`ErrorHandler`] service if it is also available.  Otherwise [`Router::route_service`] will be used
    /// to set this service up.
    ///
    /// [`ServeDir`]: tower_http::services::ServeDir
    pub fn set_embed_leptos_site_root_path(
        self,
        path: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.serve_asset(AssetMode::Embed(path.into()))
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

impl<APP, CX, SH, S, SR, IV1, IV2> traits::RouterConfiguration<S>
    for RouterConfiguration<APP, CX, SH, S, SR>
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

        #[cfg(any(feature = "default", feature = "embed"))]
        let router = 'router: {
            let mut site_pkg_routes = Vec::new();

            match self.site_pkg_mode {
                ResourceMode::Disable => (),
                #[cfg(feature = "default")]
                ResourceMode::Filesystem => {
                    site_pkg_routes.push(Site::Filesystem(
                        serve_site_root_service_route_path(&leptos_options)
                            .into(),
                    ))
                }
                #[cfg(feature = "embed")]
                ResourceMode::Embed => site_pkg_routes.push(Site::Embed(
                    serve_site_root_service_route_path(&leptos_options).into(),
                )),
            };

            match self.favicon_mode {
                ResourceMode::Disable => (),
                #[cfg(feature = "default")]
                ResourceMode::Filesystem => site_pkg_routes
                    .push(Site::Filesystem("/favicon.ico".into())),
                #[cfg(feature = "embed")]
                ResourceMode::Embed => {
                    site_pkg_routes.push(Site::Embed("/favicon.ico".into()))
                }
            };

            // if using static assets, need the interpolate feature

            if site_pkg_routes.is_empty() {
                break 'router router;
            }

            let builder = ServiceBuilder::new().option_layer(
                extra_cx.map(LeptosContextLayer::new_with_context),
            );

            site_pkg_routes
                .into_iter()
                .fold(router, |router, entry: Site| match entry {
                    #[cfg(feature = "default")]
                    Site::Filesystem(path) => {
                        let serve_dir =
                            serve_site_root_service(&leptos_options);
                        if let Some(error_handler) = error_handler.clone() {
                            router.route_service(
                                &path,
                                builder.service(
                                    serve_dir.clone().fallback(error_handler),
                                ),
                            )
                        } else {
                            router.route_service(
                                &path,
                                builder.service(serve_dir.clone()),
                            )
                        }
                    }
                    #[cfg(feature = "embed")]
                    Site::Embed(path) => {
                        let serve_dir = ServeDir::with_backend(
                            "/",
                            EmbeddedSiteRoot::new(self.site_root),
                        );
                        if let Some(error_handler) = error_handler.clone() {
                            router.route_service(
                                &path,
                                builder.service(
                                    serve_dir.clone().fallback(error_handler),
                                ),
                            )
                        } else {
                            router.route_service(
                                &path,
                                builder.service(serve_dir.clone()),
                            )
                        }
                    }
                })
        };

        // While the one set up for `site_pkg_mode` may be used, it might not be configured and so
        // reusing that clone may be problematic; much easier to create one just for here; maybe refactor
        // this later when implementation is more settled.
        #[cfg(feature = "default")]
        let builder = ServiceBuilder::new()
            .option_layer(extra_cx.map(LeptosContextLayer::new_with_context));

        let router = if let Some(error_handler) = error_handler {
            match self.serve_asset {
                #[cfg(feature = "default")]
                AssetMode::Filesystem(path) if path == "/" => router
                    .fallback_service(
                        builder.service(
                            serve_site_root_service(&leptos_options)
                                .fallback(error_handler),
                        ),
                    ),
                #[cfg(feature = "default")]
                AssetMode::Filesystem(path) => router
                    .nest(
                        &path,
                        Router::new().route_service(
                            "/{*path}",
                            builder.service(
                                serve_site_root_service(&leptos_options)
                                    .fallback(error_handler.clone()),
                            ),
                        ),
                    )
                    .fallback_service(error_handler),
                #[cfg(feature = "embed")]
                AssetMode::Embed(path) if path == "/" => router
                    .fallback_service(
                        builder.service(
                            ServeDir::with_backend(
                                "/",
                                EmbeddedSiteRoot::new(self.site_root),
                            )
                            .fallback(error_handler),
                        ),
                    ),
                #[cfg(feature = "embed")]
                AssetMode::Embed(path) => router
                    .nest(
                        &path,
                        Router::new().route_service(
                            "/{*path}",
                            builder.service(
                                ServeDir::with_backend(
                                    "/",
                                    EmbeddedSiteRoot::new(self.site_root),
                                )
                                .fallback(error_handler.clone()),
                            ),
                        ),
                    )
                    .fallback_service(error_handler),
                AssetMode::Disable => router.fallback_service(error_handler),
            }
        } else {
            match self.serve_asset {
                #[cfg(feature = "default")]
                AssetMode::Filesystem(path) if path == "/" => router
                    .fallback_service(
                        builder
                            .service(serve_site_root_service(&leptos_options)),
                    ),
                #[cfg(feature = "default")]
                AssetMode::Filesystem(path) => router.nest(
                    &path,
                    Router::new().route_service(
                        "/{*path}",
                        builder
                            .service(serve_site_root_service(&leptos_options)),
                    ),
                ),
                #[cfg(feature = "embed")]
                AssetMode::Embed(path) if path == "/" => router
                    .fallback_service(builder.service(ServeDir::with_backend(
                        "/",
                        EmbeddedSiteRoot::new(self.site_root),
                    ))),
                #[cfg(feature = "embed")]
                AssetMode::Embed(path) => router.nest(
                    &path,
                    Router::new().route_service(
                        "/{*path}",
                        builder.service(ServeDir::with_backend(
                            "/",
                            EmbeddedSiteRoot::new(self.site_root),
                        )),
                    ),
                ),
                AssetMode::Disable => router,
            }
        };

        router.with_state(state)
    }
}
