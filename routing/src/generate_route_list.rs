use crate::{Method, SsrMode, StaticDataMap, StaticMode};
use routing_utils::PathSegment;
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
};
use tachys::{renderer::Renderer, view::RenderHtml};

#[derive(Debug, Default)]
/// A route that this application can serve.
pub struct RouteListing {
    path: Vec<PathSegment>,
    mode: SsrMode,
    methods: HashSet<Method>,
    static_mode: Option<(StaticMode, StaticDataMap)>,
}

impl RouteListing {
    /// Create a route listing from its parts.
    pub fn new(
        path: impl IntoIterator<Item = PathSegment>,
        mode: SsrMode,
        methods: impl IntoIterator<Item = Method>,
        static_mode: Option<(StaticMode, StaticDataMap)>,
    ) -> Self {
        Self {
            path: path.into_iter().collect(),
            mode,
            methods: methods.into_iter().collect(),
            static_mode,
        }
    }

    /// Create a route listing from a path, with the other fields set to default values.
    pub fn from_path(path: impl IntoIterator<Item = PathSegment>) -> Self {
        Self::new(path, SsrMode::Async, [], None)
    }

    /// The path this route handles.
    pub fn path(&self) -> &[PathSegment] {
        &self.path
    }

    /// The rendering mode for this path.
    pub fn mode(&self) -> SsrMode {
        self.mode
    }

    /// The HTTP request methods this path can handle.
    pub fn methods(&self) -> impl Iterator<Item = Method> + '_ {
        self.methods.iter().copied()
    }

    /// Whether this route is statically rendered.
    #[inline(always)]
    pub fn static_mode(&self) -> Option<StaticMode> {
        self.static_mode.as_ref().map(|n| n.0)
    }

    /// Whether this route is statically rendered.
    #[inline(always)]
    pub fn static_data_map(&self) -> Option<&StaticDataMap> {
        self.static_mode.as_ref().map(|n| &n.1)
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
        match self.static_mode {
            None => Ok(false),
            Some(_) => {
                let mut path = StaticPath::new(&self.leptos_path);
                path.add_params(params);
                for path in path.into_paths() {
                    path.write(
                        options,
                        app_fn.clone(),
                        additional_context.clone(),
                    )
                    .await?;
                }
                Ok(true)
            }
        }
    }*/
}

#[derive(Debug, Default)]
pub struct RouteList(Vec<RouteListing>);

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
}

impl RouteList {
    // this is used to indicate to the Router that we are generating
    // a RouteList for server path generation
    thread_local! {
        static IS_GENERATING: Cell<bool> = const { Cell::new(false) };
        static GENERATED: RefCell<Option<RouteList>> = const { RefCell::new(None) };
    }

    pub fn generate<T, Rndr>(app: impl FnOnce() -> T) -> Option<Self>
    where
        T: RenderHtml<Rndr>,
        Rndr: Renderer,
        Rndr::Node: Clone,
        Rndr::Element: Clone,
    {
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
