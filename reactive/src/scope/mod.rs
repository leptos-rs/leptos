pub mod owner;
mod scope_arena;
use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    cell::RefCell,
    collections::HashMap,
    marker::PhantomData,
};

use scope_arena::*;

use crate::root_context::RootContext;

pub type Scope<'a> = BoundedScope<'a, 'a>;

#[must_use = "Creating a Scope without calling its disposer will leak memory."]
pub fn create_scope<'disposer>(
    root_context: &'static RootContext,
    f: impl for<'a> FnOnce(Scope<'a>),
) -> ScopeDisposer<'disposer> {
    let inner = ScopeInner::new(root_context);
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
    pub(crate) inner: &'a ScopeInner<'a>,
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

    pub(crate) fn root_context(&self) -> &'static RootContext {
        self.inner.borrow().root_context
    }

    pub fn create_ref<T>(&self, value: T) -> &'a T {
        self.inner
            .arena
            .alloc(bumpalo::boxed::Box::new_in(value, &self.inner.arena))
    }

    pub fn untrack<T>(&self, f: impl FnOnce() -> T) -> T {
        self.inner.root_context.untrack(f)
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

pub(crate) struct ScopeInner<'a> {
    pub(crate) root_context: &'static RootContext,
    pub(crate) parent: Option<&'a ScopeInner<'a>>,
    pub(crate) arena: bumpalo::Bump,
    pub(crate) cleanup_callbacks: RefCell<Vec<&'a mut dyn FnMut()>>,
    pub(crate) context: RefCell<HashMap<TypeId, &'a dyn Any>>,
    pub(crate) children: RefCell<ScopeArena<*mut ScopeInner<'a>>>,
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
}

/// A handle that allows cleaning up a [`Scope`].
pub struct ScopeDisposer<'a> {
    f: Box<dyn FnOnce() + 'a>,
}

impl<'a> ScopeDisposer<'a> {
    fn new(f: impl FnOnce() + 'a) -> Self {
        Self { f: Box::new(f) }
    }

    /// Clean up the resources owned by the [`Scope`].
    ///
    /// This method will clean up resources in a certain order such that it is impossible to access a
    /// dangling-reference within cleanup callbacks, effects, etc.
    ///
    /// If a [`Scope`] has already been disposed, calling it again does nothing.
    ///
    /// # Safety
    ///
    /// `dispose` should not be called inside the `create_scope` or `create_child_scope` closure.
    pub unsafe fn dispose(self) {
        (self.f)();
    }
}
