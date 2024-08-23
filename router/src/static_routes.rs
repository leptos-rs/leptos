use crate::PathSegment;
use futures::Stream;
use std::{
    fmt::{Debug, Display},
    future::Future,
    pin::Pin,
    sync::Arc,
};

type PinnedFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
type PinnedStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

pub type StaticParams = Arc<StaticParamsFn>;
pub type StaticParamsFn =
    dyn Fn() -> PinnedFuture<StaticParamsMap> + Send + Sync + 'static;

#[derive(Clone, Default)]
pub struct StaticRoute {
    pub(crate) prerender_params: Option<StaticParams>,
    pub(crate) invalidate:
        Option<Arc<dyn Fn() -> PinnedStream<()> + Send + Sync>>,
}

impl StaticRoute {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prerender_params<Fut>(
        mut self,
        params: impl Fn() -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        Fut: Future<Output = StaticParamsMap> + Send + 'static,
    {
        self.prerender_params = Some(Arc::new(move || Box::pin(params())));
        self
    }

    pub fn invalidate<St>(
        mut self,
        invalidate: impl Fn() -> St + Send + Sync + 'static,
    ) -> Self
    where
        St: Stream<Item = ()> + Send + 'static,
    {
        self.invalidate = Some(Arc::new(move || Box::pin(invalidate())));
        self
    }

    pub async fn to_prerendered_params(&self) -> Option<StaticParamsMap> {
        match &self.prerender_params {
            None => None,
            Some(params) => Some(params().await),
        }
    }
}

impl Debug for StaticRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticRoute").finish_non_exhaustive()
    }
}

impl PartialOrd for StaticRoute {
    fn partial_cmp(&self, _other: &Self) -> Option<std::cmp::Ordering> {
        Some(std::cmp::Ordering::Equal)
    }
}

impl Ord for StaticRoute {
    fn cmp(&self, _other: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}

impl PartialEq for StaticRoute {
    fn eq(&self, other: &Self) -> bool {
        let prerender = match (&self.prerender_params, &other.prerender_params)
        {
            (None, None) => true,
            (None, Some(_)) | (Some(_), None) => false,
            (Some(this), Some(that)) => Arc::ptr_eq(&this, &that),
        };
        let invalidate = match (&self.invalidate, &other.invalidate) {
            (None, None) => true,
            (None, Some(_)) | (Some(_), None) => false,
            (Some(this), Some(that)) => Arc::ptr_eq(&this, &that),
        };
        prerender && invalidate
    }
}

impl Eq for StaticRoute {}

#[derive(Debug, Clone, Default)]
pub struct StaticParamsMap(pub Vec<(String, Vec<String>)>);

impl StaticParamsMap {
    /// Create a new empty `StaticParamsMap`.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a value into the map.
    #[inline]
    pub fn insert(&mut self, key: impl ToString, value: Vec<String>) {
        let key = key.to_string();
        for item in self.0.iter_mut() {
            if item.0 == key {
                item.1 = value;
                return;
            }
        }
        self.0.push((key, value));
    }

    /// Get a value from the map.
    #[inline]
    pub fn get(&self, key: &str) -> Option<&Vec<String>> {
        self.0
            .iter()
            .find_map(|entry| (entry.0 == key).then_some(&entry.1))
    }
}

impl IntoIterator for StaticParamsMap {
    type Item = (String, Vec<String>);
    type IntoIter = StaticParamsIter;

    fn into_iter(self) -> Self::IntoIter {
        StaticParamsIter(self.0.into_iter())
    }
}

#[derive(Debug)]
pub struct StaticParamsIter(
    <Vec<(String, Vec<String>)> as IntoIterator>::IntoIter,
);

impl Iterator for StaticParamsIter {
    type Item = (String, Vec<String>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<A> FromIterator<A> for StaticParamsMap
where
    A: Into<(String, Vec<String>)>,
{
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        Self(iter.into_iter().map(Into::into).collect())
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct StaticPath {
    segments: Vec<PathSegment>,
}

impl StaticPath {
    pub fn new(segments: Vec<PathSegment>) -> StaticPath {
        Self { segments }
    }

    pub fn into_paths(
        self,
        params: Option<StaticParamsMap>,
    ) -> Vec<ResolvedStaticPath> {
        use PathSegment::*;
        let mut paths = vec![ResolvedStaticPath(String::new())];

        for segment in &self.segments {
            match segment {
                Unit => todo!(),
                Static(s) => {
                    paths = paths
                        .into_iter()
                        .map(|p| ResolvedStaticPath(format!("{p}/{s}")))
                        .collect::<Vec<_>>();
                }
                Param(name) | Splat(name) => {
                    let mut new_paths = vec![];
                    let params = params.as_ref().unwrap_or_else(|| {
                        panic!("missing params for path: {:?}", self.segments);
                    });
                    for path in paths {
                        let Some(params) = params.get(&name) else {
                            panic!(
                                "missing param {} for path: {:?}",
                                name, self.segments
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

    pub fn parent(&self) -> Option<StaticPath> {
        todo!()
        /*
        if self.path() == "/" || self.path().is_empty() {
            return None;
        }
        self.path()
            .rfind('/')
            .map(|i| StaticPath::new(&self.path()[..i]))
            */
    }

    pub fn parents(&self) -> Vec<StaticPath> {
        let mut parents = vec![];
        let mut parent = self.parent();
        while let Some(p) = parent {
            parent = p.parent();
            parents.push(p);
        }
        parents
    }
}

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct ResolvedStaticPath(pub String);

impl AsRef<str> for ResolvedStaticPath {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Display for ResolvedStaticPath {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

/*use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap, fmt::Debug, future::Future, ops::Deref, pin::Pin,
    sync::Arc,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StaticParamsMap(pub Vec<(String, Vec<String>)>);

impl StaticParamsMap {
    /// Create a new empty `StaticParamsMap`.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a value into the map.
    #[inline]
    pub fn insert(&mut self, key: impl ToString, value: Vec<String>) {
        let key = key.to_string();
        for item in self.0.iter_mut() {
            if item.0 == key {
                item.1 = value;
                return;
            }
        }
        self.0.push((key, value));
    }

    /// Get a value from the map.
    #[inline]
    pub fn get(&self, key: &str) -> Option<&Vec<String>> {
        self.0
            .iter()
            .find_map(|entry| (entry.0 == key).then_some(&entry.1))
    }
}

/// A function that returns the static data for a route.
#[derive(Clone)]
pub struct StaticData(Arc<StaticDataFn>);

impl Debug for StaticData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticData").finish_non_exhaustive()
    }
}

impl Deref for StaticData {
    type Target = StaticDataFn;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

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
#[derive(Debug)]
pub struct StaticPath<'b, 'a: 'b> {
    path: &'a str,
    segments: Vec<PathSegment>,
    params: LinearMap<&'a str, &'b Vec<String>>,
}

impl<'b, 'a: 'b> StaticPath<'b, 'a> {
    pub fn new(path: &'a str) -> StaticPath<'b, 'a> {
        use PathSegment::*;

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
*/
