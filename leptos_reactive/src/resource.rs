#[cfg(feature = "experimental-islands")]
use crate::SharedContext;
#[cfg(debug_assertions)]
use crate::SpecialNonReactiveZone;
use crate::{
    create_isomorphic_effect, create_memo, create_render_effect, create_signal,
    queue_microtask, runtime::with_runtime, serialization::Serializable,
    signal_prelude::format_signal_warning, spawn::spawn_local,
    suspense::LocalStatus, use_context, GlobalSuspenseContext, Memo,
    ReadSignal, ScopeProperty, Signal, SignalDispose, SignalGet,
    SignalGetUntracked, SignalSet, SignalUpdate, SignalWith,
    SignalWithUntracked, SuspenseContext, WriteSignal,
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
/// # let runtime = create_runtime();
/// // any old async function; maybe this is calling a REST API or something
/// async fn fetch_cat_picture_urls(how_many: i32) -> Vec<String> {
///   // pretend we're fetching cat pics
///   vec![how_many.to_string()]
/// }
///
/// // a signal that controls how many cat pics we want
/// let (how_many_cats, set_how_many_cats) = create_signal(1);
///
/// // create a resource that will refetch whenever `how_many_cats` changes
/// # // `csr`, `hydrate`, and `ssr` all have issues here
/// # // because we're not running in a browser or in Tokio. Let's just ignore it.
/// # if false {
/// let cats = create_resource(move || how_many_cats.get(), fetch_cat_picture_urls);
///
/// // when we read the signal, it contains either
/// // 1) None (if the Future isn't ready yet) or
/// // 2) Some(T) (if the future's already resolved)
/// assert_eq!(cats.get(), Some(vec!["1".to_string()]));
///
/// // when the signal's value changes, the `Resource` will generate and run a new `Future`
/// set_how_many_cats.set(2);
/// assert_eq!(cats.get(), Some(vec!["2".to_string()]));
/// # }
/// # runtime.dispose();
/// ```
///
/// We can provide single, multiple or even a non-reactive signal as `source`
///
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if false {
/// # let how_many_cats = RwSignal::new(0); let how_many_dogs = RwSignal::new(0);
/// // Single signal. `Resource` will run once initially and then every time `how_many_cats` changes
/// let async_data = create_resource(move || how_many_cats.get() , |_| async move { todo!() });
/// // Non-reactive signal. `Resource` runs only once
/// let async_data = create_resource(|| (), |_| async move { todo!() });
/// // Multiple signals. `Resource` will run once initially and then every time `how_many_cats` or `how_many_dogs` changes
/// let async_data = create_resource(move || (how_many_cats.get(), how_many_dogs.get()), |_| async move { todo!() });
/// # runtime.dispose();
/// # }
/// ```
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            ty = %std::any::type_name::<T>(),
            signal_ty = %std::any::type_name::<S>(),
        )
    )
)]
pub fn create_resource<S, T, Fu>(
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

    create_resource_with_initial_value(source, fetcher, initial_value)
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
        level = "trace",
        skip_all,
        fields(
            ty = %std::any::type_name::<T>(),
            signal_ty = %std::any::type_name::<S>(),
        )
    )
)]
#[track_caller]
pub fn create_resource_with_initial_value<S, T, Fu>(
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
///
/// When used with the leptos_router and `SsrMode::PartiallyBlocked`, a
/// blocking resource will ensure `<Suspense/>` blocks depending on the resource
/// are fully rendered on the server side, without requiring JavaScript or
/// WebAssembly on the client.
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            ty = %std::any::type_name::<T>(),
            signal_ty = %std::any::type_name::<S>(),
        )
    )
)]
#[track_caller]
pub fn create_blocking_resource<S, T, Fu>(
    source: impl Fn() -> S + 'static,
    fetcher: impl Fn(S) -> Fu + 'static,
) -> Resource<S, T>
where
    S: PartialEq + Clone + 'static,
    T: Serializable + 'static,
    Fu: Future<Output = T> + 'static,
{
    create_resource_helper(
        source,
        fetcher,
        None,
        ResourceSerialization::Blocking,
    )
}

fn create_resource_helper<S, T, Fu>(
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
    let (value, set_value) = create_signal(initial_value);

    let (loading, set_loading) = create_signal(false);

    //crate::macros::debug_warn!("creating fetcher");
    let fetcher = Rc::new(move |s| {
        Box::pin(fetcher(s)) as Pin<Box<dyn Future<Output = T>>>
    });
    let source = create_memo(move |_| source());

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
        #[cfg(feature = "experimental-islands")]
        should_send_to_client: Default::default(),
    });

    let id = with_runtime(|runtime| {
        let r = Rc::clone(&r) as Rc<dyn SerializableResource>;
        let id = runtime.create_serializable_resource(r);
        runtime.push_scope_property(ScopeProperty::Resource(id));
        id
    })
    .expect("tried to create a Resource in a Runtime that has been disposed.");

    create_isomorphic_effect({
        let r = Rc::clone(&r);
        move |_| {
            source.track();
            load_resource(id, r.clone());
        }
    });

    Resource {
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
/// and therefore its result type does not need to be [`Serializable`].
///
/// Local resources do not load on the server, only in the client’s browser.
///
/// ## When to use a Local Resource
///
/// `create_resource` has three different features:
/// 1. gives a synchronous API for asynchronous things
/// 2. integrates with `Suspense`/`Transition``
/// 3. makes your application faster by starting things like DB access or an API request on the server,
///    rather than waiting until you've fully loaded the client
///
/// `create_local_resource` is useful when you can't or don't need to do #3 (serializing data from server
/// to client), but still want #1 (synchronous API for async) and #2 (integration with `Suspense`).
/// ```
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// #[derive(Debug, Clone)] // doesn't implement Serialize, Deserialize
/// struct ComplicatedUnserializableStruct {
///     // something here that can't be serialized
/// }
///
/// // an async function whose results can't be serialized from the server to the client
/// // (for example, opening a connection to the user's device camera)
/// async fn setup_complicated_struct() -> ComplicatedUnserializableStruct {
///     // do some work
///     ComplicatedUnserializableStruct {}
/// }
///
/// // create the resource; it will run but not be serialized
/// # // `csr`, `hydrate`, and `ssr` all have issues here
/// # // because we're not running in a browser or in Tokio. Let's just ignore it.
/// # if false {
/// let result =
///     create_local_resource(move || (), |_| setup_complicated_struct());
/// # }
/// # runtime.dispose();
/// ```
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            ty = %std::any::type_name::<T>(),
            signal_ty = %std::any::type_name::<S>(),
        )
    )
)]
pub fn create_local_resource<S, T, Fu>(
    source: impl Fn() -> S + 'static,
    fetcher: impl Fn(S) -> Fu + 'static,
) -> Resource<S, T>
where
    S: PartialEq + Clone + 'static,
    T: 'static,
    Fu: Future<Output = T> + 'static,
{
    let initial_value = None;
    create_local_resource_with_initial_value(source, fetcher, initial_value)
}

/// Creates a _local_ [`Resource`](crate::Resource) with the given initial value,
/// which will only generate and run a [`Future`] using the `fetcher` when the
/// `source` changes.
///
/// Unlike [`create_resource_with_initial_value()`], this [`Future`] will always run
/// on the local system and therefore its output type does not need to be
/// [`Serializable`].
///
/// Local resources do not load on the server, only in the client’s browser.
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            ty = %std::any::type_name::<T>(),
            signal_ty = %std::any::type_name::<S>(),
        )
    )
)]
pub fn create_local_resource_with_initial_value<S, T, Fu>(
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
    let (value, set_value) = create_signal(initial_value);

    let (loading, set_loading) = create_signal(false);

    let fetcher = Rc::new(move |s| {
        Box::pin(fetcher(s)) as Pin<Box<dyn Future<Output = T>>>
    });
    let source = create_memo(move |_| source());

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
        #[cfg(feature = "experimental-islands")]
        should_send_to_client: Default::default(),
    });

    let id = with_runtime(|runtime| {
        let r = Rc::clone(&r) as Rc<dyn UnserializableResource>;
        let id = runtime.create_unserializable_resource(r);
        runtime.push_scope_property(ScopeProperty::Resource(id));
        id
    })
    .expect("tried to create a Resource in a runtime that has been disposed.");

    // This is a local resource, so we're always going to handle it on the
    // client
    create_render_effect({
        let r = Rc::clone(&r);
        move |_| {
            source.track();
            r.load(false, id)
        }
    });

    Resource {
        id,
        source_ty: PhantomData,
        out_ty: PhantomData,
        #[cfg(any(debug_assertions, feature = "ssr"))]
        defined_at: std::panic::Location::caller(),
    }
}

#[cfg(not(feature = "hydrate"))]
fn load_resource<S, T>(id: ResourceId, r: Rc<ResourceState<S, T>>)
where
    S: PartialEq + Clone + 'static,
    T: 'static,
{
    SUPPRESS_RESOURCE_LOAD.with(|s| {
        if !s.get() {
            r.load(false, id)
        }
    });
}

#[cfg(feature = "hydrate")]
fn load_resource<S, T>(id: ResourceId, r: Rc<ResourceState<S, T>>)
where
    S: PartialEq + Clone + 'static,
    T: Serializable + 'static,
{
    use wasm_bindgen::{JsCast, UnwrapThrowExt};

    _ = with_runtime(|runtime| {
        let mut context = runtime.shared_context.borrow_mut();
        if let Some(data) = context.resolved_resources.remove(&id) {
            // The server already sent us the serialized resource value, so
            // deserialize & set it now
            context.pending_resources.remove(&id); // no longer pending
            r.resolved.set(true);

            let res = T::de(&data).unwrap_or_else(|e| {
                panic!(
                    "could not deserialize Resource<{}> JSON for {id:?}: {e:?}",
                    std::any::type_name::<T>()
                )
            });

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
                    let res = T::de(&res).unwrap_or_else(|e| {
                        panic!(
                            "could not deserialize Resource JSON for {id:?}: \
                             {e:?}"
                        )
                    });
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
            r.load(false, id);
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
    /// (`value.read()` is equivalent to `value.with(T::clone)`.)
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    #[track_caller]
    #[deprecated = "You can now use .get() on resources."]
    pub fn read(&self) -> Option<T>
    where
        T: Clone,
    {
        self.get()
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
        instrument(level = "trace", skip_all,)
    )]
    #[track_caller]
    pub fn map<U>(&self, f: impl FnOnce(&T) -> U) -> Option<U> {
        let location = std::panic::Location::caller();
        with_runtime(|runtime| {
            runtime.resource(self.id, |resource: &ResourceState<S, T>| {
                resource.with(f, location, self.id)
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
    pub fn loading(&self) -> Signal<bool> {
        #[allow(unused_variables)]
        let (loading, is_from_server) = with_runtime(|runtime| {
            let loading = runtime
                .resource(self.id, |resource: &ResourceState<S, T>| {
                    resource.loading
                });
            #[cfg(feature = "hydrate")]
            let is_from_server = runtime
                .shared_context
                .borrow()
                .server_resources
                .contains(&self.id);

            #[cfg(not(feature = "hydrate"))]
            let is_from_server = false;
            (loading, is_from_server)
        })
        .expect(
            "tried to call Resource::loading() in a runtime that has already \
             been disposed.",
        );

        #[cfg(feature = "hydrate")]
        {
            // if the loading signal is read outside Suspense
            // in hydrate mode, there will be a mismatch on first render
            // unless we delay a tick
            let (initial, set_initial) = create_signal(true);
            queue_microtask(move || set_initial.set(false));
            Signal::derive(move || {
                if is_from_server
                    && initial.get()
                    && use_context::<SuspenseContext>().is_none()
                {
                    true
                } else {
                    loading.get()
                }
            })
        }

        #[cfg(not(feature = "hydrate"))]
        {
            loading.into()
        }
    }

    /// Re-runs the async function with the current source data.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn refetch(&self) {
        _ = with_runtime(|runtime| {
            runtime.resource(self.id, |resource: &ResourceState<S, T>| {
                #[cfg(debug_assertions)]
                let prev = SpecialNonReactiveZone::enter();
                resource.refetch(self.id);
                #[cfg(debug_assertions)]
                {
                    SpecialNonReactiveZone::exit(prev);
                }
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
    pub async fn to_serialization_resolver(&self) -> (ResourceId, String)
    where
        T: Serializable,
    {
        with_runtime(|runtime| {
            runtime.resource(self.id, |resource: &ResourceState<S, T>| {
                resource.to_serialization_resolver(self.id)
            })
        })
        .expect(
            "tried to serialize a Resource in a runtime that has already been \
             disposed",
        )
        .await
    }
}

impl<S, T, E> Resource<S, Result<T, E>>
where
    E: Clone,
    S: Clone,
{
    /// Applies the given function when a resource that returns `Result<T, E>`
    /// has resolved and loaded an `Ok(_)`, rather than requiring nested `.map()`
    /// calls over the `Option<Result<_, _>>` returned by the resource.
    ///
    /// This is useful when used with features like server functions, in conjunction
    /// with `<ErrorBoundary/>` and `<Suspense/>`, when these other components are
    /// left to handle the `None` and `Err(_)` states.
    ///
    /// ```
    /// # use leptos_reactive::*;
    /// # if false {
    /// # // for miniserde support
    /// # #[cfg(not(any(feature="miniserde", feature="serde-lite")))] {
    /// let cats = create_resource(
    ///     || (),
    ///     |_| async { Ok(vec![0, 1, 2]) as Result<Vec<i32>, ()> },
    /// );
    /// create_effect(move |_| {
    ///     cats.and_then(|data: &Vec<i32>| println!("{}", data.len()));
    /// });
    /// # }
    /// # }
    /// ```
    #[track_caller]
    pub fn and_then<U>(&self, f: impl FnOnce(&T) -> U) -> Option<Result<U, E>> {
        self.map(|data| data.as_ref().map(f).map_err(|e| e.clone()))
    }
}

impl<S, T> SignalUpdate for Resource<S, T> {
    type Value = Option<T>;

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
        with_runtime(|runtime| {
            runtime.try_resource(self.id, |resource: &ResourceState<S, T>| {
                if resource.loading.get_untracked() {
                    resource.version.set(resource.version.get() + 1);
                    for suspense_context in
                        resource.suspense_contexts.borrow().iter()
                    {
                        suspense_context.decrement_for_resource(
                            resource.serializable
                                != ResourceSerialization::Local,
                            self.id,
                        );
                    }
                }
                resource.set_loading.set(false);
                resource.set_value.try_update(f)
            })
        })
        .ok()
        .flatten()
        .flatten()
    }
}

impl<S, T> SignalWith for Resource<S, T>
where
    S: Clone,
{
    type Value = Option<T>;

    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Resource::with()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    fn with<O>(&self, f: impl FnOnce(&Option<T>) -> O) -> O {
        let location = std::panic::Location::caller();
        match with_runtime(|runtime| {
            runtime.resource(self.id, |resource: &ResourceState<S, T>| {
                resource.with_maybe(f, location, self.id)
            })
        })
        .expect("runtime to be alive")
        {
            Some(o) => o,
            None => panic!(
                "{}",
                format_signal_warning(
                    "Attempted to read a resource after it was disposed.",
                    #[cfg(any(debug_assertions, feature = "ssr"))]
                    location,
                )
            ),
        }
    }

    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Resource::try_with()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    fn try_with<O>(&self, f: impl FnOnce(&Option<T>) -> O) -> Option<O> {
        let location = std::panic::Location::caller();
        with_runtime(|runtime| {
            runtime
                .try_resource(self.id, |resource: &ResourceState<S, T>| {
                    resource.with_maybe(f, location, self.id)
                })
                .flatten()
        })
        .ok()
        .flatten()
    }
}

impl<S, T> SignalGet for Resource<S, T>
where
    S: Clone,
    T: Clone,
{
    type Value = Option<T>;

    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Resource::get()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[inline(always)]
    fn get(&self) -> Option<T> {
        self.try_get().flatten()
    }

    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Resource::try_get()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[inline(always)]
    #[track_caller]
    fn try_get(&self) -> Option<Option<T>> {
        let location = std::panic::Location::caller();
        with_runtime(|runtime| {
            runtime.resource(self.id, |resource: &ResourceState<S, T>| {
                resource.read(location, self.id)
            })
        })
        .ok()
    }
}

impl<S, T> SignalSet for Resource<S, T> {
    type Value = T;

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
        let mut new_value = Some(new_value);
        self.try_update(|n| *n = new_value.take());
        new_value
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
/// # let runtime = create_runtime();
/// // any old async function; maybe this is calling a REST API or something
/// async fn fetch_cat_picture_urls(how_many: i32) -> Vec<String> {
///   // pretend we're fetching cat pics
///   vec![how_many.to_string()]
/// }
///
/// // a signal that controls how many cat pics we want
/// let (how_many_cats, set_how_many_cats) = create_signal(1);
///
/// // create a resource that will refetch whenever `how_many_cats` changes
/// # // `csr`, `hydrate`, and `ssr` all have issues here
/// # // because we're not running in a browser or in Tokio. Let's just ignore it.
/// # if false {
/// let cats = create_resource(move || how_many_cats.get(), fetch_cat_picture_urls);
///
/// // when we read the signal, it contains either
/// // 1) None (if the Future isn't ready yet) or
/// // 2) Some(T) (if the future's already resolved)
/// assert_eq!(cats.get(), Some(vec!["1".to_string()]));
///
/// // when the signal's value changes, the `Resource` will generate and run a new `Future`
/// set_how_many_cats.set(2);
/// assert_eq!(cats.get(), Some(vec!["2".to_string()]));
/// # }
/// # runtime.dispose();
/// ```
///
/// We can provide single, multiple or even a non-reactive signal as `source`
///
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if false {
/// # let how_many_cats = RwSignal::new(0); let how_many_dogs = RwSignal::new(0);
/// // Single signal. `Resource` will run once initially and then every time `how_many_cats` changes
/// let async_data = create_resource(move || how_many_cats.get() , |_| async move { todo!() });
/// // Non-reactive signal. `Resource` runs only once
/// let async_data = create_resource(|| (), |_| async move { todo!() });
/// // Multiple signals. `Resource` will run once initially and then every time `how_many_cats` or `how_many_dogs` changes
/// let async_data = create_resource(move || (how_many_cats.get(), how_many_dogs.get()), |_| async move { todo!() });
/// # runtime.dispose();
/// # }
/// ```
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Resource<S, T>
where
    S: 'static,
    T: 'static,
{
    pub(crate) id: ResourceId,
    pub(crate) source_ty: PhantomData<S>,
    pub(crate) out_ty: PhantomData<T>,
    #[cfg(any(debug_assertions, feature = "ssr"))]
    pub(crate) defined_at: &'static std::panic::Location<'static>,
}

impl<S, T> Resource<S, T>
where
    S: 'static,
    T: 'static,
{
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
    /// This is identical with [`create_resource`].
    ///
    /// ```
    /// # use leptos_reactive::*;
    /// # let runtime = create_runtime();
    /// // any old async function; maybe this is calling a REST API or something
    /// async fn fetch_cat_picture_urls(how_many: i32) -> Vec<String> {
    ///   // pretend we're fetching cat pics
    ///   vec![how_many.to_string()]
    /// }
    ///
    /// // a signal that controls how many cat pics we want
    /// let (how_many_cats, set_how_many_cats) = create_signal(1);
    ///
    /// // create a resource that will refetch whenever `how_many_cats` changes
    /// # // `csr`, `hydrate`, and `ssr` all have issues here
    /// # // because we're not running in a browser or in Tokio. Let's just ignore it.
    /// # if false {
    /// let cats = Resource::new(move || how_many_cats.get(), fetch_cat_picture_urls);
    ///
    /// // when we read the signal, it contains either
    /// // 1) None (if the Future isn't ready yet) or
    /// // 2) Some(T) (if the future's already resolved)
    /// assert_eq!(cats.get(), Some(vec!["1".to_string()]));
    ///
    /// // when the signal's value changes, the `Resource` will generate and run a new `Future`
    /// set_how_many_cats.set(2);
    /// assert_eq!(cats.get(), Some(vec!["2".to_string()]));
    /// # }
    /// # runtime.dispose();
    /// ```
    #[inline(always)]
    #[track_caller]
    pub fn new<Fu>(
        source: impl Fn() -> S + 'static,
        fetcher: impl Fn(S) -> Fu + 'static,
    ) -> Resource<S, T>
    where
        S: PartialEq + Clone + 'static,
        T: Serializable + 'static,
        Fu: Future<Output = T> + 'static,
    {
        create_resource(source, fetcher)
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
    /// This is identical with [`create_local_resource`].
    ///
    /// ```
    /// # use leptos_reactive::*;
    /// # let runtime = create_runtime();
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
    /// # // `csr`, `hydrate`, and `ssr` all have issues here
    /// # // because we're not running in a browser or in Tokio. Let's just ignore it.
    /// # if false {
    /// let result =
    ///     create_local_resource(move || (), |_| setup_complicated_struct());
    /// # }
    /// # runtime.dispose();
    /// ```
    #[inline(always)]
    #[track_caller]
    pub fn local<Fu>(
        source: impl Fn() -> S + 'static,
        fetcher: impl Fn(S) -> Fu + 'static,
    ) -> Resource<S, T>
    where
        S: PartialEq + Clone + 'static,
        T: 'static,
        Fu: Future<Output = T> + 'static,
    {
        let initial_value = None;
        create_local_resource_with_initial_value(source, fetcher, initial_value)
    }
}

impl<T> Resource<(), T>
where
    T: 'static,
{
    /// Creates a resource that will only load once, and will not respond
    /// to any reactive changes, including changes in any reactive variables
    /// read in its fetcher.
    ///
    /// This identical to `create_resource(|| (), move |_| fetcher())`.
    #[inline(always)]
    #[track_caller]
    pub fn once<Fu>(fetcher: impl Fn() -> Fu + 'static) -> Resource<(), T>
    where
        T: Serializable + 'static,
        Fu: Future<Output = T> + 'static,
    {
        create_resource(|| (), move |_| fetcher())
    }
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
    #[cfg(feature = "experimental-islands")]
    should_send_to_client: Rc<Cell<Option<bool>>>,
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
    #[track_caller]
    pub fn read(
        &self,
        location: &'static Location<'static>,
        id: ResourceId,
    ) -> Option<T>
    where
        T: Clone,
    {
        self.with(T::clone, location, id)
    }

    #[track_caller]
    pub fn with<U>(
        &self,
        f: impl FnOnce(&T) -> U,
        location: &'static Location<'static>,
        id: ResourceId,
    ) -> Option<U> {
        let global_suspense_cx = use_context::<GlobalSuspenseContext>();
        let suspense_cx = use_context::<SuspenseContext>();

        let v = self
            .value
            .try_with(|n| n.as_ref().map(|n| Some(f(n))))
            .ok()?
            .flatten();

        self.handle_result(
            location,
            global_suspense_cx,
            suspense_cx,
            v,
            false,
            id,
        )
    }

    #[track_caller]
    pub fn with_maybe<U>(
        &self,
        f: impl FnOnce(&Option<T>) -> U,
        location: &'static Location<'static>,
        id: ResourceId,
    ) -> Option<U> {
        let global_suspense_cx = use_context::<GlobalSuspenseContext>();
        let suspense_cx = use_context::<SuspenseContext>();
        let (was_loaded, v) =
            self.value.try_with(|n| (n.is_some(), f(n))).ok()?;

        self.handle_result(
            location,
            global_suspense_cx,
            suspense_cx,
            Some(v),
            !was_loaded,
            id,
        )
    }

    fn handle_result<U>(
        &self,
        location: &'static Location<'static>,
        global_suspense_cx: Option<GlobalSuspenseContext>,
        suspense_cx: Option<SuspenseContext>,
        v: Option<U>,
        force_suspend: bool,
        id: ResourceId,
    ) -> Option<U> {
        let suspense_contexts = self.suspense_contexts.clone();
        let has_value = v.is_some();

        let serializable = self.serializable;
        if let Some(suspense_cx) = &suspense_cx {
            if serializable != ResourceSerialization::Local {
                suspense_cx.local_status.update_value(|status| {
                    *status = Some(match status {
                        None => LocalStatus::SerializableOnly,
                        Some(LocalStatus::LocalOnly) => LocalStatus::LocalOnly,
                        Some(LocalStatus::Mixed) => LocalStatus::Mixed,
                        Some(LocalStatus::SerializableOnly) => {
                            LocalStatus::SerializableOnly
                        }
                    });
                });
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
            crate::on_cleanup({
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
                        if !has_value || force_suspend {
                            s.increment_for_resource(
                                serializable != ResourceSerialization::Local,
                                id,
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

                            if !has_value || force_suspend {
                                s.increment_for_resource(
                                    serializable
                                        != ResourceSerialization::Local,
                                    id,
                                );
                            }
                        }
                    })
                }
            }
        };

        create_isomorphic_effect(increment);
        v
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn refetch(&self, id: ResourceId) {
        self.load(true, id);
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    fn load(&self, refetching: bool, id: ResourceId) {
        // doesn't refetch if already refetching
        if refetching && self.scheduled.get() {
            return;
        }

        // if it's 1) in normal mode and is read, or
        // 2) is in island mode and read in an island, tell it to ship
        #[cfg(feature = "experimental-islands")]
        if self.should_send_to_client.get().is_none()
            && !SharedContext::no_hydrate()
        {
            self.should_send_to_client.set(Some(true));
        }

        let version = self.version.get() + 1;
        self.version.set(version);
        self.scheduled.set(false);

        _ = self.source.try_with_untracked(|source| {
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
                suspense_context.increment_for_resource(
                    self.serializable != ResourceSerialization::Local,
                    id,
                );
                if self.serializable == ResourceSerialization::Blocking {
                    suspense_context.should_block.set_value(true);
                }
            }

            let current_span = tracing::Span::current();
            // run the Future
            let serializable = self.serializable;
            spawn_local({
                let resolved = self.resolved.clone();
                let set_value = self.set_value;
                let set_loading = self.set_loading;
                let last_version = self.version.clone();
                async move {
                    // continue trace context within resource fetcher
                    let _guard = current_span.enter();
                    let res = fut.await;

                    if version == last_version.get() {
                        resolved.set(true);
                        set_value.try_update(|n| *n = Some(res));
                        set_loading.try_update(|n| *n = false);
                    }

                    for suspense_context in suspense_contexts.borrow().iter() {
                        suspense_context.decrement_for_resource(
                            serializable != ResourceSerialization::Local,
                            id,
                        );
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
        id: ResourceId,
    ) -> std::pin::Pin<Box<dyn futures::Future<Output = (ResourceId, String)>>>
    where
        T: Serializable,
    {
        use futures::StreamExt;

        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        let value = self.value;
        create_isomorphic_effect(move |_| {
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
        id: ResourceId,
    ) -> Pin<Box<dyn Future<Output = (ResourceId, String)>>>;

    fn should_send_to_client(&self) -> bool;
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
    #[inline(always)]
    fn to_serialization_resolver(
        &self,
        id: ResourceId,
    ) -> Pin<Box<dyn Future<Output = (ResourceId, String)>>> {
        let fut = self.resource_to_serialization_resolver(id);
        Box::pin(fut)
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    #[inline(always)]
    fn should_send_to_client(&self) -> bool {
        #[cfg(feature = "experimental-islands")]
        {
            self.should_send_to_client.get() == Some(true)
        }
        #[cfg(not(feature = "experimental-islands"))]
        {
            true
        }
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
    static SUPPRESS_RESOURCE_LOAD: Cell<bool> = const { Cell::new(false) };
}

#[doc(hidden)]
pub fn suppress_resource_load(suppress: bool) {
    SUPPRESS_RESOURCE_LOAD.with(|w| w.set(suppress));
}

#[doc(hidden)]
pub fn is_suppressing_resource_load() -> bool {
    SUPPRESS_RESOURCE_LOAD.with(|w| w.get())
}

impl<S, T> SignalDispose for Resource<S, T>
where
    S: 'static,
    T: 'static,
{
    #[track_caller]
    fn dispose(self) {
        let res = with_runtime(|runtime| {
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

#[cfg(feature = "nightly")]
impl<S: Clone, T: Clone> FnOnce<()> for Resource<S, T> {
    type Output = Option<T>;

    #[inline(always)]
    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(feature = "nightly")]
impl<S: Clone, T: Clone> FnMut<()> for Resource<S, T> {
    #[inline(always)]
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(feature = "nightly")]
impl<S: Clone, T: Clone> Fn<()> for Resource<S, T> {
    #[inline(always)]
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.get()
    }
}
