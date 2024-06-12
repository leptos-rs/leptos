#[cfg(feature = "ssr")]
use crate::{RouteListing, RouterIntegrationContext, ServerIntegration};
#[cfg(feature = "ssr")]
use leptos::{create_runtime, provide_context, IntoView, LeptosOptions};
#[cfg(feature = "ssr")]
use leptos_meta::MetaContext;
use linear_map::LinearMap;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use std::path::Path;
use std::{
    collections::HashMap,
    fmt::Display,
    future::Future,
    hash::{Hash, Hasher},
    path::PathBuf,
    pin::Pin,
    sync::Arc,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StaticParamsMap(pub LinearMap<String, Vec<String>>);

impl StaticParamsMap {
    /// Create a new empty `StaticParamsMap`.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a value into the map.
    #[inline]
    pub fn insert(&mut self, key: impl ToString, value: Vec<String>) {
        self.0.insert(key.to_string(), value);
    }

    /// Get a value from the map.
    #[inline]
    pub fn get(&self, key: &str) -> Option<&Vec<String>> {
        self.0.get(key)
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct StaticPath<'b, 'a: 'b> {
    path: &'a str,
    segments: Vec<StaticPathSegment<'a>>,
    params: LinearMap<&'a str, &'b Vec<String>>,
}

#[doc(hidden)]
#[derive(Debug)]
enum StaticPathSegment<'a> {
    Static(&'a str),
    Param(&'a str),
    Wildcard(&'a str),
}

impl<'b, 'a: 'b> StaticPath<'b, 'a> {
    pub fn new(path: &'a str) -> StaticPath<'b, 'a> {
        use StaticPathSegment::*;
        Self {
            path,
            segments: path
                .split('/')
                .filter(|s| !s.is_empty())
                .map(|s| match s.chars().next() {
                    Some(':') => Param(&s[1..]),
                    Some('*') => Wildcard(&s[1..]),
                    _ => Static(s),
                })
                .collect::<Vec<_>>(),
            params: LinearMap::new(),
        }
    }

    pub fn add_params(&mut self, params: &'b StaticParamsMap) {
        use StaticPathSegment::*;
        for segment in self.segments.iter() {
            match segment {
                Param(name) | Wildcard(name) => {
                    if let Some(value) = params.get(name) {
                        self.params.insert(name, value);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn into_paths(self) -> Vec<ResolvedStaticPath> {
        use StaticPathSegment::*;
        let mut paths = vec![ResolvedStaticPath(String::new())];

        for segment in self.segments {
            match segment {
                Static(s) => {
                    paths = paths
                        .into_iter()
                        .map(|p| ResolvedStaticPath(format!("{p}/{s}")))
                        .collect::<Vec<_>>();
                }
                Param(name) | Wildcard(name) => {
                    let mut new_paths = vec![];
                    for path in paths {
                        let Some(params) = self.params.get(name) else {
                            panic!(
                                "missing param {} for path: {}",
                                name, self.path
                            );
                        };
                        for val in params.iter() {
                            new_paths.push(ResolvedStaticPath(format!(
                                "{path}/{val}"
                            )));
                        }
                    }
                    paths = new_paths;
                }
            }
        }
        paths
    }

    pub fn parent(&self) -> Option<StaticPath<'b, 'a>> {
        if self.path == "/" || self.path.is_empty() {
            return None;
        }
        self.path
            .rfind('/')
            .map(|i| StaticPath::new(&self.path[..i]))
    }

    pub fn parents(&self) -> Vec<StaticPath<'b, 'a>> {
        let mut parents = vec![];
        let mut parent = self.parent();
        while let Some(p) = parent {
            parent = p.parent();
            parents.push(p);
        }
        parents
    }

    pub fn path(&self) -> &'a str {
        self.path
    }
}

impl Hash for StaticPath<'_, '_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

impl StaticPath<'_, '_> {}

#[doc(hidden)]
#[repr(transparent)]
pub struct ResolvedStaticPath(pub String);

impl Display for ResolvedStaticPath {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl ResolvedStaticPath {
    #[cfg(feature = "ssr")]
    pub async fn build<IV>(
        &self,
        options: &LeptosOptions,
        app_fn: impl Fn() -> IV + 'static + Clone,
        additional_context: impl Fn() + 'static + Clone,
    ) -> String
    where
        IV: IntoView + 'static,
    {
        let url = format!("http://leptos{self}");
        let app = {
            let app_fn = app_fn.clone();
            move || {
                provide_context(RouterIntegrationContext::new(
                    ServerIntegration { path: url },
                ));
                provide_context(MetaContext::new());
                (app_fn)().into_view()
            }
        };
        let (stream, runtime) = leptos::ssr::render_to_stream_in_order_with_prefix_undisposed_with_context(app, move || "".into(), additional_context.clone());
        leptos_integration_utils::build_async_response(stream, options, runtime)
            .await
    }

    #[cfg(feature = "ssr")]
    pub async fn write<IV>(
        &self,
        options: &LeptosOptions,
        app_fn: impl Fn() -> IV + 'static + Clone,
        additional_context: impl Fn() + 'static + Clone,
    ) -> Result<String, std::io::Error>
    where
        IV: IntoView + 'static,
    {
        let html = self.build(options, app_fn, additional_context).await;
        let file_path = static_file_path(options, &self.0);
        let path = Path::new(&file_path);
        if let Some(path) = path.parent() {
            std::fs::create_dir_all(path)?
        }
        std::fs::write(path, &html)?;
        Ok(html)
    }
}

#[cfg(feature = "ssr")]
pub async fn build_static_routes<IV>(
    options: &LeptosOptions,
    app_fn: impl Fn() -> IV + 'static + Clone,
    routes: &[RouteListing],
    static_data_map: &StaticDataMap,
) -> Result<(), std::io::Error>
where
    IV: IntoView + 'static,
{
    build_static_routes_with_additional_context(
        options,
        app_fn,
        || {},
        routes,
        static_data_map,
    )
    .await
}

#[cfg(feature = "ssr")]
pub async fn build_static_routes_with_additional_context<IV>(
    options: &LeptosOptions,
    app_fn: impl Fn() -> IV + 'static + Clone,
    additional_context: impl Fn() + 'static + Clone,
    routes: &[RouteListing],
    static_data_map: &StaticDataMap,
) -> Result<(), std::io::Error>
where
    IV: IntoView + 'static,
{
    let mut static_data: HashMap<&str, StaticParamsMap> = HashMap::new();
    let runtime = create_runtime();
    additional_context();
    for (key, value) in static_data_map {
        match value {
            Some(value) => static_data.insert(key, value.as_ref()().await),
            None => static_data.insert(key, StaticParamsMap::default()),
        };
    }
    runtime.dispose();
    let static_routes = routes
        .iter()
        .filter(|route| route.static_mode().is_some())
        .collect::<Vec<_>>();
    // TODO: maybe make this concurrent in some capacity
    for route in static_routes {
        let mut path = StaticPath::new(route.leptos_path());
        for p in path.parents().into_iter().rev() {
            if let Some(data) = static_data.get(p.path()) {
                path.add_params(data);
            }
        }
        if let Some(data) = static_data.get(path.path()) {
            path.add_params(data);
        }
        #[allow(clippy::print_stdout)]
        for path in path.into_paths() {
            println!("building static route: {path}");
            path.write(options, app_fn.clone(), additional_context.clone())
                .await?;
        }
    }
    Ok(())
}

pub type StaticData = Arc<StaticDataFn>;

pub type StaticDataFn = dyn Fn() -> Pin<Box<dyn Future<Output = StaticParamsMap> + Send + Sync>>
    + Send
    + Sync
    + 'static;

pub type StaticDataMap = HashMap<String, Option<StaticData>>;

/// The mode to use when rendering the route statically.
/// On mode `Upfront`, the route will be built with the server is started using the provided static
/// data. On mode `Incremental`, the route will be built on the first request to it and then cached
/// and returned statically for subsequent requests.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StaticMode {
    #[default]
    Upfront,
    Incremental,
}

#[doc(hidden)]
pub enum StaticStatusCode {
    Ok,
    NotFound,
    InternalServerError,
}

#[doc(hidden)]
pub enum StaticResponse {
    ReturnResponse {
        body: String,
        status: StaticStatusCode,
        content_type: Option<&'static str>,
    },
    RenderDynamic,
    RenderNotFound,
    WriteFile {
        body: String,
        path: PathBuf,
    },
}

#[doc(hidden)]
#[inline(always)]
#[cfg(feature = "ssr")]
pub fn static_file_path(options: &LeptosOptions, path: &str) -> String {
    let trimmed_path = path.trim_start_matches('/');
    let path = if trimmed_path.is_empty() {
        "index"
    } else {
        trimmed_path
    };
    format!("{}/{}.html", options.site_root, path)
}

#[doc(hidden)]
#[inline(always)]
#[cfg(feature = "ssr")]
pub fn not_found_path(options: &LeptosOptions) -> String {
    format!("{}{}.html", options.site_root, options.not_found_path)
}

#[doc(hidden)]
#[inline(always)]
pub fn upfront_static_route(
    res: Result<String, std::io::Error>,
) -> StaticResponse {
    match res {
        Ok(body) => StaticResponse::ReturnResponse {
            body,
            status: StaticStatusCode::Ok,
            content_type: Some("text/html"),
        },
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => StaticResponse::RenderNotFound,
            _ => {
                tracing::error!("error reading file: {}", e);
                StaticResponse::ReturnResponse {
                    body: "Internal Server Error".into(),
                    status: StaticStatusCode::InternalServerError,
                    content_type: None,
                }
            }
        },
    }
}

#[doc(hidden)]
#[inline(always)]
pub fn not_found_page(res: Result<String, std::io::Error>) -> StaticResponse {
    match res {
        Ok(body) => StaticResponse::ReturnResponse {
            body,
            status: StaticStatusCode::NotFound,
            content_type: Some("text/html"),
        },
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => StaticResponse::ReturnResponse {
                body: "Not Found".into(),
                status: StaticStatusCode::Ok,
                content_type: None,
            },
            _ => {
                tracing::error!("error reading not found file: {}", e);
                StaticResponse::ReturnResponse {
                    body: "Internal Server Error".into(),
                    status: StaticStatusCode::InternalServerError,
                    content_type: None,
                }
            }
        },
    }
}

#[doc(hidden)]
pub fn incremental_static_route(
    res: Result<String, std::io::Error>,
) -> StaticResponse {
    match res {
        Ok(body) => StaticResponse::ReturnResponse {
            body,
            status: StaticStatusCode::Ok,
            content_type: Some("text/html"),
        },
        Err(_) => StaticResponse::RenderDynamic,
    }
}

#[doc(hidden)]
#[cfg(feature = "ssr")]
pub async fn render_dynamic<IV>(
    path: &str,
    options: &LeptosOptions,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    additional_context: impl Fn() + 'static + Clone + Send,
) -> StaticResponse
where
    IV: IntoView + 'static,
{
    let body = ResolvedStaticPath(path.into())
        .build(options, app_fn, additional_context)
        .await;
    let path = Path::new(&static_file_path(options, path)).into();
    StaticResponse::WriteFile { body, path }
}
