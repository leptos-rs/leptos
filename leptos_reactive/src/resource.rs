use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    fmt::Debug,
    future::Future,
    rc::Rc,
};

use crate::{
    queue_microtask, spawn::spawn_local, Memo, ReadSignal, Scope, SuspenseContext, WriteSignal,
};

impl Scope {
    pub fn create_resource<S, T, Fu>(
        self,
        source: ReadSignal<S>,
        fetcher: impl Fn(&S) -> Fu + 'static,
    ) -> Rc<Resource<S, T, Fu>>
    where
        S: Debug + Clone + 'static,
        T: Debug + Clone + 'static,
        Fu: Future<Output = T> + 'static,
    {
        self.create_resource_with_initial_value(source, fetcher, None)
    }

    pub fn create_resource_with_initial_value<S, T, Fu>(
        self,
        source: ReadSignal<S>,
        fetcher: impl Fn(&S) -> Fu + 'static,
        initial_value: Option<T>,
    ) -> Rc<Resource<S, T, Fu>>
    where
        S: Debug + Clone + 'static,
        T: Debug + Clone + 'static,
        Fu: Future<Output = T> + 'static,
    {
        let resolved = initial_value.is_some();
        let (value, set_value) = self.create_signal(initial_value);
        let (loading, set_loading) = self.create_signal(false);
        let (track, trigger) = self.create_signal(0);
        let fetcher = Rc::new(fetcher);
        let source = self.create_memo(move |_| source());

        // TODO hydration/streaming logic

        let r = Rc::new(Resource {
            scope: self,
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
        self.create_effect({
            let r = Rc::clone(&r);
            move |_| r.load(false)
        });

        r
    }
}

#[derive(Clone)]
pub struct Resource<S, T, Fu>
where
    S: 'static,
    T: Debug + 'static,
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

impl<S, T, Fu> Resource<S, T, Fu>
where
    S: Debug + Clone + 'static,
    T: Debug + Clone + 'static,
    Fu: Future<Output = T> + 'static,
{
    pub fn read(&self) -> Option<T> {
        let suspense_cx = self.scope.use_context::<SuspenseContext>();

        let v = self.value.get();

        if let Some(s) = &suspense_cx {
            let mut contexts = self.suspense_contexts.borrow_mut();
            if !contexts.contains(s) {
                contexts.insert(*s);
                s.increment();
            }
        }

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

        let fut = /* self.scope.untrack(||  */(self.fetcher)(&self.source.get())/* ) */;

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

        // run the Future
        spawn_local({
            let resolved = self.resolved.clone();
            let scope = self.scope;
            let set_value = self.set_value;
            let set_loading = self.set_loading;
            let suspense_contexts = self.suspense_contexts.clone();
            async move {
                let res = fut.await;
                resolved.set(true);

                // TODO hydration

                if let Some(transition) = scope.runtime.transition() {
                    todo!()
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
