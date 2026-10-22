//! Provides a builder and implementation for wholesale configuration of [`axum::Router`].

#[cfg(feature = "default")]
use crate::serve_site_root_service;
#[cfg(feature = "embed")]
use crate::service::{EmbeddedSiteRoot, ExcludeDir};
use crate::{ErrorHandler, LeptosRoutes, generate_route_list};
#[cfg(any(feature = "default", feature = "embed"))]
use crate::{LeptosContextLayer, serve_site_root_service_route_path};
use axum::{Router, extract::FromRef};
use leptos::{IntoView, config::LeptosOptions};
#[cfg(feature = "embed")]
use rust_embed::{EmbeddedFile, RustEmbed};
#[cfg(any(feature = "default", feature = "embed"))]
use std::borrow::Cow;
#[cfg(feature = "embed")]
use std::path::Path;
#[cfg(feature = "default")]
use std::path::PathBuf;
#[cfg(any(feature = "default", feature = "embed"))]
use tower::builder::ServiceBuilder;
#[cfg(any(feature = "default", feature = "embed"))]
use tower_http::services::ServeDir;
#[cfg(feature = "embed")]
use tower_http::services::fs::DefaultServeDirFallback;

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
    #[cfg(feature = "default")]
    FilesystemNested { url: String, dir: PathBuf },
    #[cfg(feature = "embed")]
    EmbedNested { url: String, base: String },
    /// The site pkg embedded on its own, served at `url`.
    #[cfg(feature = "embed")]
    EmbedSitePkg { url: String },
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
    SP = NoEmbedSiteRoot,
> {
    app_fn: Option<APP>,
    shell: Option<SH>,
    state: Option<S>,
    extra_cx: Option<CX>,
    site_pkg_mode: ResourceMode,
    favicon_mode: ResourceMode,
    serve_asset: ResourceMode,
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
            serve_asset: ResourceMode::Disable,
            error_handler: false,
            site_root: NoEmbedSiteRoot,
            site_pkg: None,
        }
    }
}

impl<APP> RouterConfiguration<APP> {
    /// Create a new configuration base with the commonly recommended values.
    ///
    /// With default features enabled, this constructor will enable the routing of the site pkg URL,
    /// `LEPTOS_SITE_PKG_URL`, to a [`ServeDir`] service for `LEPTOS_SITE_PKG_DIR`, with the [`Router`]'s
    /// fallback handler set to the [`ErrorHandler`] service.  A route to `/favicon.ico` is also provided to
    /// the corresponding file at `LEPTOS_SITE_ROOT`.  Refer to [`.enable_fs_site_pkg`] and [`.error_handler`]
    /// for additional details.
    ///
    /// Without default features enabled, this constructor is equivalent to `RouterConfiguration::default()`,
    /// as it does not have any additional routes, options, and services enabled.
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

            serve_asset: ResourceMode::Disable,
            error_handler: true,

            site_root: NoEmbedSiteRoot,
            site_pkg: None,
        }
    }

    /// A configuration base that has a [`ServeDir`] serving `LEPTOS_SITE_ROOT` at `LEPTOS_SITE_ROOT_URL`.
    ///
    /// Create a new configuration base that sets up a [`Router`] that has a `ServeDir` serving the
    /// `LEPTOS_SITE_ROOT` (as the default fallback service when `LEPTOS_SITE_ROOT_URL` is `/`), with its
    /// fallback service being an [`ErrorHandler`] acting as the ultimate fallback handler.  Additionally, a
    /// route for the site pkg URL, `LEPTOS_SITE_PKG_URL`, will be added to a [`ServeDir`] service for
    /// `LEPTOS_SITE_PKG_DIR`, avoiding the dependency on the fallback handler for serving of the site pkg.  If
    /// the resulting `Router` is reconfigured by merging or otherwise causing the fallback service be replaced,
    /// it won't result in the loss of site pkg availability given the explicit route definition.
    ///
    /// `LEPTOS_SITE_PKG_DIR` may be inside `LEPTOS_SITE_ROOT`, outside it, or absolute.  The site root
    /// service does not serve the [`hidden_site_pkg_dir`](LeptosOptions::hidden_site_pkg_dir), so the site
    /// pkg has one URL.
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
            serve_asset: ResourceMode::Filesystem,
            error_handler: true,
            site_root: NoEmbedSiteRoot,
            site_pkg: None,
        }
    }
}

// Future work may include the following:
//
// - Runtime configuration/remapping of the site pkg.
// - Leptos specific derive macro to set up the `RustEmbed`.
#[cfg(feature = "embed")]
impl<APP, SR> RouterConfiguration<APP, fn(), (), (), SR>
where
    SR: RustEmbed,
{
    /// The embedded counterpart to [`RouterConfiguration::new()`].
    ///
    /// Create a new configuration with the provided `site_root` to be converted into an `EmbeddedSiteRoot`
    /// backend for [`ServeDir`], in order to serve the embedded site pkg provided by that `site_root`.
    /// The [`Router`] will route the site pkg URL, `LEPTOS_SITE_PKG_URL`, to the aforementioned
    /// `ServeDir` service, with the `Router`'s fallback handler set to the [`ErrorHandler`] service.  A route
    /// to `/favicon.ico` is also provided to route to the corresponding embedded resource through the same
    /// `ServeDir` service.
    ///
    /// The site pkg files are looked up at the place of `LEPTOS_SITE_PKG_DIR` in the embedded site root.
    /// A `LEPTOS_SITE_PKG_DIR` outside `LEPTOS_SITE_ROOT` must be embedded on its own with
    /// [`.enable_embed_site_pkg`].
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
    /// # let state = leptos::config::LeptosOptions::builder().output_name("test").build().unwrap();
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
            serve_asset: ResourceMode::Disable,
            error_handler: true,
            site_root,
            site_pkg: None,
        }
    }

    /// The embedded counterpart to [`RouterConfiguration::new_with_assets()`].
    ///
    /// Create a new configuration with the provided `site_root` to be converted into an `EmbeddedSiteRoot`
    /// backend for [`ServeDir`], in order to serve the embedded data provided by that `site_root`.  This
    /// service is set up at `LEPTOS_SITE_ROOT_URL` (as a fallback service on the [`Router`] when that is
    /// `/`), with its fallback service being an [`ErrorHandler`] acting as the ultimate fallback handler.
    /// Additionally, a route for the site pkg URL, `LEPTOS_SITE_PKG_URL`, will be added to serve the site pkg
    /// that may be embedded within.  This additional explicit route avoids the dependency on the fallback
    /// handler for serving of the site pkg.  If the resulting `Router` is reconfigured by merging or
    /// otherwise causing the fallback service be replaced, it won't result in the loss of site pkg
    /// availability given the explicit route definition.
    ///
    /// The site pkg files are looked up at the place of `LEPTOS_SITE_PKG_DIR` in the embedded site root.
    /// A `LEPTOS_SITE_PKG_DIR` outside `LEPTOS_SITE_ROOT` must be embedded on its own with
    /// [`.enable_embed_site_pkg`].
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
    /// # let state = leptos::config::LeptosOptions::builder().output_name("test").build().unwrap();
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
    /// [`.enable_embed_site_pkg`]: RouterConfiguration::enable_embed_site_pkg
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
            serve_asset: ResourceMode::Embed,
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
            serve_asset: self.serve_asset,
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
            serve_asset: self.serve_asset,
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
            serve_asset: self.serve_asset,
            error_handler: self.error_handler,
            site_root: self.site_root,
            site_pkg: self.site_pkg,
        }
    }
}

impl<APP, CX, SH, S, SR, SP> RouterConfiguration<APP, CX, SH, S, SR, SP> {
    /// Configure the [`ResourceMode`] to serve the assets with.
    ///
    /// When not disabled, the underlying `LeptosOptions` will be referenced along the configured mode to
    /// provide the relevant route or configure the appropriate fallback service to serve the assets.
    fn serve_asset(mut self, v: ResourceMode) -> Self {
        self.serve_asset = v;
        self
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

    /// Configure how the `favicon.ico` is served.
    fn favicon_mode(mut self, v: ResourceMode) -> Self {
        self.favicon_mode = v;
        self
    }

    /// Disable setting up of the routing of files in `LEPTOS_SITE_ROOT`.
    pub fn disable_leptos_site_root(self) -> Self {
        self.serve_asset(ResourceMode::Disable)
    }

    /// Disables setting up of the routing of files in `LEPTOS_SITE_PKG_DIR`.
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
    /// Enable the `ServeDir` service that will provide the files found in the `LEPTOS_SITE_ROOT` defined
    /// at runtime, at the base route `LEPTOS_SITE_ROOT_URL`.
    ///
    /// If the base route is `"/"`, the fallback service will be used instead, in conjunction with the
    /// [`ErrorHandler`] service if it is also available.  Otherwise the service is nested at the base route.
    ///
    /// This configuration is not meant for setting up multiple paths to multiple `ServeDir` services; if
    /// that is required, please do so on the resulting `Router`.
    ///
    /// The service does not serve the [`hidden_site_pkg_dir`](LeptosOptions::hidden_site_pkg_dir), so
    /// the site pkg has one URL.
    pub fn enable_fs_leptos_site_root(self) -> Self {
        self.serve_asset(ResourceMode::Filesystem)
    }

    /// Enable the routing of the site pkg URL, `LEPTOS_SITE_PKG_URL`, to a [`ServeDir`] service set up at
    /// runtime for `LEPTOS_SITE_PKG_DIR` on the filesystem.
    ///
    /// This is used to serve the JS/WASM bundle such that the application will be activated on the client.
    ///
    /// `LEPTOS_SITE_PKG_DIR` may be inside `LEPTOS_SITE_ROOT`, outside it, or absolute.
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
    /// Enable the [`ServeDir`] service with the `EmbeddedSiteRoot` backend for serving of the embedded
    /// files, at the base route `LEPTOS_SITE_ROOT_URL`.
    ///
    /// If the base route is `"/"`, the service will be set up as a fallback service, in conjunction with
    /// the [`ErrorHandler`] service if it is also available.  Otherwise the service is nested at the base
    /// route.
    ///
    /// This configuration is not meant for setting up multiple paths to multiple `ServeDir` services; if
    /// that is required, please do so on the resulting `Router`.
    ///
    /// This configuration replaces any embedded site root provided before.  The site pkg is also looked up
    /// in this site root, unless [`.enable_embed_site_pkg`] embeds the site pkg on its own.
    ///
    /// The service does not serve the [`hidden_site_pkg_dir`](LeptosOptions::hidden_site_pkg_dir), so
    /// the site pkg has one URL.
    ///
    /// [`ServeDir`]: tower_http::services::ServeDir
    /// [`.enable_embed_site_pkg`]: RouterConfiguration::enable_embed_site_pkg
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
            serve_asset: ResourceMode::Embed,
            error_handler: self.error_handler,
            site_root,
            site_pkg: self.site_pkg,
        }
    }

    /// Enable the routing of the site pkg embedded on its own in the provided `site_pkg`, which holds the
    /// files of `LEPTOS_SITE_PKG_DIR` at its root.  It will be converted to a [`ServeDir`] service with it
    /// as the backend, set up at runtime on the base route of the site pkg.
    ///
    /// This is used to serve the JS/WASM bundle embedded in the server binary, such that the application will
    /// be activated on the client.  As the site pkg is embedded on its own, `LEPTOS_SITE_PKG_DIR` may be
    /// inside or outside `LEPTOS_SITE_ROOT`, and the embedded site root of this configuration still serves
    /// the other files.
    ///
    /// This may be used in conjunction with the other `enable_fs` prefixed configurations, such that
    /// other additional data may be provided from the filesystem through the relevant `ServeDir` service that
    /// will be set up, though the application may elect to set up their own `ServeDir` or other services
    /// directly on the `Router` as that may be the more suitable and direct option.
    ///
    /// For the most common use cases (i.e. where the intent is to have only one source of files be embedded),
    /// the more convenient methods to set this up may be through [`RouterConfiguration::embed`] or
    /// [`RouterConfiguration::embed_with_assets`].  For complete details, including the caveats of enabling
    /// the embedded site root, please refer to the documentation for those two constructors.
    ///
    /// A more size optimized setup may be achieved using this method alone, assuming no other data is
    /// required from `LEPTOS_SITE_ROOT` (e.g. the `/favicon.ico` route is to be provided by some other
    /// service).  The `RustEmbed` may be derived like so, with `folder` set to the path of
    /// `LEPTOS_SITE_PKG_DIR` instead if that is absolute:
    ///
    /// ```rust
    /// use leptos_axum::{
    ///     RouterConfiguration,
    ///     rust_embed::{self, Embed},
    /// };
    ///
    /// #[derive(Clone, Copy, Embed)]
    /// #[folder = "$LEPTOS_SITE_ROOT/$LEPTOS_SITE_PKG_DIR"]
    /// #[prefix = "/"]
    /// # #[allow_missing = true]
    /// struct SitePkg;
    ///
    /// # let (app, shell) = (|| (), |_| ());
    /// # let state = leptos::config::LeptosOptions::builder().output_name("test").build().unwrap();
    /// // Usage of the above derive may be like so:
    /// let config = RouterConfiguration::default()
    ///     .enable_embed_site_pkg(SitePkg)
    ///     .error_handler(true)
    ///     .app(app)
    ///     .shell(shell)
    ///     .state(state);
    /// ```
    ///
    /// [`ServeDir`]: tower_http::services::ServeDir
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
            serve_asset: self.serve_asset,
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

/// Serves the embedded site root, without the hidden site pkg dir.
#[cfg(feature = "embed")]
fn serve_embedded_site_root<SR>(
    site_root: SR,
    options: &LeptosOptions,
) -> ServeDir<DefaultServeDirFallback, ExcludeDir<EmbeddedSiteRoot<SR>>>
where
    SR: RustEmbed + Clone + Send + Sync + 'static,
{
    let hidden = options
        .hidden_site_pkg_dir()
        .map(|dir| Path::new("/").join(dir));
    ServeDir::with_backend(
        "/",
        ExcludeDir::new(EmbeddedSiteRoot::new(site_root), hidden),
    )
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

        #[cfg(any(feature = "default", feature = "embed"))]
        let router = 'router: {
            let mut site_pkg_routes = Vec::new();

            match self.site_pkg_mode {
                ResourceMode::Disable => (),
                // A set `site_pkg_url` moves the site pkg route off `site_pkg_dir`; an
                // empty one keeps it there, as `Router::nest` cannot nest at the root.
                #[cfg(feature = "default")]
                ResourceMode::Filesystem
                    if leptos_options.site_pkg_url.is_some()
                        && !leptos_options.pkg_url_path().is_empty() =>
                {
                    site_pkg_routes.push(Site::FilesystemNested {
                        url: format!("/{}", leptos_options.pkg_url_path()),
                        dir: leptos_options.site_pkg_path(),
                    })
                }
                #[cfg(feature = "embed")]
                ResourceMode::Embed
                    if self.site_pkg.is_some()
                        && !leptos_options.pkg_url_path().is_empty() =>
                {
                    site_pkg_routes.push(Site::EmbedSitePkg {
                        url: format!("/{}", leptos_options.pkg_url_path()),
                    })
                }
                #[cfg(feature = "embed")]
                ResourceMode::Embed
                    if leptos_options.site_pkg_url.is_some()
                        && !leptos_options.pkg_url_path().is_empty() =>
                {
                    // The embedded site root holds the pkg at its place there.
                    match leptos_options.site_pkg_dir_in_site_root() {
                        Some(place) => {
                            site_pkg_routes.push(Site::EmbedNested {
                                url: format!(
                                    "/{}",
                                    leptos_options.pkg_url_path()
                                ),
                                base: format!("/{place}"),
                            })
                        }
                        None => leptos::logging::error!(
                            "the embedded site root cannot hold a pkg dir \
                             outside site_root; embed it with \
                             `enable_embed_site_pkg` to serve it"
                        ),
                    }
                }
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
                    Site::FilesystemNested { url, dir } => {
                        let serve_dir = ServeDir::new(&dir)
                            .precompressed_gzip()
                            .precompressed_br();
                        let inner = if let Some(error_handler) =
                            error_handler.clone()
                        {
                            Router::new().route_service(
                                "/{*path}",
                                builder
                                    .service(serve_dir.fallback(error_handler)),
                            )
                        } else {
                            Router::new().route_service(
                                "/{*path}",
                                builder.service(serve_dir),
                            )
                        };
                        router.nest(&url, inner)
                    }
                    #[cfg(feature = "embed")]
                    Site::EmbedNested { url, base } => {
                        let serve_dir = ServeDir::with_backend(
                            base,
                            EmbeddedSiteRoot::new(self.site_root),
                        );
                        let inner = if let Some(error_handler) =
                            error_handler.clone()
                        {
                            Router::new().route_service(
                                "/{*path}",
                                builder
                                    .service(serve_dir.fallback(error_handler)),
                            )
                        } else {
                            Router::new().route_service(
                                "/{*path}",
                                builder.service(serve_dir),
                            )
                        };
                        router.nest(&url, inner)
                    }
                    #[cfg(feature = "embed")]
                    Site::EmbedSitePkg { url } => {
                        let Some(site_pkg) = self.site_pkg else {
                            return router;
                        };
                        let serve_dir = ServeDir::with_backend(
                            "/",
                            EmbeddedSiteRoot::new(site_pkg),
                        );
                        let inner = if let Some(error_handler) =
                            error_handler.clone()
                        {
                            Router::new().route_service(
                                "/{*path}",
                                builder
                                    .service(serve_dir.fallback(error_handler)),
                            )
                        } else {
                            Router::new().route_service(
                                "/{*path}",
                                builder.service(serve_dir),
                            )
                        };
                        router.nest(&url, inner)
                    }
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
                        let serve_dir = serve_embedded_site_root(
                            self.site_root,
                            &leptos_options,
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
        #[cfg(any(feature = "default", feature = "embed"))]
        let builder = ServiceBuilder::new()
            .option_layer(extra_cx.map(LeptosContextLayer::new_with_context));
        #[cfg(any(feature = "default", feature = "embed"))]
        let path = format!("/{}", leptos_options.site_root_url_path());

        let router = if let Some(error_handler) = error_handler {
            match self.serve_asset {
                #[cfg(feature = "default")]
                ResourceMode::Filesystem if path == "/" => router
                    .fallback_service(
                        builder.service(
                            serve_site_root_service(&leptos_options)
                                .fallback(error_handler),
                        ),
                    ),
                #[cfg(feature = "default")]
                ResourceMode::Filesystem => router
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
                ResourceMode::Embed if path == "/" => router.fallback_service(
                    builder.service(
                        serve_embedded_site_root(
                            self.site_root,
                            &leptos_options,
                        )
                        .fallback(error_handler),
                    ),
                ),
                #[cfg(feature = "embed")]
                ResourceMode::Embed => router
                    .nest(
                        &path,
                        Router::new().route_service(
                            "/{*path}",
                            builder.service(
                                serve_embedded_site_root(
                                    self.site_root,
                                    &leptos_options,
                                )
                                .fallback(error_handler.clone()),
                            ),
                        ),
                    )
                    .fallback_service(error_handler),
                ResourceMode::Disable => router.fallback_service(error_handler),
            }
        } else {
            match self.serve_asset {
                #[cfg(feature = "default")]
                ResourceMode::Filesystem if path == "/" => router
                    .fallback_service(
                        builder
                            .service(serve_site_root_service(&leptos_options)),
                    ),
                #[cfg(feature = "default")]
                ResourceMode::Filesystem => router.nest(
                    &path,
                    Router::new().route_service(
                        "/{*path}",
                        builder
                            .service(serve_site_root_service(&leptos_options)),
                    ),
                ),
                #[cfg(feature = "embed")]
                ResourceMode::Embed if path == "/" => router.fallback_service(
                    builder.service(serve_embedded_site_root(
                        self.site_root,
                        &leptos_options,
                    )),
                ),
                #[cfg(feature = "embed")]
                ResourceMode::Embed => router.nest(
                    &path,
                    Router::new().route_service(
                        "/{*path}",
                        builder.service(serve_embedded_site_root(
                            self.site_root,
                            &leptos_options,
                        )),
                    ),
                ),
                ResourceMode::Disable => router,
            }
        };

        router.with_state(state)
    }
}
