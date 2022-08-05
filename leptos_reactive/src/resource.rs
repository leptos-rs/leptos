use std::{cell::RefCell, future::Future, rc::Rc};

use futures::future::{AbortHandle, Abortable};

use crate::{spawn_local, ReadSignal, ReadSignalRef, Scope, SuspenseContext, WriteSignal};

pub enum ResourceState<T> {
    Idle,
    Pending { abort_handle: AbortHandle },
    Ready { data: T },
}

impl<T> std::fmt::Debug for ResourceState<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Pending { abort_handle } => f
                .debug_struct("Pending")
                .field("abort_handle", abort_handle)
                .finish(),
            Self::Ready { data } => f.debug_struct("Ready").field("data", data).finish(),
        }
    }
}

pub struct Resource<'a, S, T, Fu>
where
    S: 'static + Clone,
    T: 'static,
    Fu: Future<Output = T>,
{
    state: ReadSignal<ResourceState<T>>,
    set_state: WriteSignal<ResourceState<T>>,
    source: ReadSignal<S>,
    source_memoized: Rc<RefCell<Option<S>>>,
    fetcher: Rc<dyn Fn(&S) -> Fu>,
    cx: Scope<'a>,
}

impl<'a, S, T, Fu> Clone for Resource<'a, S, T, Fu>
where
    S: 'static + Clone + PartialEq,
    T: 'static,
    Fu: Future<Output = T>,
{
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            set_state: self.set_state.clone(),
            source: self.source.clone(),
            source_memoized: Rc::clone(&self.source_memoized),
            fetcher: self.fetcher.clone(),
            cx: self.cx,
        }
    }
}

impl<'a, S, T, Fu> Resource<'a, S, T, Fu>
where
    S: 'static + Clone + PartialEq,
    T: 'static,
    Fu: Future<Output = T> + 'static,
{
    pub fn new(cx: Scope<'a>, source: ReadSignal<S>, fetcher: impl Fn(&S) -> Fu + 'static) -> Self {
        // create signals to handle response
        let (state, set_state) = cx.create_signal_owned(ResourceState::Idle);
        let fetcher = Rc::new(fetcher);

        // return the Resource synchronously
        Self {
            state,
            set_state,
            source,
            source_memoized: Default::default(),
            fetcher,
            cx,
        }
    }

    pub fn read(&self) -> ReadSignalRef<ResourceState<T>> {
        self.cx.create_effect(|| {
            // reactivity should only be driven by the source signal
            let source = self.source.get();
            let source_has_changed = {
                let mut prev_source = self.source_memoized.borrow_mut();
                let source_has_changed = prev_source.as_ref() != Some(&source);
                if source_has_changed {
                    *prev_source = Some(source.clone());
                }
                source_has_changed
            };

            match (source_has_changed, &*self.state.get_untracked()) {
                // if it's already loaded or is loading and source hasn't changed, return value when read
                (false, ResourceState::Ready { .. } | ResourceState::Pending { .. }) => {
                    crate::debug_warn!("\nResource::read() called while ResourceState is Ready or Pending");
                }
                // if source has changed and we have a result, run fetch logic
                (true, ResourceState::Ready { .. }) => {
                    crate::debug_warn!("\nResource::read() called while ResourceState::Ready but source has changed");
                    self.refetch();
                }
                // if source has changed and it's loading, abort loading and run fetch logic
                (true, ResourceState::Pending { abort_handle}) => {
                    crate::debug_warn!("\nResource::read() called while ResourceState::Pending but source has changed");
                    abort_handle.abort();
                    self.refetch();
                }
                // if this is the first read, run the logic
                (_, ResourceState::Idle) => {
                    crate::debug_warn!("\nResource::read() called while ResourceState is idle");
                    self.refetch();
                }
            }
        });

        self.state.get()
    }

    pub fn refetch(&self) {
        let suspense_cx = self.cx.use_context::<SuspenseContext>().cloned();
        if let Some(context) = &suspense_cx {
            context.increment();
        }

        // actually await the future
        let source = self.source.clone();
        let set_state = self.set_state.clone();
        let fetcher = Rc::clone(&self.fetcher);

        // get Future from factory function and make it abortable
        let fut = (fetcher)(&source.get_untracked());
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let fut = Abortable::new(fut, abort_registration);

        // set loading state
        set_state.update(|state| *state = ResourceState::Pending { abort_handle });

        spawn_local(async move {
            let data = fut.await;

            // if future has not been aborted, update state
            if let Ok(data) = data {
                set_state.update(move |state| *state = ResourceState::Ready { data });
            }

            // if any case, decrement the read counter
            if let Some(suspense_cx) = &suspense_cx {
                suspense_cx.decrement();
            }
        });
    }

    pub fn mutate(&self, update_fn: impl FnOnce(&mut ResourceState<T>)) {
        self.set_state.update(update_fn);
    }
}
