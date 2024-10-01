use crate::{
    matching::PathSegment,
    static_routes::{
        RegenerationFn, ResolvedStaticPath, StaticPath, StaticRoute,
    },
    Method, SsrMode,
};
use futures::future::join_all;
use reactive_graph::owner::Owner;
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    future::Future,
    mem,
};
use tachys::view::RenderHtml;

#[derive(Clone, Debug, Default)]
/// A route that this application can serve.
pub struct RouteListing {
    path: Vec<PathSegment>,
    mode: SsrMode,
    methods: HashSet<Method>,
    regenerate: Vec<RegenerationFn>,
}

impl RouteListing {
    /// Create a route listing from its parts.
    pub fn new(
        path: impl IntoIterator<Item = PathSegment>,
        mode: SsrMode,
        methods: impl IntoIterator<Item = Method>,
        regenerate: impl IntoIterator<Item = RegenerationFn>,
    ) -> Self {
        Self {
            path: path.into_iter().collect(),
            mode,
            methods: methods.into_iter().collect(),
            regenerate: regenerate.into_iter().collect(),
        }
    }

    /// Create a route listing from a path, with the other fields set to default values.
    pub fn from_path(path: impl IntoIterator<Item = PathSegment>) -> Self {
        Self::new(path, SsrMode::Async, [], [])
    }

    /// The path this route handles.
    pub fn path(&self) -> &[PathSegment] {
        &self.path
    }

    /// The rendering mode for this path.
    pub fn mode(&self) -> &SsrMode {
        &self.mode
    }

    /// The HTTP request methods this path can handle.
    pub fn methods(&self) -> impl Iterator<Item = Method> + '_ {
        self.methods.iter().copied()
    }

    /// The set of regeneration functions that should be applied to this route, if it is statically
    /// generated (either up front or incrementally).
    pub fn regenerate(&self) -> &[RegenerationFn] {
        &self.regenerate
    }

    /// Whether this route is statically rendered.
    #[inline(always)]
    pub fn static_route(&self) -> Option<&StaticRoute> {
        match self.mode {
            SsrMode::Static(ref route) => Some(route),
            _ => None,
        }
    }

    pub async fn into_static_paths(self) -> Option<Vec<ResolvedStaticPath>> {
        let params = self.static_route()?.to_prerendered_params().await;
        Some(StaticPath::new(self.path).into_paths(params))
    }

    pub async fn generate_static_files<Fut, WriterFut>(
        mut self,
        render_fn: impl Fn(&ResolvedStaticPath) -> Fut + Send + Clone + 'static,
        writer: impl Fn(&ResolvedStaticPath, &Owner, String) -> WriterFut
            + Send
            + Clone
            + 'static,
        was_404: impl Fn(&Owner) -> bool + Send + Clone + 'static,
    ) where
        Fut: Future<Output = (Owner, String)> + Send + 'static,
        WriterFut: Future<Output = Result<(), std::io::Error>> + Send + 'static,
    {
        if let SsrMode::Static(_) = self.mode() {
            let (all_initial_tx, all_initial_rx) = std::sync::mpsc::channel();

            let render_fn = render_fn.clone();
            let regenerate = mem::take(&mut self.regenerate);
            let paths = self.into_static_paths().await.unwrap_or_default();

            for path in paths {
                // Err(_) here would just mean they've dropped the rx and are no longer awaiting
                // it; we're only using it to notify them it's done so it doesn't matter in that
                // case
                _ = all_initial_tx.send(path.build(
                    render_fn.clone(),
                    writer.clone(),
                    was_404.clone(),
                    regenerate.clone(),
                ));
            }

            join_all(all_initial_rx.try_iter()).await;
        }
    }

    /*
        /// Build a route statically, will return `Ok(true)` on success or `Ok(false)` when the route
        /// is not marked as statically rendered. All route parameters to use when resolving all paths
        /// to render should be passed in the `params` argument.
        pub async fn build_static<IV>(
            &self,
            options: &LeptosOptions,
            app_fn: impl Fn() -> IV + Send + 'static + Clone,
            additional_context: impl Fn() + Send + 'static + Clone,
            params: &StaticParamsMap,
        ) -> Result<bool, std::io::Error>
        where
            IV: IntoView + 'static,
        {
            match self.mode {
                SsrMode::Static(route) => {
                    let mut path = StaticPath::new(self.path.clone());
                    for path in path.into_paths(params) {
                        /*path.write(
                            options,
                            app_fn.clone(),
                            additional_context.clone(),
                        )
                        .await?;*/ println!()
                    }
                    Ok(true)
                }
                _ => Ok(false),
            }
        }
    */
}

#[derive(Debug, Default, Clone)]
pub struct RouteList(Vec<RouteListing>);

impl From<Vec<RouteListing>> for RouteList {
    fn from(value: Vec<RouteListing>) -> Self {
        Self(value)
    }
}

impl RouteList {
    pub fn push(&mut self, data: RouteListing) {
        self.0.push(data);
    }
}

impl RouteList {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn into_inner(self) -> Vec<RouteListing> {
        self.0
    }

    pub fn iter(&self) -> impl Iterator<Item = &RouteListing> {
        self.0.iter()
    }

    pub async fn into_static_paths(self) -> Vec<ResolvedStaticPath> {
        futures::future::join_all(
            self.into_inner()
                .into_iter()
                .map(|route_listing| route_listing.into_static_paths()),
        )
        .await
        .into_iter()
        .flatten()
        .flatten()
        .collect::<Vec<_>>()
    }

    pub async fn generate_static_files<Fut, WriterFut>(
        self,
        render_fn: impl Fn(&ResolvedStaticPath) -> Fut + Send + Clone + 'static,
        writer: impl Fn(&ResolvedStaticPath, &Owner, String) -> WriterFut
            + Send
            + Clone
            + 'static,
        was_404: impl Fn(&Owner) -> bool + Send + Clone + 'static,
    ) where
        Fut: Future<Output = (Owner, String)> + Send + 'static,
        WriterFut: Future<Output = Result<(), std::io::Error>> + Send + 'static,
    {
        join_all(self.into_inner().into_iter().map(|route| {
            route.generate_static_files(
                render_fn.clone(),
                writer.clone(),
                was_404.clone(),
            )
        }))
        .await;
    }
}

impl RouteList {
    // this is used to indicate to the Router that we are generating
    // a RouteList for server path generation
    thread_local! {
        static IS_GENERATING: Cell<bool> = const { Cell::new(false) };
        static GENERATED: RefCell<Option<RouteList>> = const { RefCell::new(None) };
    }

    pub fn generate<T>(app: impl FnOnce() -> T) -> Option<Self>
    where
        T: RenderHtml,
    {
        let _resource_guard = leptos::server::SuppressResourceLoad::new();
        Self::IS_GENERATING.set(true);
        // run the app once, but throw away the HTML
        // the router won't actually route, but will fill the listing
        _ = app().to_html();
        Self::IS_GENERATING.set(false);
        Self::GENERATED.take()
    }

    pub fn is_generating() -> bool {
        Self::IS_GENERATING.get()
    }

    pub fn register(routes: RouteList) {
        Self::GENERATED.with(|inner| {
            *inner.borrow_mut() = Some(routes);
        });
    }
}
