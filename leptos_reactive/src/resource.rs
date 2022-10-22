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

use serde::{de::DeserializeOwned, Serialize};

use crate::{
    create_isomorphic_effect, create_memo, create_signal, queue_microtask, runtime::Runtime,
    spawn::spawn_local, use_context, Memo, ReadSignal, Scope, ScopeProperty, SuspenseContext,
    WriteSignal,
};

/// Creates [Resource](crate::Resource), which is a signal that reflects the
/// current state of an asynchronous task, allowing you to integrate
/// `async` [Future]s into the synchronous reactive system.
///
/// Takes a `fetcher` function that generates a [Future] when called and a
/// `source` signal that provides the argument for the `fetcher`. Whenever
/// the value of the `source` changes, a new [Future] will be created and run.
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(|cx| {
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
/// # }).dispose();
/// ```
pub fn create_resource<S, T, Fu>(
    cx: Scope,
    source: impl Fn() -> S + 'static,
    fetcher: impl Fn(S) -> Fu + 'static,
) -> Resource<S, T>
where
    S: PartialEq + Debug + Clone + 'static,
    T: Debug + Clone + Serialize + DeserializeOwned + 'static,
    Fu: Future<Output = T> + 'static,
{
    #[cfg(not(feature = "ssr"))]
    let initial_value = None;
    #[cfg(feature = "ssr")]
    let initial_value = {
        use futures::FutureExt;

        let initial_fut = fetcher(source());
        initial_fut.now_or_never()
    };

    create_resource_with_initial_value(cx, source, fetcher, initial_value)
}

/// Creates a [Resource](crate::Resource) with the given initial value, which
/// will only generate and run a [Future] using the `fetcher` when the `source` changes.
pub fn create_resource_with_initial_value<S, T, Fu>(
    cx: Scope,
    source: impl Fn() -> S + 'static,
    fetcher: impl Fn(S) -> Fu + 'static,
    initial_value: Option<T>,
) -> Resource<S, T>
where
    S: PartialEq + Debug + Clone + 'static,
    T: Debug + Clone + Serialize + DeserializeOwned + 'static,
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

    let id = cx.runtime.create_resource(Rc::clone(&r));

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

#[cfg(not(feature = "hydrate"))]
fn load_resource<S, T>(_cx: Scope, _id: ResourceId, r: Rc<ResourceState<S, T>>)
where
    S: PartialEq + Debug + Clone + 'static,
    T: Debug + Clone + Serialize + DeserializeOwned + 'static,
{
    r.load(false)
}

#[cfg(feature = "hydrate")]
fn load_resource<S, T>(cx: Scope, id: ResourceId, r: Rc<ResourceState<S, T>>)
where
    S: PartialEq + Debug + Clone + 'static,
    T: Debug + Clone + Serialize + DeserializeOwned + 'static,
{
    use wasm_bindgen::{JsCast, UnwrapThrowExt};

    if let Some(ref mut context) = *cx.runtime.shared_context.borrow_mut() {
        if let Some(data) = context.resolved_resources.remove(&id) {
            context.pending_resources.remove(&id); // no longer pending
            r.resolved.set(true);
            let res =
                serde_json::from_str(&data).expect_throw("could not deserialize Resource JSON");
            r.set_value.update(|n| *n = Some(res));
            r.set_loading.update(|n| *n = false);

            // for reactivity
            _ = r.source.try_with(|n| n.clone());
        } else if context.pending_resources.remove(&id) {
            r.set_loading.update(|n| *n = true);

            let resolve = {
                let resolved = r.resolved.clone();
                let set_value = r.set_value;
                let set_loading = r.set_loading;
                move |res: String| {
                    let res = serde_json::from_str(&res)
                        .expect_throw("could not deserialize JSON for already-resolved Resource");
                    resolved.set(true);
                    set_value.update(|n| *n = res);
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
            let id = serde_json::to_string(&id).expect_throw("could not deserialize Resource ID");
            _ = js_sys::Reflect::set(
                &resource_resolvers,
                &wasm_bindgen::JsValue::from_str(&id),
                resolve.as_ref().unchecked_ref(),
            );

            // for reactivity
            _ = r.source.get();
        } else {
            r.load(false);
        }
    } else {
        r.load(false)
    }
}

impl<S, T> Resource<S, T>
where
    S: Debug + Clone + 'static,
    T: Debug + Clone + 'static,
{
    pub fn read(&self) -> Option<T> {
        self.runtime
            .resource(self.id, |resource: &ResourceState<S, T>| resource.read())
    }

    pub fn loading(&self) -> bool {
        self.runtime
            .resource(self.id, |resource: &ResourceState<S, T>| {
                resource.loading.try_with(|n| *n).unwrap_or(false)
            })
    }

    pub fn refetch(&self) {
        self.runtime
            .resource(self.id, |resource: &ResourceState<S, T>| resource.refetch())
    }

    #[cfg(feature = "ssr")]
    pub async fn to_serialization_resolver(&self) -> (ResourceId, String)
    where
        T: Serialize + DeserializeOwned,
    {
        self.runtime
            .resource(self.id, |resource: &ResourceState<S, T>| {
                resource.to_serialization_resolver(self.id)
            })
            .await
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Resource<S, T>
where
    S: Debug + Clone + 'static,
    T: Debug + Clone + 'static,
{
    runtime: &'static Runtime,
    pub(crate) id: ResourceId,
    pub(crate) source_ty: PhantomData<S>,
    pub(crate) out_ty: PhantomData<T>,
}

// Resources
slotmap::new_key_type! { pub struct ResourceId; }

impl<S, T> Clone for Resource<S, T>
where
    S: Debug + Clone + 'static,
    T: Debug + Clone + 'static,
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
    S: Debug + Clone + 'static,
    T: Debug + Clone + 'static,
{
}

#[cfg(not(feature = "stable"))]
impl<S, T> FnOnce<()> for Resource<S, T>
where
    S: Debug + Clone + 'static,
    T: Debug + Clone + 'static,
{
    type Output = Option<T>;

    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.read()
    }
}

#[cfg(not(feature = "stable"))]
impl<S, T> FnMut<()> for Resource<S, T>
where
    S: Debug + Clone + 'static,
    T: Debug + Clone + 'static,
{
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.read()
    }
}

#[cfg(not(feature = "stable"))]
impl<S, T> Fn<()> for Resource<S, T>
where
    S: Debug + Clone + 'static,
    T: Debug + Clone + 'static,
{
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.read()
    }
}

#[derive(Clone)]
pub(crate) struct ResourceState<S, T>
where
    S: 'static,
    T: Clone + Debug + 'static,
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
    S: Debug + Clone + 'static,
    T: Debug + Clone + 'static,
{
    pub fn read(&self) -> Option<T> {
        let suspense_cx = use_context::<SuspenseContext>(self.scope);

        let v = self.value.try_with(|n| n.clone()).ok()?;

        let suspense_contexts = self.suspense_contexts.clone();
        let has_value = v.is_some();

        let increment = move |_| {
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

    #[cfg(feature = "ssr")]
    pub fn resource_to_serialization_resolver(
        &self,
        id: ResourceId,
    ) -> std::pin::Pin<Box<dyn futures::Future<Output = (ResourceId, String)>>>
    where
        T: Serialize,
    {
        let fut = (self.fetcher)(self.source.get());
        Box::pin(async move {
            let res = fut.await;
            (
                id,
                serde_json::to_string(&res).expect("could not serialize Resource"),
            )
        })
    }
}

pub(crate) trait AnyResource {
    fn as_any(&self) -> &dyn Any;

    #[cfg(feature = "ssr")]
    fn to_serialization_resolver(
        &self,
        id: ResourceId,
    ) -> Pin<Box<dyn Future<Output = (ResourceId, String)>>>;
}

impl<S, T> AnyResource for ResourceState<S, T>
where
    S: Debug + Clone,
    T: Clone + Debug + Serialize + DeserializeOwned,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[cfg(feature = "ssr")]
    fn to_serialization_resolver(
        &self,
        id: ResourceId,
    ) -> Pin<Box<dyn Future<Output = (ResourceId, String)>>> {
        let fut = self.resource_to_serialization_resolver(id);
        Box::pin(fut)
    }
}
