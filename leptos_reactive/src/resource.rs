#![forbid(unsafe_code)]
use crate::{
    create_effect, create_isomorphic_effect, create_memo, create_signal,
    queue_microtask,
    runtime::{with_runtime, RuntimeId},
    serialization::Serializable,
    spawn::spawn_local,
    use_context, GlobalSuspenseContext, Memo, ReadSignal, Scope, ScopeProperty,
    SignalDispose, SignalGetUntracked, SignalSet, SignalUpdate, SignalWith,
    SuspenseContext, WriteSignal,
};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::HashSet,
    fmt::Debug,
    future::Future,
    marker::PhantomData,
    panic::Location,
    pin::Pin,
    rc::Rc,
};

/// Creates a [`Resource`](crate::Resource), which is a signal that reflects the
/// current state of an asynchronous task, allowing you to integrate `async`
/// [`Future`]s into the synchronous reactive system.
///
/// Takes a `fetcher` function that generates a [`Future`] when called and a
/// `source` signal that provides the argument for the `fetcher`. Whenever the
/// value of the `source` changes, a new [`Future`] will be created and run.
///
/// When server-side rendering is used, the server will handle running the
/// [`Future`] and will stream the result to the client. This process requires the
/// output type of the Future to be [`Serializable`]. If your output cannot be
/// serialized, or you just want to make sure the [`Future`] runs locally, use
/// [`create_local_resource()`].
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// // any old async function; maybe this is calling a REST API or something
/// async fn fetch_cat_picture_urls(how_many: i32) -> Vec<String> {
///   // pretend we're fetching cat pics
///   vec![how_many.to_string()]
/// }
///
/// // a signal that controls how many cat pics we want
/// let (how_many_cats, set_how_many_cats) = create_signal(cx, 1);
///
/// // create a resource that will refetch whenever `how_many_cats` changes
/// # // `csr`, `hydrate`, and `ssr` all have issues here
/// # // because we're not running in a browser or in Tokio. Let's just ignore it.
/// # if false {
/// let cats = create_resource(cx, move || how_many_cats.get(), fetch_cat_picture_urls);
///
/// // when we read the signal, it contains either
/// // 1) None (if the Future isn't ready yet) or
/// // 2) Some(T) (if the future's already resolved)
/// assert_eq!(cats.read(cx), Some(vec!["1".to_string()]));
///
/// // when the signal's value changes, the `Resource` will generate and run a new `Future`
/// set_how_many_cats.set(2);
/// assert_eq!(cats.read(cx), Some(vec!["2".to_string()]));
/// # }
/// # }).dispose();
/// ```
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "info",
        skip_all,
        fields(
            scope = ?cx.id,
            ty = %std::any::type_name::<T>(),
            signal_ty = %std::any::type_name::<S>(),
        )
    )
)]
pub fn create_resource<S, T, Fu>(
    cx: Scope,
    source: impl Fn() -> S + 'static,
    fetcher: impl Fn(S) -> Fu + 'static,
) -> Resource<S, T>
where
    S: PartialEq + Clone + 'static,
    T: Serializable + 'static,
    Fu: Future<Output = T> + 'static,
{
    // can't check this on the server without running the future
    let initial_value = None;

    create_resource_with_initial_value(cx, source, fetcher, initial_value)
}

/// Creates a [`Resource`](crate::Resource) with the given initial value, which
/// will only generate and run a [`Future`] using the `fetcher` when the `source` changes.
///
/// When server-side rendering is used, the server will handle running the
/// [`Future`] and will stream the result to the client. This process requires the
/// output type of the Future to be [`Serializable`]. If your output cannot be
/// serialized, or you just want to make sure the [`Future`] runs locally, use
/// [`create_local_resource_with_initial_value()`].
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "info",
        skip_all,
        fields(
            scope = ?cx.id,
            ty = %std::any::type_name::<T>(),
            signal_ty = %std::any::type_name::<S>(),
        )
    )
)]
#[track_caller]
pub fn create_resource_with_initial_value<S, T, Fu>(
    cx: Scope,
    source: impl Fn() -> S + 'static,
    fetcher: impl Fn(S) -> Fu + 'static,
    initial_value: Option<T>,
) -> Resource<S, T>
where
    S: PartialEq + Clone + 'static,
    T: Serializable + 'static,
    Fu: Future<Output = T> + 'static,
{
    create_resource_helper(
        cx,
        source,
        fetcher,
        initial_value,
        ResourceSerialization::Serializable,
    )
}

/// Creates a “blocking” [`Resource`](crate::Resource). When server-side rendering is used,
/// this resource will cause any `<Suspense/>` you read it under to block the initial
/// chunk of HTML from being sent to the client. This means that if you set things like
/// HTTP headers or `<head>` metadata in that `<Suspense/>`, that header material will
/// be included in the server’s original response.
///
/// This causes a slow time to first byte (TTFB) but is very useful for loading data that
/// is essential to the first load. For example, a blog post page that needs to include
/// the title of the blog post in the page’s initial HTML `<title>` tag for SEO reasons
/// might use a blocking resource to load blog post metadata, which will prevent the page from
/// returning until that data has loaded.
///
/// **Note**: This is not “blocking” in the sense that it blocks the current thread. Rather,
/// it is blocking in the sense that it blocks the server from sending a response.
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "info",
        skip_all,
        fields(
            scope = ?cx.id,
            ty = %std::any::type_name::<T>(),
            signal_ty = %std::any::type_name::<S>(),
        )
    )
)]
#[track_caller]
pub fn create_blocking_resource<S, T, Fu>(
    cx: Scope,
    source: impl Fn() -> S + 'static,
    fetcher: impl Fn(S) -> Fu + 'static,
) -> Resource<S, T>
where
    S: PartialEq + Clone + 'static,
    T: Serializable + 'static,
    Fu: Future<Output = T> + 'static,
{
    create_resource_helper(
        cx,
        source,
        fetcher,
        None,
        ResourceSerialization::Blocking,
    )
}

fn create_resource_helper<S, T, Fu>(
    cx: Scope,
    source: impl Fn() -> S + 'static,
    fetcher: impl Fn(S) -> Fu + 'static,
    initial_value: Option<T>,
    serializable: ResourceSerialization,
) -> Resource<S, T>
where
    S: PartialEq + Clone + 'static,
    T: Serializable + 'static,
    Fu: Future<Output = T> + 'static,
{
    let resolved = initial_value.is_some();
    let (value, set_value) = create_signal(cx, initial_value);

    let (loading, set_loading) = create_signal(cx, false);

    //crate::macros::debug_warn!("creating fetcher");
    let fetcher = Rc::new(move |s| {
        Box::pin(fetcher(s)) as Pin<Box<dyn Future<Output = T>>>
    });
    let source = create_memo(cx, move |_| source());

    let r = Rc::new(ResourceState {
        value,
        set_value,
        loading,
        set_loading,
        source,
        fetcher,
        resolved: Rc::new(Cell::new(resolved)),
        scheduled: Rc::new(Cell::new(false)),
        version: Rc::new(Cell::new(0)),
        suspense_contexts: Default::default(),
        serializable,
    });

    let id = with_runtime(cx.runtime, |runtime| {
        let r = Rc::clone(&r) as Rc<dyn SerializableResource>;
        runtime.create_serializable_resource(r)
    })
    .expect("tried to create a Resource in a Runtime that has been disposed.");

    //crate::macros::debug_warn!("creating effect");
    create_isomorphic_effect(cx, {
        let r = Rc::clone(&r);
        move |_| {
            load_resource(cx, id, r.clone());
        }
    });

    cx.push_scope_property(ScopeProperty::Resource(id));

    Resource {
        runtime: cx.runtime,
        id,
        source_ty: PhantomData,
        out_ty: PhantomData,
        #[cfg(any(debug_assertions, feature = "ssr"))]
        defined_at: std::panic::Location::caller(),
    }
}

/// Creates a _local_ [`Resource`](crate::Resource), which is a signal that
/// reflects the current state of an asynchronous task, allowing you to
/// integrate `async` [`Future`]s into the synchronous reactive system.
///
/// Takes a `fetcher` function that generates a [`Future`] when called and a
/// `source` signal that provides the argument for the `fetcher`. Whenever the
/// value of the `source` changes, a new [`Future`] will be created and run.
///
/// Unlike [`create_resource()`], this [`Future`] is always run on the local system
/// and therefore it's result type does not need to be [`Serializable`].
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// #[derive(Debug, Clone)] // doesn't implement Serialize, Deserialize
/// struct ComplicatedUnserializableStruct {
///     // something here that can't be serialized
/// }
/// // any old async function; maybe this is calling a REST API or something
/// async fn setup_complicated_struct() -> ComplicatedUnserializableStruct {
///     // do some work
///     ComplicatedUnserializableStruct {}
/// }
///
/// // create the resource; it will run but not be serialized
/// # if cfg!(not(any(feature = "csr", feature = "hydrate"))) {
/// let result =
///     create_local_resource(cx, move || (), |_| setup_complicated_struct());
/// # }
/// # }).dispose();
/// ```
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "info",
        skip_all,
        fields(
            scope = ?cx.id,
            ty = %std::any::type_name::<T>(),
            signal_ty = %std::any::type_name::<S>(),
        )
    )
)]
pub fn create_local_resource<S, T, Fu>(
    cx: Scope,
    source: impl Fn() -> S + 'static,
    fetcher: impl Fn(S) -> Fu + 'static,
) -> Resource<S, T>
where
    S: PartialEq + Clone + 'static,
    T: 'static,
    Fu: Future<Output = T> + 'static,
{
    let initial_value = None;
    create_local_resource_with_initial_value(cx, source, fetcher, initial_value)
}

/// Creates a _local_ [`Resource`](crate::Resource) with the given initial value,
/// which will only generate and run a [`Future`] using the `fetcher` when the
/// `source` changes.
///
/// Unlike [`create_resource_with_initial_value()`], this [`Future`] will always run
/// on the local system and therefore its output type does not need to be
/// [`Serializable`].
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "info",
        skip_all,
        fields(
            scope = ?cx.id,
            ty = %std::any::type_name::<T>(),
            signal_ty = %std::any::type_name::<S>(),
        )
    )
)]
pub fn create_local_resource_with_initial_value<S, T, Fu>(
    cx: Scope,
    source: impl Fn() -> S + 'static,
    fetcher: impl Fn(S) -> Fu + 'static,
    initial_value: Option<T>,
) -> Resource<S, T>
where
    S: PartialEq + Clone + 'static,
    T: 'static,
    Fu: Future<Output = T> + 'static,
{
    let resolved = initial_value.is_some();
    let (value, set_value) = create_signal(cx, initial_value);

    let (loading, set_loading) = create_signal(cx, false);

    let fetcher = Rc::new(move |s| {
        Box::pin(fetcher(s)) as Pin<Box<dyn Future<Output = T>>>
    });
    let source = create_memo(cx, move |_| source());

    let r = Rc::new(ResourceState {
        value,
        set_value,
        loading,
        set_loading,
        source,
        fetcher,
        resolved: Rc::new(Cell::new(resolved)),
        scheduled: Rc::new(Cell::new(false)),
        version: Rc::new(Cell::new(0)),
        suspense_contexts: Default::default(),
        serializable: ResourceSerialization::Local,
    });

    let id = with_runtime(cx.runtime, |runtime| {
        let r = Rc::clone(&r) as Rc<dyn UnserializableResource>;
        runtime.create_unserializable_resource(r)
    })
    .expect("tried to create a Resource in a runtime that has been disposed.");

    create_effect(cx, {
        let r = Rc::clone(&r);
        // This is a local resource, so we're always going to handle it on the
        // client
        move |_| r.load(false)
    });

    cx.push_scope_property(ScopeProperty::Resource(id));

    Resource {
        runtime: cx.runtime,
        id,
        source_ty: PhantomData,
        out_ty: PhantomData,
        #[cfg(any(debug_assertions, feature = "ssr"))]
        defined_at: std::panic::Location::caller(),
    }
}

#[cfg(not(feature = "hydrate"))]
fn load_resource<S, T>(_cx: Scope, _id: ResourceId, r: Rc<ResourceState<S, T>>)
where
    S: PartialEq + Clone + 'static,
    T: 'static,
{
    SUPPRESS_RESOURCE_LOAD.with(|s| {
        if !s.get() {
            r.load(false)
        }
    });
}

#[cfg(feature = "hydrate")]
fn load_resource<S, T>(cx: Scope, id: ResourceId, r: Rc<ResourceState<S, T>>)
where
    S: PartialEq + Clone + 'static,
    T: Serializable + 'static,
{
    use wasm_bindgen::{JsCast, UnwrapThrowExt};

    _ = with_runtime(cx.runtime, |runtime| {
        let mut context = runtime.shared_context.borrow_mut();
        if let Some(data) = context.resolved_resources.remove(&id) {
            // The server already sent us the serialized resource value, so
            // deserialize & set it now
            context.pending_resources.remove(&id); // no longer pending
            r.resolved.set(true);

            let res = T::de(&data)
                .expect_throw("could not deserialize Resource JSON");

            r.set_value.update(|n| *n = Some(res));
            r.set_loading.update(|n| *n = false);

            // for reactivity
            r.source.track();
        } else if context.pending_resources.remove(&id) {
            // We're still waiting for the resource, add a "resolver" closure so
            // that it will be set as soon as the server sends the serialized
            // value
            r.set_loading.update(|n| *n = true);

            let resolve = {
                let resolved = r.resolved.clone();
                let set_value = r.set_value;
                let set_loading = r.set_loading;
                move |res: String| {
                    let res = T::de(&res)
                        .expect_throw("could not deserialize Resource JSON");
                    resolved.set(true);
                    set_value.update(|n| *n = Some(res));
                    set_loading.update(|n| *n = false);
                }
            };
            let resolve = wasm_bindgen::closure::Closure::wrap(
                Box::new(resolve) as Box<dyn Fn(String)>,
            );
            let resource_resolvers = js_sys::Reflect::get(
                &web_sys::window().unwrap(),
                &wasm_bindgen::JsValue::from_str("__LEPTOS_RESOURCE_RESOLVERS"),
            )
            .expect_throw(
                "no __LEPTOS_RESOURCE_RESOLVERS found in the JS global scope",
            );
            let id = serde_json::to_string(&id)
                .expect_throw("could not serialize Resource ID");
            _ = js_sys::Reflect::set(
                &resource_resolvers,
                &wasm_bindgen::JsValue::from_str(&id),
                resolve.as_ref().unchecked_ref(),
            );

            // for reactivity
            r.source.track()
        } else {
            // Server didn't mark the resource as pending, so load it on the
            // client
            r.load(false);
        }
    })
}

impl<S, T> Resource<S, T>
where
    S: Clone + 'static,
    T: 'static,
{
    /// Clones and returns the current value of the resource ([Option::None] if the
    /// resource is still pending). Also subscribes the running effect to this
    /// resource.
    ///
    /// If you want to get the value without cloning it, use [`Resource::with`].
    /// (`value.read(cx)` is equivalent to `value.with(cx, T::clone)`.)
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", skip_all,)
    )]
    #[track_caller]
    pub fn read(&self, cx: Scope) -> Option<T>
    where
        T: Clone,
    {
        let location = std::panic::Location::caller();
        with_runtime(self.runtime, |runtime| {
            runtime.resource(self.id, |resource: &ResourceState<S, T>| {
                resource.read(cx, location)
            })
        })
        .ok()
        .flatten()
    }

    /// Applies a function to the current value of the resource, and subscribes
    /// the running effect to this resource. If the resource hasn't yet
    /// resolved, the function won't be called and this will return
    /// [`Option::None`].
    ///
    /// If you want to get the value by cloning it, you can use
    /// [`Resource::read`].
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", skip_all,)
    )]
    #[track_caller]
    pub fn with<U>(&self, cx: Scope, f: impl FnOnce(&T) -> U) -> Option<U> {
        let location = std::panic::Location::caller();
        with_runtime(self.runtime, |runtime| {
            runtime.resource(self.id, |resource: &ResourceState<S, T>| {
                resource.with(cx, f, location)
            })
        })
        .ok()
        .flatten()
    }

    /// Returns a signal that indicates whether the resource is currently loading.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn loading(&self) -> ReadSignal<bool> {
        with_runtime(self.runtime, |runtime| {
            runtime.resource(self.id, |resource: &ResourceState<S, T>| {
                resource.loading
            })
        })
        .expect(
            "tried to call Resource::loading() in a runtime that has already \
             been disposed.",
        )
    }

    /// Re-runs the async function with the current source data.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn refetch(&self) {
        _ = with_runtime(self.runtime, |runtime| {
            runtime.resource(self.id, |resource: &ResourceState<S, T>| {
                resource.refetch()
            })
        });
    }

    /// Returns a [`Future`] that will resolve when the resource has loaded,
    /// yield its [`ResourceId`] and a JSON string.
    #[cfg(any(feature = "ssr", doc))]
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub async fn to_serialization_resolver(
        &self,
        cx: Scope,
    ) -> (ResourceId, String)
    where
        T: Serializable,
    {
        with_runtime(self.runtime, |runtime| {
            runtime.resource(self.id, |resource: &ResourceState<S, T>| {
                resource.to_serialization_resolver(cx, self.id)
            })
        })
        .expect(
            "tried to serialize a Resource in a runtime that has already been \
             disposed",
        )
        .await
    }
}

impl<S, T> SignalUpdate<Option<T>> for Resource<S, T> {
    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Resource::update()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[inline(always)]
    fn update(&self, f: impl FnOnce(&mut Option<T>)) {
        self.try_update(f);
    }

    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Resource::try_update()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[inline(always)]
    fn try_update<O>(&self, f: impl FnOnce(&mut Option<T>) -> O) -> Option<O> {
        with_runtime(self.runtime, |runtime| {
            runtime.resource(self.id, |resource: &ResourceState<S, T>| {
                if resource.loading.get_untracked() {
                    resource.version.set(resource.version.get() + 1);
                    for suspense_context in
                        resource.suspense_contexts.borrow().iter()
                    {
                        suspense_context.decrement(
                            resource.serializable
                                != ResourceSerialization::Local,
                        );
                    }
                }
                resource.set_loading.set(false);
                resource.set_value.try_update(f)
            })
        })
        .ok()
        .flatten()
    }
}

impl<S, T> SignalSet<T> for Resource<S, T> {
    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Resource::set()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[inline(always)]
    fn set(&self, new_value: T) {
        self.try_set(new_value);
    }

    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Resource::try_set()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[inline(always)]
    fn try_set(&self, new_value: T) -> Option<T> {
        self.update(|n| *n = Some(new_value));
        None
    }
}

/// A signal that reflects the
/// current state of an asynchronous task, allowing you to integrate `async`
/// [`Future`]s into the synchronous reactive system.
///
/// Takes a `fetcher` function that generates a [`Future`] when called and a
/// `source` signal that provides the argument for the `fetcher`. Whenever the
/// value of the `source` changes, a new [`Future`] will be created and run.
///
/// When server-side rendering is used, the server will handle running the
/// [`Future`] and will stream the result to the client. This process requires the
/// output type of the Future to be [`Serializable`]. If your output cannot be
/// serialized, or you just want to make sure the [`Future`] runs locally, use
/// [`create_local_resource()`].
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// // any old async function; maybe this is calling a REST API or something
/// async fn fetch_cat_picture_urls(how_many: i32) -> Vec<String> {
///   // pretend we're fetching cat pics
///   vec![how_many.to_string()]
/// }
///
/// // a signal that controls how many cat pics we want
/// let (how_many_cats, set_how_many_cats) = create_signal(cx, 1);
///
/// // create a resource that will refetch whenever `how_many_cats` changes
/// # // `csr`, `hydrate`, and `ssr` all have issues here
/// # // because we're not running in a browser or in Tokio. Let's just ignore it.
/// # if false {
/// let cats = create_resource(cx, move || how_many_cats.get(), fetch_cat_picture_urls);
///
/// // when we read the signal, it contains either
/// // 1) None (if the Future isn't ready yet) or
/// // 2) Some(T) (if the future's already resolved)
/// assert_eq!(cats.read(cx), Some(vec!["1".to_string()]));
///
/// // when the signal's value changes, the `Resource` will generate and run a new `Future`
/// set_how_many_cats.set(2);
/// assert_eq!(cats.read(cx), Some(vec!["2".to_string()]));
/// # }
/// # }).dispose();
/// ```
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Resource<S, T>
where
    S: 'static,
    T: 'static,
{
    runtime: RuntimeId,
    pub(crate) id: ResourceId,
    pub(crate) source_ty: PhantomData<S>,
    pub(crate) out_ty: PhantomData<T>,
    #[cfg(any(debug_assertions, feature = "ssr"))]
    pub(crate) defined_at: &'static std::panic::Location<'static>,
}

// Resources
slotmap::new_key_type! {
    /// Unique ID assigned to a [`Resource`](crate::Resource).
    pub struct ResourceId;
}

impl<S, T> Clone for Resource<S, T>
where
    S: 'static,
    T: 'static,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, T> Copy for Resource<S, T>
where
    S: 'static,
    T: 'static,
{
}

#[derive(Clone)]
pub(crate) struct ResourceState<S, T>
where
    S: 'static,
    T: 'static,
{
    value: ReadSignal<Option<T>>,
    set_value: WriteSignal<Option<T>>,
    pub loading: ReadSignal<bool>,
    set_loading: WriteSignal<bool>,
    source: Memo<S>,
    #[allow(clippy::type_complexity)]
    fetcher: Rc<dyn Fn(S) -> Pin<Box<dyn Future<Output = T>>>>,
    resolved: Rc<Cell<bool>>,
    scheduled: Rc<Cell<bool>>,
    version: Rc<Cell<usize>>,
    suspense_contexts: Rc<RefCell<HashSet<SuspenseContext>>>,
    serializable: ResourceSerialization,
}

/// Whether and how the resource can be serialized.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum ResourceSerialization {
    /// Not serializable.
    Local,
    /// Can be serialized.
    Serializable,
    /// Can be serialized, and cause the first chunk to be blocked until
    /// their suspense has resolved.
    Blocking,
}

impl<S, T> ResourceState<S, T>
where
    S: Clone + 'static,
    T: 'static,
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", skip_all,)
    )]
    #[track_caller]
    pub fn read(
        &self,
        cx: Scope,
        location: &'static Location<'static>,
    ) -> Option<T>
    where
        T: Clone,
    {
        self.with(cx, T::clone, location)
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", skip_all,)
    )]
    #[track_caller]
    pub fn with<U>(
        &self,
        cx: Scope,
        f: impl FnOnce(&T) -> U,
        location: &'static Location<'static>,
    ) -> Option<U> {
        let global_suspense_cx = use_context::<GlobalSuspenseContext>(cx);
        let suspense_cx = use_context::<SuspenseContext>(cx);

        let v = self
            .value
            .try_with(|n| n.as_ref().map(|n| Some(f(n))))
            .ok()?
            .flatten();

        let suspense_contexts = self.suspense_contexts.clone();
        let has_value = v.is_some();

        let serializable = self.serializable;
        if let Some(suspense_cx) = &suspense_cx {
            if serializable != ResourceSerialization::Local {
                suspense_cx.has_local_only.set_value(false);
            }
        } else {
            #[cfg(not(all(feature = "hydrate", debug_assertions)))]
            {
                _ = location;
            }
            #[cfg(all(feature = "hydrate", debug_assertions))]
            {
                if self.serializable != ResourceSerialization::Local {
                    crate::macros::debug_warn!(
                        "At {location}, you are reading a resource in \
                         `hydrate` mode outside a <Suspense/> or \
                         <Transition/>. This can cause hydration mismatch \
                         errors and loses out on a significant performance \
                         optimization. To fix this issue, you can either: \
                         \n1. Wrap the place where you read the resource in a \
                         <Suspense/> or <Transition/> component, or \n2. \
                         Switch to using create_local_resource(), which will \
                         wait to load the resource until the app is hydrated \
                         on the client side. (This will have worse \
                         performance in most cases.)",
                    );
                }
            }
        }

        // on cleanup of this component, remove this read from parent `<Suspense/>`
        // it will be added back in when this is rendered again
        if let Some(s) = suspense_cx {
            crate::on_cleanup(cx, {
                let suspense_contexts = Rc::clone(&suspense_contexts);
                move || {
                    if let Ok(ref mut contexts) =
                        suspense_contexts.try_borrow_mut()
                    {
                        contexts.remove(&s);
                    }
                }
            });
        }

        let increment = move |_: Option<()>| {
            if let Some(s) = &suspense_cx {
                if let Ok(ref mut contexts) = suspense_contexts.try_borrow_mut()
                {
                    if !contexts.contains(s) {
                        contexts.insert(*s);

                        // on subsequent reads, increment will be triggered in load()
                        // because the context has been tracked here
                        // on the first read, resource is already loading without having incremented
                        if !has_value {
                            s.increment(
                                serializable != ResourceSerialization::Local,
                            );
                            if serializable == ResourceSerialization::Blocking {
                                s.should_block.set_value(true);
                            }
                        }
                    }
                }
            }

            if let Some(g) = &global_suspense_cx {
                if let Ok(ref mut contexts) = suspense_contexts.try_borrow_mut()
                {
                    g.with_inner(|s| {
                        if !contexts.contains(s) {
                            contexts.insert(*s);

                            if !has_value {
                                s.increment(
                                    serializable
                                        != ResourceSerialization::Local,
                                );
                            }
                        }
                    })
                }
            }
        };

        create_isomorphic_effect(cx, increment);
        v
    }
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn refetch(&self) {
        self.load(true);
    }
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    fn load(&self, refetching: bool) {
        // doesn't refetch if already refetching
        if refetching && self.scheduled.get() {
            return;
        }

        let version = self.version.get() + 1;
        self.version.set(version);
        self.scheduled.set(false);

        _ = self.source.try_with(|source| {
            let fut = (self.fetcher)(source.clone());

            // `scheduled` is true for the rest of this code only
            self.scheduled.set(true);
            queue_microtask({
                let scheduled = Rc::clone(&self.scheduled);
                move || {
                    scheduled.set(false);
                }
            });

            self.set_loading.update(|n| *n = true);

            // increment counter everywhere it's read
            let suspense_contexts = self.suspense_contexts.clone();

            for suspense_context in suspense_contexts.borrow().iter() {
                suspense_context.increment(
                    self.serializable != ResourceSerialization::Local,
                );
                if self.serializable == ResourceSerialization::Blocking {
                    suspense_context.should_block.set_value(true);
                }
            }

            // run the Future
            let serializable = self.serializable;
            spawn_local({
                let resolved = self.resolved.clone();
                let set_value = self.set_value;
                let set_loading = self.set_loading;
                let last_version = self.version.clone();
                async move {
                    let res = fut.await;

                    if version == last_version.get() {
                        resolved.set(true);
                        set_value.try_update(|n| *n = Some(res));
                        set_loading.try_update(|n| *n = false);

                        for suspense_context in
                            suspense_contexts.borrow().iter()
                        {
                            suspense_context.decrement(
                                serializable != ResourceSerialization::Local,
                            );
                        }
                    }
                }
            })
        });
    }
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn resource_to_serialization_resolver(
        &self,
        cx: Scope,
        id: ResourceId,
    ) -> std::pin::Pin<Box<dyn futures::Future<Output = (ResourceId, String)>>>
    where
        T: Serializable,
    {
        use futures::StreamExt;

        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        let value = self.value;
        create_isomorphic_effect(cx, move |_| {
            value.with({
                let mut tx = tx.clone();
                move |value| {
                    if let Some(value) = value.as_ref() {
                        tx.try_send((
                            id,
                            value.ser().expect("could not serialize Resource"),
                        ))
                        .expect(
                            "failed while trying to write to Resource \
                             serializer",
                        );
                    }
                }
            })
        });
        Box::pin(async move {
            rx.next()
                .await
                .expect("failed while trying to resolve Resource serializer")
        })
    }
}

#[derive(Clone)]
pub(crate) enum AnyResource {
    Unserializable(Rc<dyn UnserializableResource>),
    Serializable(Rc<dyn SerializableResource>),
}

pub(crate) trait SerializableResource {
    fn as_any(&self) -> &dyn Any;

    fn to_serialization_resolver(
        &self,
        cx: Scope,
        id: ResourceId,
    ) -> Pin<Box<dyn Future<Output = (ResourceId, String)>>>;
}

impl<S, T> SerializableResource for ResourceState<S, T>
where
    S: Clone,
    T: Serializable,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    fn to_serialization_resolver(
        &self,
        cx: Scope,
        id: ResourceId,
    ) -> Pin<Box<dyn Future<Output = (ResourceId, String)>>> {
        let fut = self.resource_to_serialization_resolver(cx, id);
        Box::pin(fut)
    }
}

pub(crate) trait UnserializableResource {
    fn as_any(&self) -> &dyn Any;
}

impl<S, T> UnserializableResource for ResourceState<S, T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

thread_local! {
    static SUPPRESS_RESOURCE_LOAD: Cell<bool> = Cell::new(false);
}

#[doc(hidden)]
pub fn suppress_resource_load(suppress: bool) {
    SUPPRESS_RESOURCE_LOAD.with(|w| w.set(suppress));
}

impl<S, T> SignalDispose for Resource<S, T>
where
    S: 'static,
    T: 'static,
{
    #[track_caller]
    fn dispose(self) {
        let res = with_runtime(self.runtime, |runtime| {
            let mut resources = runtime.resources.borrow_mut();
            resources.remove(self.id)
        });
        if res.ok().flatten().is_none() {
            crate::macros::debug_warn!(
                "At {}, you are calling Resource::dispose() on a resource \
                 that no longer exists, probably because its Scope has \
                 already been disposed.",
                std::panic::Location::caller()
            );
        }
    }
}
