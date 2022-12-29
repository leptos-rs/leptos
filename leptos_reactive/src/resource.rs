use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::HashSet,
    fmt::Debug,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    rc::Rc,
};
use crate::{
    create_effect, create_isomorphic_effect, create_memo, create_signal, queue_microtask,
    runtime::{with_runtime, RuntimeId},
    serialization::Serializable,
    spawn::spawn_local,
    use_context, Memo, ReadSignal, Scope, ScopeProperty, SuspenseContext, WriteSignal,
};

/// Creates [Resource](crate::Resource), which is a signal that reflects the
/// current state of an asynchronous task, allowing you to integrate `async`
/// [Future]s into the synchronous reactive system.
///
/// Takes a `fetcher` function that generates a [Future] when called and a
/// `source` signal that provides the argument for the `fetcher`. Whenever the
/// value of the `source` changes, a new [Future] will be created and run.
///
/// When server-side rendering is used, the server will handle running the
/// [Future] and will stream the result to the client. This process requires the
/// output type of the Future to be [Serializable]. If your output cannot be
/// serialized, or you just want to make sure the [Future] runs locally, use
/// [create_local_resource()].
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
/// let cats = create_resource(cx, how_many_cats, fetch_cat_picture_urls);
///
/// // when we read the signal, it contains either
/// // 1) None (if the Future isn't ready yet) or
/// // 2) Some(T) (if the future's already resolved)
/// assert_eq!(cats(), Some(vec!["1".to_string()]));
///
/// // when the signal's value changes, the `Resource` will generate and run a new `Future`
/// set_how_many_cats(2);
/// assert_eq!(cats(), Some(vec!["2".to_string()]));
/// # }
/// # }).dispose();
/// ```
pub fn create_resource<S, T, Fu>(
    cx: Scope,
    source: impl Fn() -> S + 'static,
    fetcher: impl Fn(S) -> Fu + 'static,
) -> Resource<S, T>
where
    S: PartialEq + Debug + Clone + 'static,
    T: Serializable + 'static,
    Fu: Future<Output = T> + 'static,
{
    // can't check this on the server without running the future
    let initial_value = None;

    create_resource_with_initial_value(cx, source, fetcher, initial_value)
}

/// Creates a [Resource](crate::Resource) with the given initial value, which
/// will only generate and run a [Future] using the `fetcher` when the `source` changes.
///
/// When server-side rendering is used, the server will handle running the
/// [Future] and will stream the result to the client. This process requires the
/// output type of the Future to be [Serializable]. If your output cannot be
/// serialized, or you just want to make sure the [Future] runs locally, use
/// [create_local_resource_with_initial_value()].
pub fn create_resource_with_initial_value<S, T, Fu>(
    cx: Scope,
    source: impl Fn() -> S + 'static,
    fetcher: impl Fn(S) -> Fu + 'static,
    initial_value: Option<T>,
) -> Resource<S, T>
where
    S: PartialEq + Debug + Clone + 'static,
    T: Serializable + 'static,
    Fu: Future<Output = T> + 'static,
{
    let resolved = initial_value.is_some();
    let (value, set_value) = create_signal(cx, initial_value);

    let (loading, set_loading) = create_signal(cx, false);

    let fetcher = Rc::new(move |s| Box::pin(fetcher(s)) as Pin<Box<dyn Future<Output = T>>>);
    let source = create_memo(cx, move |_| source());

    let r = Rc::new(ResourceState {
        scope: cx,
        value,
        set_value,
        loading,
        set_loading,
        source,
        fetcher,
        resolved: Rc::new(Cell::new(resolved)),
        scheduled: Rc::new(Cell::new(false)),
        suspense_contexts: Default::default(),
    });

    let id = with_runtime(cx.runtime, |runtime| {
        runtime.create_serializable_resource(Rc::clone(&r))
    });

    create_isomorphic_effect(cx, {
        let r = Rc::clone(&r);
        move |_| {
            load_resource(cx, id, r.clone());
        }
    });

    cx.with_scope_property(|prop| prop.push(ScopeProperty::Resource(id)));

    Resource {
        runtime: cx.runtime,
        id,
        source_ty: PhantomData,
        out_ty: PhantomData,
    }
}

/// Creates a _local_ [Resource](crate::Resource), which is a signal that
/// reflects the current state of an asynchronous task, allowing you to
/// integrate `async` [Future]s into the synchronous reactive system.
///
/// Takes a `fetcher` function that generates a [Future] when called and a
/// `source` signal that provides the argument for the `fetcher`. Whenever the
/// value of the `source` changes, a new [Future] will be created and run.
///
/// Unlike [create_resource()], this [Future] is always run on the local system
/// and therefore it's result type does not need to be [Serializable].
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// #[derive(Debug, Clone)] // doesn't implement Serialize, Deserialize
/// struct ComplicatedUnserializableStruct {
///   // something here that can't be serialized
/// }
/// // any old async function; maybe this is calling a REST API or something
/// async fn setup_complicated_struct() -> ComplicatedUnserializableStruct {
///   // do some work
///   ComplicatedUnserializableStruct { }
/// }
///
/// // create the resource; it will run but not be serialized
/// # if cfg!(not(any(feature = "csr", feature = "hydrate"))) {
/// let result = create_local_resource(cx, move || (), |_| setup_complicated_struct());
/// # }
/// # }).dispose();
/// ```
pub fn create_local_resource<S, T, Fu>(
    cx: Scope,
    source: impl Fn() -> S + 'static,
    fetcher: impl Fn(S) -> Fu + 'static,
) -> Resource<S, T>
where
    S: PartialEq + Debug + Clone + 'static,
    T: 'static,
    Fu: Future<Output = T> + 'static,
{
    let initial_value = None;
    create_local_resource_with_initial_value(cx, source, fetcher, initial_value)
}

/// Creates a _local_ [Resource](crate::Resource) with the given initial value,
/// which will only generate and run a [Future] using the `fetcher` when the
/// `source` changes.
///
/// Unlike [create_resource_with_initial_value()], this [Future] will always run
/// on the local system and therefore its output type does not need to be
/// [Serializable].
pub fn create_local_resource_with_initial_value<S, T, Fu>(
    cx: Scope,
    source: impl Fn() -> S + 'static,
    fetcher: impl Fn(S) -> Fu + 'static,
    initial_value: Option<T>,
) -> Resource<S, T>
where
    S: PartialEq + Debug + Clone + 'static,
    T: 'static,
    Fu: Future<Output = T> + 'static,
{
    let resolved = initial_value.is_some();
    let (value, set_value) = create_signal(cx, initial_value);

    let (loading, set_loading) = create_signal(cx, false);

    let fetcher = Rc::new(move |s| Box::pin(fetcher(s)) as Pin<Box<dyn Future<Output = T>>>);
    let source = create_memo(cx, move |_| source());

    let r = Rc::new(ResourceState {
        scope: cx,
        value,
        set_value,
        loading,
        set_loading,
        source,
        fetcher,
        resolved: Rc::new(Cell::new(resolved)),
        scheduled: Rc::new(Cell::new(false)),
        suspense_contexts: Default::default(),
    });

    let id = with_runtime(cx.runtime, |runtime| {
        runtime.create_unserializable_resource(Rc::clone(&r))
    });

    create_effect(cx, {
        let r = Rc::clone(&r);
        // This is a local resource, so we're always going to handle it on the
        // client
        move |_| r.load(false)
    });

    cx.with_scope_property(|prop| prop.push(ScopeProperty::Resource(id)));

    Resource {
        runtime: cx.runtime,
        id,
        source_ty: PhantomData,
        out_ty: PhantomData,
    }
}

#[cfg(not(feature = "hydrate"))]
fn load_resource<S, T>(_cx: Scope, _id: ResourceId, r: Rc<ResourceState<S, T>>)
where
    S: PartialEq + Debug + Clone + 'static,
    T: 'static,
{
    r.load(false)
}

#[cfg(feature = "hydrate")]
fn load_resource<S, T>(cx: Scope, id: ResourceId, r: Rc<ResourceState<S, T>>)
where
    S: PartialEq + Debug + Clone + 'static,
    T: Serializable + 'static,
{
    use wasm_bindgen::{JsCast, UnwrapThrowExt};

    with_runtime(cx.runtime, |runtime| {
        let mut context = runtime.shared_context.borrow_mut();
        if let Some(data) = context.resolved_resources.remove(&id) {
            // The server already sent us the serialized resource value, so
            // deserialize & set it now
            context.pending_resources.remove(&id); // no longer pending
            r.resolved.set(true);

            let res = T::from_json(&data).expect_throw("could not deserialize Resource JSON");

            r.set_value.update(|n| *n = Some(res));
            r.set_loading.update(|n| *n = false);

            // for reactivity
            r.source.subscribe();
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
                    let res =
                        T::from_json(&res).expect_throw("could not deserialize Resource JSON");
                    resolved.set(true);
                    set_value.update(|n| *n = Some(res));
                    set_loading.update(|n| *n = false);
                }
            };
            let resolve =
                wasm_bindgen::closure::Closure::wrap(Box::new(resolve) as Box<dyn Fn(String)>);
            let resource_resolvers = js_sys::Reflect::get(
                &web_sys::window().unwrap(),
                &wasm_bindgen::JsValue::from_str("__LEPTOS_RESOURCE_RESOLVERS"),
            )
            .expect_throw("no __LEPTOS_RESOURCE_RESOLVERS found in the JS global scope");
            let id = serde_json::to_string(&id).expect_throw("could not serialize Resource ID");
            _ = js_sys::Reflect::set(
                &resource_resolvers,
                &wasm_bindgen::JsValue::from_str(&id),
                resolve.as_ref().unchecked_ref(),
            );

            // for reactivity
            r.source.subscribe()
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
    /// If you want to get the value without cloning it, use [Resource::with].
    /// (`value.read()` is equivalent to `value.with(T::clone)`.)
    pub fn read(&self) -> Option<T>
    where
        T: Clone,
    {
        with_runtime(self.runtime, |runtime| {
            runtime.resource(self.id, |resource: &ResourceState<S, T>| resource.read())
        })
    }

    /// Applies a function to the current value of the resource, and subscribes
    /// the running effect to this resource. If the resource hasn't yet
    /// resolved, the function won't be called and this will return
    /// [Option::None].
    ///
    /// If you want to get the value by cloning it, you can use
    /// [Resource::read].
    pub fn with<U>(&self, f: impl FnOnce(&T) -> U) -> Option<U> {
        with_runtime(self.runtime, |runtime| {
            runtime.resource(self.id, |resource: &ResourceState<S, T>| resource.with(f))
        })
    }

    /// Returns a signal that indicates whether the resource is currently loading.
    pub fn loading(&self) -> ReadSignal<bool> {
        with_runtime(self.runtime, |runtime| {
            runtime.resource(self.id, |resource: &ResourceState<S, T>| resource.loading)
        })
    }

    /// Re-runs the async function with the current source data.
    pub fn refetch(&self) {
        with_runtime(self.runtime, |runtime| {
            runtime.resource(self.id, |resource: &ResourceState<S, T>| resource.refetch())
        });
    }

    /// Returns a [std::future::Future] that will resolve when the resource has loaded,
    /// yield its [ResourceId] and a JSON string.
    #[cfg(any(feature = "ssr", doc))]
    pub async fn to_serialization_resolver(&self) -> (ResourceId, String)
    where
        T: Serializable,
    {
        with_runtime(self.runtime, |runtime| {
            runtime.resource(self.id, |resource: &ResourceState<S, T>| {
                resource.to_serialization_resolver(self.id)
            })
        })
        .await
    }
}

/// A signal that reflects the
/// current state of an asynchronous task, allowing you to integrate `async`
/// [Future]s into the synchronous reactive system.
///
/// Takes a `fetcher` function that generates a [Future] when called and a
/// `source` signal that provides the argument for the `fetcher`. Whenever the
/// value of the `source` changes, a new [Future] will be created and run.
///
/// When server-side rendering is used, the server will handle running the
/// [Future] and will stream the result to the client. This process requires the
/// output type of the Future to be [Serializable]. If your output cannot be
/// serialized, or you just want to make sure the [Future] runs locally, use
/// [create_local_resource()].
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
/// let cats = create_resource(cx, how_many_cats, fetch_cat_picture_urls);
///
/// // when we read the signal, it contains either
/// // 1) None (if the Future isn't ready yet) or
/// // 2) Some(T) (if the future's already resolved)
/// assert_eq!(cats(), Some(vec!["1".to_string()]));
///
/// // when the signal's value changes, the `Resource` will generate and run a new `Future`
/// set_how_many_cats(2);
/// assert_eq!(cats(), Some(vec!["2".to_string()]));
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
}

// Resources
slotmap::new_key_type! {
    /// Unique ID assigned to a [Resource](crate::Resource).
    pub struct ResourceId;
}

impl<S, T> Clone for Resource<S, T>
where
    S: Clone + 'static,
    T: Clone + 'static,
{
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime,
            id: self.id,
            source_ty: PhantomData,
            out_ty: PhantomData,
        }
    }
}

impl<S, T> Copy for Resource<S, T>
where
    S: Clone + 'static,
    T: Clone + 'static,
{
}

#[cfg(not(feature = "stable"))]
impl<S, T> FnOnce<()> for Resource<S, T>
where
    S: Clone + 'static,
    T: Clone + 'static,
{
    type Output = Option<T>;

    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.read()
    }
}

#[cfg(not(feature = "stable"))]
impl<S, T> FnMut<()> for Resource<S, T>
where
    S: Clone + 'static,
    T: Clone + 'static,
{
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.read()
    }
}

#[cfg(not(feature = "stable"))]
impl<S, T> Fn<()> for Resource<S, T>
where
    S: Clone + 'static,
    T: Clone + 'static,
{
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.read()
    }
}

#[derive(Clone)]
pub(crate) struct ResourceState<S, T>
where
    S: 'static,
    T: 'static,
{
    scope: Scope,
    value: ReadSignal<Option<T>>,
    set_value: WriteSignal<Option<T>>,
    pub loading: ReadSignal<bool>,
    set_loading: WriteSignal<bool>,
    source: Memo<S>,
    #[allow(clippy::type_complexity)]
    fetcher: Rc<dyn Fn(S) -> Pin<Box<dyn Future<Output = T>>>>,
    resolved: Rc<Cell<bool>>,
    scheduled: Rc<Cell<bool>>,
    suspense_contexts: Rc<RefCell<HashSet<SuspenseContext>>>,
}

impl<S, T> ResourceState<S, T>
where
    S: Clone + 'static,
    T: 'static,
{
    pub fn read(&self) -> Option<T>
    where
        T: Clone,
    {
        self.with(T::clone)
    }

    pub fn with<U>(&self, f: impl FnOnce(&T) -> U) -> Option<U> {
        let suspense_cx = use_context::<SuspenseContext>(self.scope);

        let v = self
            .value
            .try_with(|n| n.as_ref().map(|n| Some(f(n))))
            .ok()?
            .flatten();

        let suspense_contexts = self.suspense_contexts.clone();
        let has_value = v.is_some();

        let increment = move |_: Option<()>| {
            if let Some(s) = &suspense_cx {
                let mut contexts = suspense_contexts.borrow_mut();
                if !contexts.contains(s) {
                    contexts.insert(*s);

                    // on subsequent reads, increment will be triggered in load()
                    // because the context has been tracked here
                    // on the first read, resource is already loading without having incremented
                    if !has_value {
                        s.increment();
                    }
                }
            }
        };

        create_isomorphic_effect(self.scope, increment);
        v
    }

    pub fn refetch(&self) {
        self.load(true);
    }

    fn load(&self, refetching: bool) {
        // doesn't refetch if already refetching
        if refetching && self.scheduled.get() {
            return;
        }

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
                suspense_context.increment();
            }

            // run the Future
            spawn_local({
                let resolved = self.resolved.clone();
                let set_value = self.set_value;
                let set_loading = self.set_loading;
                async move {
                    let res = fut.await;

                    resolved.set(true);

                    set_value.update(|n| *n = Some(res));
                    set_loading.update(|n| *n = false);

                    for suspense_context in suspense_contexts.borrow().iter() {
                        suspense_context.decrement();
                    }
                }
            })
        });
    }

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
        create_isomorphic_effect(self.scope, move |_| {
            value.with({
                let mut tx = tx.clone();
                move |value| {
                    if let Some(value) = value.as_ref() {
                        tx.try_send((id, value.to_json().expect("could not serialize Resource")))
                            .expect("failed while trying to write to Resource serializer");
                    }
                }
            })
        });
        Box::pin(async move {
            rx.next().await.expect("failed while trying to resolve Resource serializer")
        })
    }
}

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
}

impl<S, T> SerializableResource for ResourceState<S, T>
where
    S: Clone,
    T: Serializable,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn to_serialization_resolver(
        &self,
        id: ResourceId,
    ) -> Pin<Box<dyn Future<Output = (ResourceId, String)>>> {
        let fut = self.resource_to_serialization_resolver(id);
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
