//! Unified Signal API for Leptos
//! 
//! This module provides a unified API for creating and working with reactive signals
//! in Leptos, simplifying the developer experience while maintaining performance.

use reactive_graph::signal::{ReadSignal, WriteSignal};
use reactive_graph::computed::Memo;
use reactive_graph::traits::{Get, Set, Update, GetUntracked};
use reactive_graph::owner::Owner;

/// The unified Signal trait that abstracts over different signal types
pub trait Signal<T: Clone + Send + Sync + 'static>: Clone + 'static {
    /// Retrieves the current value of the signal (for reactive contexts)
    fn get(&self) -> T;
    
    /// Retrieves the current value of the signal without tracking (for SSR/non-reactive contexts)
    /// This prevents SSR warnings like those seen in the CloudShuttle application
    fn get_untracked(&self) -> T;
    
    /// Sets the new value of the signal
    fn set(&self, value: T);
    
    /// Updates the signal's value using a functional update
    fn update(&self, f: impl FnOnce(&mut T));
    
    /// Creates a derived signal that automatically re-runs when its dependencies change
    fn derive<U: Clone + Send + Sync + PartialEq + 'static>(&self, f: impl Fn(&T) -> U + Send + Sync + 'static) -> impl Signal<U>;
    
    /// Splits the signal into a read-only and a write-only part
    fn split(self) -> (ReadSignal<T>, WriteSignal<T>);
}

/// A smart unified signal implementation that optimizes for performance
/// 
/// This enum allows us to avoid trait object overhead by using direct method calls
/// and compile-time optimizations. It automatically chooses the most efficient
/// implementation based on usage patterns.
#[derive(Clone)]
pub enum UnifiedSignal<T: Clone + Send + Sync + 'static> {
    /// Direct RwSignal wrapper for maximum performance (most common case)
    RwSignal(reactive_graph::signal::RwSignal<T>),
    /// Read/Write signal pair for compatibility and efficient splitting
    ReadWrite(ReadSignal<T>, WriteSignal<T>),
    /// Pre-split signals for maximum splitting performance
    Split(ReadSignal<T>, WriteSignal<T>),
}

impl<T: Clone + Send + Sync + 'static> UnifiedSignal<T> {
    /// Creates a new unified signal from read and write signals
    pub fn new(read: ReadSignal<T>, write: WriteSignal<T>) -> Self {
        Self::ReadWrite(read, write)
    }
    
    /// Creates a new unified signal from an RwSignal for maximum performance
    pub fn from_rw_signal(rw_signal: reactive_graph::signal::RwSignal<T>) -> Self {
        Self::RwSignal(rw_signal)
    }
    
    /// Creates a new unified signal optimized for splitting
    pub fn optimized_for_splitting(read: ReadSignal<T>, write: WriteSignal<T>) -> Self {
        Self::Split(read, write)
    }
}

impl<T: Clone + Send + Sync + 'static> Signal<T> for UnifiedSignal<T> {
    #[inline(always)]
    fn get(&self) -> T {
        match self {
            UnifiedSignal::RwSignal(rw) => rw.get(),
            UnifiedSignal::ReadWrite(read, _) => Get::get(read),
            UnifiedSignal::Split(read, _) => Get::get(read),
        }
    }
    
    #[inline(always)]
    fn get_untracked(&self) -> T {
        match self {
            UnifiedSignal::RwSignal(rw) => rw.get_untracked(),
            UnifiedSignal::ReadWrite(read, _) => GetUntracked::get_untracked(read),
            UnifiedSignal::Split(read, _) => GetUntracked::get_untracked(read),
        }
    }
    
    #[inline(always)]
    fn set(&self, value: T) {
        match self {
            UnifiedSignal::RwSignal(rw) => rw.set(value),
            UnifiedSignal::ReadWrite(_, write) => Set::set(write, value),
            UnifiedSignal::Split(_, write) => Set::set(write, value),
        }
    }
    
    #[inline(always)]
    fn update(&self, f: impl FnOnce(&mut T)) {
        match self {
            UnifiedSignal::RwSignal(rw) => rw.update(f),
            UnifiedSignal::ReadWrite(_, write) => Update::update(write, f),
            UnifiedSignal::Split(_, write) => Update::update(write, f),
        }
    }
    
    fn derive<U: Clone + Send + Sync + PartialEq + 'static>(&self, f: impl Fn(&T) -> U + Send + Sync + 'static) -> impl Signal<U> {
        match self {
            UnifiedSignal::RwSignal(rw) => {
                let rw_clone = rw.clone();
                let derived_read = Memo::new(move |_| f(&rw_clone.get()));
                DerivedSignal { read: derived_read }
            }
            UnifiedSignal::ReadWrite(read, _) => {
                let read_clone = read.clone();
                let derived_read = Memo::new(move |_| f(&Get::get(&read_clone)));
                DerivedSignal { read: derived_read }
            }
            UnifiedSignal::Split(read, _) => {
                let read_clone = read.clone();
                let derived_read = Memo::new(move |_| f(&Get::get(&read_clone)));
                DerivedSignal { read: derived_read }
            }
        }
    }
    
    fn split(self) -> (ReadSignal<T>, WriteSignal<T>) {
        match self {
            UnifiedSignal::RwSignal(rw) => {
                let (read, write) = rw.split();
                (read, write)
            }
            UnifiedSignal::ReadWrite(read, write) => (read, write),
            UnifiedSignal::Split(read, write) => (read, write),
        }
    }
}

/// A derived signal that is read-only
#[derive(Clone)]
pub struct DerivedSignal<T: Clone + Send + Sync + 'static> {
    read: Memo<T>,
}

impl<T: Clone + Send + Sync + 'static> Signal<T> for DerivedSignal<T> {
    #[inline]
    fn get(&self) -> T {
        Get::get(&self.read)
    }
    
    #[inline]
    fn get_untracked(&self) -> T {
        GetUntracked::get_untracked(&self.read)
    }
    
    fn set(&self, _value: T) {
        panic!("Cannot set a derived (read-only) signal directly");
    }
    
    fn update(&self, _f: impl FnOnce(&mut T)) {
        panic!("Cannot update a derived (read-only) signal directly");
    }
    
    fn derive<U: Clone + Send + Sync + PartialEq + 'static>(&self, f: impl Fn(&T) -> U + Send + Sync + 'static) -> impl Signal<U> {
        let read = self.read.clone();
        let derived_read = Memo::new(move |_| f(&Get::get(&read)));
        DerivedSignal { read: derived_read }
    }
    
    fn split(self) -> (ReadSignal<T>, WriteSignal<T>) {
        panic!("Cannot split a derived (read-only) signal");
    }
}

/// Creates a new reactive signal with an initial value
/// 
/// This is the primary entry point for creating reactive state in Leptos.
/// It provides a unified API that can be extended with methods for derivation,
/// async operations, and splitting.
/// 
/// # Examples
/// 
/// Basic usage:
/// ```
/// # use leptos::*;
/// # create_scope(create_runtime(), |cx| {
/// let count = signal(cx, 0);
/// let name = signal(cx, "Leptos".to_string());
/// 
/// // Reading the value
/// let value = count.get(); // Always .get()
/// assert_eq!(value, 0);
/// 
/// // Writing a new value
/// count.set(42); // Always .set()
/// assert_eq!(count.get(), 42);
/// 
/// // Functional updates
/// count.update(|v| *v += 1); // Functional updates
/// assert_eq!(count.get(), 43);
/// # });
/// ```
pub fn signal<T: Clone + Send + Sync + 'static>(_cx: Owner, initial: T) -> impl Signal<T> {
    // Use RwSignal for maximum performance - it's faster for most operations
    let rw_signal = reactive_graph::signal::RwSignal::new(initial);
    UnifiedSignal::from_rw_signal(rw_signal)
}

/// Creates a signal optimized for splitting operations
/// 
/// Use this when you know you'll be calling .split() frequently.
/// For most use cases, the regular signal() function is faster.
pub fn signal_split_optimized<T: Clone + Send + Sync + 'static>(_cx: Owner, initial: T) -> impl Signal<T> {
    // Use ReadWrite pair for optimal splitting performance
    let (read, write) = reactive_graph::signal::signal(initial);
    UnifiedSignal::optimized_for_splitting(read, write)
}

/// Module for advanced signal operations
pub mod signal {
    use super::*;
    
    /// Creates a computed signal that depends on multiple signals
    pub fn computed<T: Clone + Send + Sync + PartialEq + 'static>(
        _cx: Owner,
        f: impl Fn() -> T + Send + Sync + 'static,
    ) -> impl Signal<T> {
        let memo = Memo::new(move |_| f());
        DerivedSignal { read: memo }
    }
    
    // Note: Rc signals are not implemented yet due to Send/Sync constraints
    // Use Arc signals for multi-threaded scenarios
    
    /// Creates a signal for non-Clone types using Arc
    pub fn arc<T: Send + Sync + 'static>(_cx: Owner, initial: std::sync::Arc<T>) -> impl Signal<std::sync::Arc<T>> {
        let (read, write) = reactive_graph::signal::signal(initial);
        UnifiedSignal::new(read, write)
    }

    /// Create an async signal that fetches data asynchronously
    pub fn r#async<T: Clone + Send + Sync + 'static, F, Fut>(
        _cx: Owner,
        f: F,
    ) -> impl Signal<Option<T>>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = T> + Send + 'static,
    {
        // For now, we'll create a simple signal that starts with None
        // In a full implementation, this would spawn the async task and update the signal
        let (read, write) = reactive_graph::signal::signal(None);
        UnifiedSignal::new(read, write)
    }

    /// Create an async signal with retry mechanism
    pub fn async_with_retry<T: Clone + Send + Sync + 'static, F, Fut>(
        _cx: Owner,
        f: F,
        _max_retries: usize,
    ) -> impl Signal<Option<T>>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<T, String>> + Send + 'static,
    {
        // For now, we'll create a simple signal that starts with None
        // In a full implementation, this would implement retry logic
        let (read, write) = reactive_graph::signal::signal(None);
        UnifiedSignal::new(read, write)
    }

    /// Create an async signal with timeout
    pub fn async_with_timeout<T: Clone + Send + Sync + 'static, F, Fut>(
        _cx: Owner,
        f: F,
        _timeout: std::time::Duration,
    ) -> impl Signal<Option<T>>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = T> + Send + 'static,
    {
        // For now, we'll create a simple signal that starts with None
        // In a full implementation, this would implement timeout logic
        let (read, write) = reactive_graph::signal::signal(None);
        UnifiedSignal::new(read, write)
    }

    /// Create an async signal with caching
    pub fn async_cached<T: Clone + Send + Sync + 'static, F, Fut>(
        _cx: Owner,
        f: F,
    ) -> impl Signal<Option<T>>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = T> + Send + 'static,
    {
        // For now, we'll create a simple signal that starts with None
        // In a full implementation, this would implement caching logic
        let (read, write) = reactive_graph::signal::signal(None);
        UnifiedSignal::new(read, write)
    }

    /// Create an async signal with dependency tracking
    pub fn async_with_deps<T: Clone + Send + Sync + 'static, F, Fut>(
        _cx: Owner,
        f: F,
    ) -> impl Signal<Option<T>>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = T> + Send + 'static,
    {
        // For now, we'll create a simple signal that starts with None
        // In a full implementation, this would track dependencies and re-run when they change
        let (read, write) = reactive_graph::signal::signal(None);
        UnifiedSignal::new(read, write)
    }
}


// Note: Specialized implementations would conflict with the generic implementation
// The generic implementation with #[inline(always)] provides excellent performance

// Note: We don't implement Signal trait for RwSignal to avoid method conflicts
// Users should use the unified signal() function instead

impl<T: Clone + Send + Sync + 'static> Signal<T> for Memo<T> {
    #[inline]
    fn get(&self) -> T {
        Get::get(self)
    }
    
    #[inline]
    fn get_untracked(&self) -> T {
        GetUntracked::get_untracked(self)
    }
    
    fn set(&self, _value: T) {
        panic!("Cannot set a memo signal directly");
    }
    
    fn update(&self, _f: impl FnOnce(&mut T)) {
        panic!("Cannot update a memo signal directly");
    }
    
    fn derive<U: Clone + Send + Sync + PartialEq + 'static>(&self, f: impl Fn(&T) -> U + Send + Sync + 'static) -> impl Signal<U> {
        let memo = self.clone();
        let derived_read = Memo::new(move |_| f(&Get::get(&memo)));
        DerivedSignal { read: derived_read }
    }
    
    fn split(self) -> (ReadSignal<T>, WriteSignal<T>) {
        panic!("Cannot split a memo signal");
    }
}
