use crate::{hooks::RawParamsMap, params::ParamsMap, PathSegment};
use futures::{channel::oneshot, stream, Stream, StreamExt};
use leptos::task::spawn;
use reactive_graph::{owner::Owner, traits::GetUntracked};
use std::{
    fmt::{Debug, Display},
    future::Future,
    ops::Deref,
    pin::Pin,
    sync::Arc,
};

type PinnedFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
type PinnedStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

pub type StaticParams = Arc<StaticParamsFn>;
pub type StaticParamsFn =
    dyn Fn() -> PinnedFuture<StaticParamsMap> + Send + Sync + 'static;

#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct RegenerationFn(
    Arc<dyn Fn(&ParamsMap) -> PinnedStream<()> + Send + Sync>,
);

impl Debug for RegenerationFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegenerationFn").finish_non_exhaustive()
    }
}

impl Deref for RegenerationFn {
    type Target = dyn Fn(&ParamsMap) -> PinnedStream<()> + Send + Sync;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl PartialEq for RegenerationFn {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Default)]
pub struct StaticRoute {
    pub(crate) prerender_params: Option<StaticParams>,
    pub(crate) regenerate: Option<RegenerationFn>,
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

    pub fn regenerate<St>(
        mut self,
        invalidate: impl Fn(&ParamsMap) -> St + Send + Sync + 'static,
    ) -> Self
    where
        St: Stream<Item = ()> + Send + 'static,
    {
        self.regenerate = Some(RegenerationFn(Arc::new(move |params| {
            Box::pin(invalidate(params))
        })));
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
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
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
            (Some(this), Some(that)) => Arc::ptr_eq(this, that),
        };
        prerender && (self.regenerate == other.regenerate)
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
        let mut paths = vec![ResolvedStaticPath {
            path: String::new(),
        }];

        for segment in &self.segments {
            match segment {
                Unit => {}
                Static(s) => {
                    paths = paths
                        .into_iter()
                        .map(|p| {
                            if s.starts_with("/") {
                                ResolvedStaticPath {
                                    path: format!("{}{s}", p.path),
                                }
                            } else {
                                ResolvedStaticPath {
                                    path: format!("{}/{s}", p.path),
                                }
                            }
                        })
                        .collect::<Vec<_>>();
                }
                Param(name) | Splat(name) => {
                    let mut new_paths = vec![];
                    if let Some(params) = params.as_ref() {
                        for path in paths {
                            if let Some(params) = params.get(name) {
                                for val in params.iter() {
                                    new_paths.push(if val.starts_with("/") {
                                        ResolvedStaticPath {
                                            path: format!(
                                                "{}{}",
                                                path.path, val
                                            ),
                                        }
                                    } else {
                                        ResolvedStaticPath {
                                            path: format!(
                                                "{}/{}",
                                                path.path, val
                                            ),
                                        }
                                    });
                                }
                            }
                        }
                    }
                    paths = new_paths;
                }
            }
        }
        paths
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedStaticPath {
    pub(crate) path: String,
}

impl ResolvedStaticPath {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl AsRef<str> for ResolvedStaticPath {
    fn as_ref(&self) -> &str {
        self.path.as_ref()
    }
}

impl Display for ResolvedStaticPath {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&self.path, f)
    }
}

impl ResolvedStaticPath {
    pub async fn build<Fut, WriterFut>(
        self,
        render_fn: impl Fn(&ResolvedStaticPath) -> Fut + Send + Clone + 'static,
        writer: impl Fn(&ResolvedStaticPath, &Owner, String) -> WriterFut
            + Send
            + Clone
            + 'static,
        was_404: impl Fn(&Owner) -> bool + Send + Clone + 'static,
        regenerate: Vec<RegenerationFn>,
    ) -> (Owner, Option<String>)
    where
        Fut: Future<Output = (Owner, String)> + Send + 'static,
        WriterFut: Future<Output = Result<(), std::io::Error>> + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();

        // spawns a separate task for each path it's rendering
        // this allows us to parallelize all static site rendering,
        // and also to create long-lived tasks
        spawn({
            let render_fn = render_fn.clone();
            let writer = writer.clone();
            let was_error = was_404.clone();
            async move {
                // render and write the initial page
                let (owner, html) = render_fn(&self).await;

                // if rendering this page resulted in an error (404, 500, etc.)
                // then we should not cache it: the `was_error` function can handle notifying
                // the user that there was an error, and the server can give a dynamic response
                // that will include the 404 or 500
                if was_error(&owner) {
                    // can ignore errors from channel here, because it just means we're not
                    // awaiting the Future
                    _ = tx.send((owner.clone(), Some(html)));
                } else {
                    if let Err(e) = writer(&self, &owner, html).await {
                        #[cfg(feature = "tracing")]
                        tracing::warn!("{e}");

                        #[cfg(not(feature = "tracing"))]
                        eprintln!("{e}");
                    }
                    _ = tx.send((owner.clone(), None));
                }

                // if there's a regeneration function, keep looping
                let params = if regenerate.is_empty() {
                    None
                } else {
                    Some(
                        owner
                            .use_context_bidirectional::<RawParamsMap>()
                            .expect(
                                "using static routing, but couldn't find \
                                 ParamsMap",
                            )
                            .get_untracked(),
                    )
                };
                let mut regenerate = stream::select_all(
                    regenerate
                        .into_iter()
                        .map(|r| owner.with(|| r(params.as_ref().unwrap()))),
                );
                while regenerate.next().await.is_some() {
                    let (owner, html) = render_fn(&self).await;
                    if !was_error(&owner) {
                        if let Err(e) = writer(&self, &owner, html).await {
                            #[cfg(feature = "tracing")]
                            tracing::warn!("{e}");

                            #[cfg(not(feature = "tracing"))]
                            eprintln!("{e}");
                        }
                    }
                    drop(owner);
                }
            }
        });

        rx.await.unwrap()
    }
}
