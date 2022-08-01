use crate::scope_arena::ScopeArena;
use crate::{EffectInner, Resource, SignalState};

use super::{root_context::RootContext, Effect, ReadSignal, WriteSignal};
use std::future::Future;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    rc::Rc,
};

pub type Scope<'a> = BoundedScope<'a, 'a>;

#[must_use = "Creating a Scope without calling its disposer will leak memory."]
pub fn create_scope<'disposer>(
    root_context: &'static RootContext,
    f: impl for<'a> FnOnce(Scope<'a>),
) -> ScopeDisposer<'disposer> {
    let inner = ScopeInner::new(&root_context);
    let boxed_inner = Box::new(inner);
    let inner_ptr = Box::into_raw(boxed_inner);

    // TODO safety
    root_context.untrack(|| f(unsafe { Scope::new(&*inner_ptr) }));

    // TODO safety
    ScopeDisposer::new(move || unsafe {
        // SAFETY: Safe because ptr created using Box::into_raw.
        let boxed = Box::from_raw(inner_ptr);
        // SAFETY: Outside of call to f.
        boxed.dispose();
    })
}

#[derive(Clone, Copy)]
pub struct BoundedScope<'a, 'b: 'a> {
    inner: &'a ScopeInner<'a>,
    /// `&'b` for covariance!
    _phantom: PhantomData<&'b ()>,
}

impl<'a, 'b> BoundedScope<'a, 'b> {
    fn new(inner: &'a ScopeInner<'a>) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }

    pub fn create_ref<T>(self, value: T) -> &'a T {
        self.inner.create_ref(value)
    }

    pub fn create_signal<T>(self, value: T) -> (&'a ReadSignal<T>, &'a WriteSignal<T>) {
        let (read, write) = self.inner.signal(value);
        (self.create_ref(read), self.create_ref(write))
    }

    pub fn create_signal_owned<T>(self, value: T) -> (ReadSignal<T>, WriteSignal<T>) {
        self.inner.signal(value)
    }

    /// An effect is an observer that runs a side effect that depends Signals.
    /// It will be run once immediately. The effect automatically subscribes to the Signals
    /// it reads, so it will run again when any of them change.
    pub fn create_effect(self, f: impl FnMut()) {
        self.inner.create_effect(f)
    }

    pub fn untrack<T>(&self, f: impl Fn() -> T) -> T {
        self.inner.root_context.untrack(f)
    }

    pub fn create_memo<T>(self, f: impl Fn() -> T) -> &'a ReadSignal<T> {
        self.inner.create_memo(f)
    }

    pub fn provide_context<T: 'static>(self, value: T) {
        self.inner.provide_context(value)
    }

    pub fn use_context<T: 'static>(self) -> Option<&'a T> {
        self.inner.use_context()
    }

    pub fn create_resource<S, T, Fu>(
        self,
        source: ReadSignal<S>,
        fetcher: impl Fn(&S) -> Fu + 'static,
    ) -> &'a Resource<S, T, Fu>
    where
        S: 'static,
        T: 'static,
        Fu: Future<Output = T> + 'static,
    {
        self.create_ref(Resource::new(self, source, fetcher))
    }

    pub fn child_scope<F>(self, f: F) -> ScopeDisposer<'a>
    where
        F: for<'child_lifetime> FnOnce(BoundedScope<'child_lifetime, 'a>),
    {
        let mut child = ScopeInner::new(self.inner.root_context);
        // SAFETY: The only fields that are accessed on self from child is `context` which does not
        // have any lifetime annotations.
        child.parent = Some(unsafe { &*(self.inner as *const _) });
        let boxed = Box::new(child);
        let ptr = Box::into_raw(boxed);

        let key = self
            .inner
            .children
            .borrow_mut()
            // SAFETY: None of the fields of ptr are accessed through child_scopes therefore we can
            // safely transmute the lifetime.
            .insert(unsafe { std::mem::transmute(ptr) });

        // SAFETY: the address of the cx lives as long as 'a because:
        // - It is allocated on the heap and therefore has a stable address.
        // - self.child_cx is append only. That means that the Box<cx> will not be dropped until Self is
        //   dropped.
        f(unsafe { Scope::new(&*ptr) });
        //                      ^^^ -> `ptr` is still accessible here after call to f.
        ScopeDisposer::new(move || unsafe {
            let cx = self.inner.children.borrow_mut().remove(key).unwrap();
            // SAFETY: Safe because ptr created using Box::into_raw and closure cannot live longer
            // than 'a.
            let cx = Box::from_raw(cx);
            // SAFETY: Outside of call to f.
            cx.dispose();
        })
    }
}

struct ScopeInner<'a> {
    pub(crate) root_context: &'static RootContext,

    pub(crate) parent: Option<&'a ScopeInner<'a>>,

    arena: bumpalo::Bump,

    pub(crate) cleanup_callbacks: RefCell<Vec<&'a mut dyn FnMut()>>,

    context: RefCell<HashMap<TypeId, &'a dyn Any>>,
    children: RefCell<ScopeArena<*mut ScopeInner<'a>>>,
}

impl<'a> ScopeInner<'a> {
    pub fn new(root_context: &'static RootContext) -> Self {
        Self {
            root_context,
            parent: None,
            arena: bumpalo::Bump::new(),
            cleanup_callbacks: RefCell::new(Vec::new()),
            context: RefCell::new(HashMap::new()),
            children: RefCell::new(ScopeArena::new()),
        }
    }

    pub fn create_ref<T>(&self, value: T) -> &T {
        self.arena.alloc(value)
    }

    pub fn signal<T>(&self, value: T) -> (ReadSignal<T>, WriteSignal<T>) {
        let state = Rc::new(SignalState {
            value: RefCell::new(value),
            subscriptions: RefCell::new(HashSet::new()),
        });

        let writer = WriteSignal {
            inner: Rc::downgrade(&state),
        };

        let reader = ReadSignal {
            stack: self.root_context,
            inner: state,
        };

        (reader, writer)
    }

    pub fn untrack<T>(&self, f: impl Fn() -> T) -> T {
        self.root_context.untrack(f)
    }

    pub fn create_memo<T>(&self, f: impl Fn() -> T) -> &ReadSignal<T> {
        // the initial value should simply be an untracked call, based on initial Signal values
        // we need this initial call because the Signal must always have a value
        // (otherwise every computed Signal would be typed Signal<Option<T>>)
        // but if we track the initial here, because we haven't created the effect yet,
        // this would subscribe to the surrounding effect, which isn't what we want
        // untracking the initial_value call solves this chicken-and-egg problem
        let initial_value = self.untrack(&f);

        // now create the signal with that untracked initial values
        let (read, write) = self.signal(initial_value);

        // and start tracking based on whatever Signals are inside the computed fn
        self.create_effect(move || write(|n| *n = f()));
        self.create_ref(read)
    }

    /// An effect is an observer that runs a side effect that depends Signals.
    /// It will be run once immediately. The effect automatically subscribes to the Signals
    /// it reads, so it will run again when any of them change.
    pub fn create_effect(&self, f: impl FnMut()) {
        let f = Box::new(f) as Box<dyn FnMut()>;
        // TODO safety
        let f: Box<dyn FnMut() + 'static> = unsafe { std::mem::transmute(f) };

        // the Effect will be owned by the arena, which means we can pass
        // its reference around without worrying about ownership
        // because it's immediately executed, it will never be dropped
        // unless no Signal has it as a dependency and it's not in the stack,
        // in which case it could never be called again anyway
        let effect_ref = self.arena.alloc(Effect {
            inner: Rc::new(EffectInner {
                stack: self.root_context,
                f: RefCell::new(Box::new(f)),
                dependencies: RefCell::new(Vec::new()),
            }),
        });
        effect_ref.execute();
    }

    pub(crate) fn dispose(self) {
        // Drop child scopes.
        for (_, child) in self.children.borrow_mut().drain() {
            // SAFETY: These pointers were allocated in Self::create_child_scope.
            let cx = unsafe { Box::from_raw(child) };
            // Dispose of cx if it has not already been disposed.
            cx.dispose();
        }

        // Call cleanup functions in an untracked scope.
        for cb in self.cleanup_callbacks.borrow_mut().drain(..) {
            cb();
        }

        // unnecessary but explicit!
        drop(self)
    }

    pub fn provide_context<T: 'static>(&'a self, value: T) {
        let id = value.type_id();
        let value = self.arena.alloc(value);
        self.context.borrow_mut().insert(id, &*value);
    }

    pub fn use_context<T: 'static>(&'a self) -> Option<&T> {
        let id = TypeId::of::<T>();
        let local_value = self
            .context
            .borrow()
            .get(&id)
            .and_then(|val| val.downcast_ref::<T>());
        match local_value {
            Some(val) => Some(val),
            None => self.parent.and_then(|parent| parent.use_context::<T>()),
        }
    }
}

/// A handle that allows cleaning up a [`Scope`].
pub struct ScopeDisposer<'a> {
    f: Box<dyn FnOnce() + 'a>,
}

impl<'a> ScopeDisposer<'a> {
    fn new(f: impl FnOnce() + 'a) -> Self {
        Self { f: Box::new(f) }
    }

    pub unsafe fn dispose(self) {
        (self.f)();
    }
}
