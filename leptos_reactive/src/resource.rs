use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    fmt::Debug,
    future::Future,
    marker::PhantomData,
    rc::Rc,
};

use serde::{Deserialize, Serialize};

use crate::{
    create_effect, create_memo, create_signal, queue_microtask, runtime::Runtime,
    spawn::spawn_local, Memo, ReadSignal, Scope, ScopeId, SuspenseContext, WriteSignal,
};

pub fn create_resource<S, T, Fu>(
    cx: Scope,
    source: ReadSignal<S>,
    fetcher: impl Fn(&S) -> Fu + 'static,
) -> Resource<S, T, Fu>
where
    S: Debug + Clone + 'static,
    T: Debug + Clone + 'static,
    Fu: Future<Output = T> + 'static,
{
    create_resource_with_initial_value(cx, source, fetcher, None)
}

pub fn create_resource_with_initial_value<S, T, Fu>(
    cx: Scope,
    source: ReadSignal<S>,
    fetcher: impl Fn(&S) -> Fu + 'static,
    initial_value: Option<T>,
) -> Resource<S, T, Fu>
where
    S: Debug + Clone + 'static,
    T: Debug + Clone + 'static,
    Fu: Future<Output = T> + 'static,
{
    let resolved = initial_value.is_some();
    let (value, set_value) = create_signal(cx, initial_value);
    let (loading, set_loading) = create_signal(cx, false);
    let (track, trigger) = create_signal(cx, 0);
    let fetcher = Rc::new(fetcher);
    let source = create_memo(cx, move |_| source());

    // TODO hydration/streaming logic

    let r = Rc::new(ResourceState {
        scope: cx,
        value,
        set_value,
        loading,
        set_loading,
        track,
        trigger,
        source,
        fetcher,
        resolved: Rc::new(Cell::new(resolved)),
        scheduled: Rc::new(Cell::new(false)),
        suspense_contexts: Default::default(),
    });

    // initial load fires immediately
    create_effect(cx, {
        let r = Rc::clone(&r);
        move |_| r.load(false)
    });

    let id = cx.push_resource(r);

    Resource {
        runtime: cx.runtime,
        scope: cx.id,
        id,
        source_ty: PhantomData,
        out_ty: PhantomData,
        fut_ty: PhantomData,
    }
}

impl<S, T, Fu> Resource<S, T, Fu>
where
    S: Debug + Clone + 'static,
    T: Debug + Clone + 'static,
    Fu: Future<Output = T> + 'static,
{
    pub fn read(&self) -> Option<T> {
        self.runtime.resource(
            (self.scope, self.id),
            |resource: &ResourceState<S, T, Fu>| resource.read(),
        )
    }

    pub fn loading(&self) -> bool {
        self.runtime.resource(
            (self.scope, self.id),
            |resource: &ResourceState<S, T, Fu>| resource.loading.get(),
        )
    }

    pub fn refetch(&self) {
        self.runtime.resource(
            (self.scope, self.id),
            |resource: &ResourceState<S, T, Fu>| resource.refetch(),
        )
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Resource<S, T, Fu>
where
    S: Debug + Clone + 'static,
    T: Debug + Clone + 'static,
    Fu: Future<Output = T> + 'static,
{
    runtime: &'static Runtime,
    pub(crate) scope: ScopeId,
    pub(crate) id: ResourceId,
    pub(crate) source_ty: PhantomData<S>,
    pub(crate) out_ty: PhantomData<T>,
    pub(crate) fut_ty: PhantomData<Fu>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ResourceId(pub(crate) usize);

impl<S, T, Fu> Clone for Resource<S, T, Fu>
where
    S: Debug + Clone + 'static,
    T: Debug + Clone + 'static,
    Fu: Future<Output = T> + 'static,
{
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime,
            scope: self.scope,
            id: self.id,
            source_ty: PhantomData,
            out_ty: PhantomData,
            fut_ty: PhantomData,
        }
    }
}

impl<S, T, Fu> Copy for Resource<S, T, Fu>
where
    S: Debug + Clone + 'static,
    T: Debug + Clone + 'static,
    Fu: Future<Output = T> + 'static,
{
}

#[derive(Clone)]
pub struct ResourceState<S, T, Fu>
where
    S: 'static,
    T: Clone + Debug + 'static,
    Fu: Future<Output = T>,
{
    scope: Scope,
    value: ReadSignal<Option<T>>,
    set_value: WriteSignal<Option<T>>,
    pub loading: ReadSignal<bool>,
    set_loading: WriteSignal<bool>,
    track: ReadSignal<usize>,
    trigger: WriteSignal<usize>,
    source: Memo<S>,
    fetcher: Rc<dyn Fn(&S) -> Fu>,
    resolved: Rc<Cell<bool>>,
    scheduled: Rc<Cell<bool>>,
    suspense_contexts: Rc<RefCell<HashSet<SuspenseContext>>>,
}

impl<S, T, Fu> ResourceState<S, T, Fu>
where
    S: Debug + Clone + 'static,
    T: Debug + Clone + 'static,
    Fu: Future<Output = T> + 'static,
{
    pub fn read(&self) -> Option<T> {
        let suspense_cx = self.scope.use_context::<SuspenseContext>();

        let v = self.value.get();

        let suspense_contexts = self.suspense_contexts.clone();
        let has_value = v.is_some();
        create_effect(self.scope, move |_| {
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
        });

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

        let loaded_under_transition = self.scope.runtime.running_transition().is_some();

        let fut = (self.fetcher)(&self.source.get());

        // `scheduled` is true for the rest of this code only
        self.scheduled.set(true);
        queue_microtask({
            let scheduled = Rc::clone(&self.scheduled);
            move || {
                scheduled.set(false);
            }
        });

        self.set_loading.update(|n| *n = true);
        self.trigger.update(|n| *n += 1);

        // increment counter everywhere it's read
        let suspense_contexts = self.suspense_contexts.clone();
        let running_transition = self.scope.runtime.running_transition();
        for suspense_context in suspense_contexts.borrow().iter() {
            suspense_context.increment();
            if let Some(transition) = &running_transition {
                transition
                    .resources
                    .borrow_mut()
                    .insert(suspense_context.pending_resources);
            }
        }

        // run the Future
        spawn_local({
            let resolved = self.resolved.clone();
            let scope = self.scope;
            let set_value = self.set_value;
            let set_loading = self.set_loading;
            async move {
                let res = fut.await;
                resolved.set(true);

                // TODO hydration

                if let Some(transition) = scope.runtime.transition() {
                    // TODO transition
                }

                set_value.update(|n| *n = Some(res));
                set_loading.update(|n| *n = false);

                for suspense_context in suspense_contexts.borrow().iter() {
                    suspense_context.decrement();
                }
            }
        })
    }
}
