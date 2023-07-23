#![forbid(unsafe_code)]
use crate::{
    console_warn, create_effect, diagnostics,
    diagnostics::*,
    macros::debug_warn,
    node::NodeId,
    on_cleanup,
    runtime::{with_runtime, RuntimeId},
    Runtime, Scope, ScopeProperty,
};
use futures::Stream;
use std::{
    any::Any,
    cell::RefCell,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    pin::Pin,
    rc::Rc,
};
use thiserror::Error;

macro_rules! impl_get_fn_traits {
    ($($ty:ident $(($method_name:ident))?),*) => {
        $(
            #[cfg(feature = "nightly")]
            impl<T: Clone> FnOnce<()> for $ty<T> {
                type Output = T;

                #[inline(always)]
                extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
                    impl_get_fn_traits!(@method_name self $($method_name)?)
                }
            }

            #[cfg(feature = "nightly")]
            impl<T: Clone> FnMut<()> for $ty<T> {
                #[inline(always)]
                extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
                    impl_get_fn_traits!(@method_name self $($method_name)?)
                }
            }

            #[cfg(feature = "nightly")]
            impl<T: Clone> Fn<()> for $ty<T> {
                #[inline(always)]
                extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
                    impl_get_fn_traits!(@method_name self $($method_name)?)
                }
            }
        )*
    };
    (@method_name $self:ident) => {
        $self.get()
    };
    (@method_name $self:ident $ident:ident) => {
        $self.$ident()
    };
}

macro_rules! impl_set_fn_traits {
    ($($ty:ident $($method_name:ident)?),*) => {
        $(
            #[cfg(feature = "nightly")]
            impl<T> FnOnce<(T,)> for $ty<T> {
                type Output = ();

                #[inline(always)]
                extern "rust-call" fn call_once(self, args: (T,)) -> Self::Output {
                    impl_set_fn_traits!(@method_name self $($method_name)? args)
                }
            }

            #[cfg(feature = "nightly")]
            impl<T> FnMut<(T,)> for $ty<T> {
                #[inline(always)]
                extern "rust-call" fn call_mut(&mut self, args: (T,)) -> Self::Output {
                    impl_set_fn_traits!(@method_name self $($method_name)? args)
                }
            }

            #[cfg(feature = "nightly")]
            impl<T> Fn<(T,)> for $ty<T> {
                #[inline(always)]
                extern "rust-call" fn call(&self, args: (T,)) -> Self::Output {
                    impl_set_fn_traits!(@method_name self $($method_name)? args)
                }
            }
        )*
    };
    (@method_name $self:ident $args:ident) => {
        $self.set($args.0)
    };
    (@method_name $self:ident $ident:ident $args:ident) => {
        $self.$ident($args.0)
    };
}

impl_get_fn_traits![ReadSignal, RwSignal];
impl_set_fn_traits![WriteSignal];

/// This prelude imports all signal types as well as all signal
/// traits needed to use those types.
pub mod prelude {
    pub use super::*;
    pub use crate::{
        memo::*, selector::*, signal_wrappers_read::*, signal_wrappers_write::*,
    };
}

/// This trait allows getting an owned value of the signals
/// inner type.
pub trait SignalGet<T> {
    /// Clones and returns the current value of the signal, and subscribes
    /// the running effect to this signal.
    ///
    /// # Panics
    /// Panics if you try to access a signal that was created in a [`Scope`] that has been disposed.
    #[track_caller]
    fn get(&self) -> T;

    /// Clones and returns the signal value, returning [`Some`] if the signal
    /// is still alive, and [`None`] otherwise.
    fn try_get(&self) -> Option<T>;
}

/// This trait allows obtaining an immutable reference to the signal's
/// inner type.
pub trait SignalWith<T> {
    /// Applies a function to the current value of the signal, and subscribes
    /// the running effect to this signal.
    ///
    /// # Panics
    /// Panics if you try to access a signal that was created in a [`Scope`] that has been disposed.
    #[track_caller]
    fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O;

    /// Applies a function to the current value of the signal, and subscribes
    /// the running effect to this signal. Returns [`Some`] if the signal is
    /// valid and the function ran, otherwise returns [`None`].
    fn try_with<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O>;

    /// Subscribes to this signal in the current reactive scope without doing anything with its value.
    fn track(&self) {
        _ = self.try_with(|_| {});
    }
}

/// This trait allows setting the value of a signal.
pub trait SignalSet<T> {
    /// Sets the signal’s value and notifies subscribers.
    ///
    /// **Note:** `set()` does not auto-memoize, i.e., it will notify subscribers
    /// even if the value has not actually changed.
    #[track_caller]
    fn set(&self, new_value: T);

    /// Sets the signal’s value and notifies subscribers. Returns [`None`]
    /// if the signal is still valid, [`Some(T)`] otherwise.
    ///
    /// **Note:** `set()` does not auto-memoize, i.e., it will notify subscribers
    /// even if the value has not actually changed.
    fn try_set(&self, new_value: T) -> Option<T>;
}

/// This trait allows updating the inner value of a signal.
pub trait SignalUpdate<T> {
    /// Applies a function to the current value to mutate it in place
    /// and notifies subscribers that the signal has changed.
    ///
    /// **Note:** `update()` does not auto-memoize, i.e., it will notify subscribers
    /// even if the value has not actually changed.
    #[track_caller]
    fn update(&self, f: impl FnOnce(&mut T));

    /// Applies a function to the current value to mutate it in place
    /// and notifies subscribers that the signal has changed. Returns
    /// [`Some(O)`] if the signal is still valid, [`None`] otherwise.
    ///
    /// **Note:** `update()` does not auto-memoize, i.e., it will notify subscribers
    /// even if the value has not actually changed.
    fn try_update<O>(&self, f: impl FnOnce(&mut T) -> O) -> Option<O>;
}

/// Trait implemented for all signal types which you can `get` a value
/// from, such as [`ReadSignal`],
/// [`Memo`](crate::Memo), etc., which allows getting the inner value without
/// subscribing to the current scope.
pub trait SignalGetUntracked<T> {
    /// Gets the signal's value without creating a dependency on the
    /// current scope.
    ///
    /// # Panics
    /// Panics if you try to access a signal that was created in a [`Scope`] that has been disposed.
    #[track_caller]
    fn get_untracked(&self) -> T;

    /// Gets the signal's value without creating a dependency on the
    /// current scope. Returns [`Some(T)`] if the signal is still
    /// valid, [`None`] otherwise.
    fn try_get_untracked(&self) -> Option<T>;
}

/// This trait allows getting a reference to the signals inner value
/// without creating a dependency on the signal.
pub trait SignalWithUntracked<T> {
    /// Runs the provided closure with a reference to the current
    /// value without creating a dependency on the current scope.
    ///
    /// # Panics
    /// Panics if you try to access a signal that was created in a [`Scope`] that has been disposed.
    #[track_caller]
    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O;

    /// Runs the provided closure with a reference to the current
    /// value without creating a dependency on the current scope.
    /// Returns [`Some(O)`] if the signal is still valid, [`None`]
    /// otherwise.
    #[track_caller]
    fn try_with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O>;
}

/// Trait implemented for all signal types which you can `set` the inner
/// value, such as [`WriteSignal`] and [`RwSignal`], which allows setting
/// the inner value without causing effects which depend on the signal
/// from being run.
pub trait SignalSetUntracked<T> {
    /// Sets the signal's value without notifying dependents.
    #[track_caller]
    fn set_untracked(&self, new_value: T);

    /// Attempts to set the signal if it's still valid. Returns [`None`]
    /// if the signal was set, [`Some(T)`] otherwise.
    #[track_caller]
    fn try_set_untracked(&self, new_value: T) -> Option<T>;
}

/// This trait allows updating the signals value without causing
/// dependant effects to run.
pub trait SignalUpdateUntracked<T> {
    /// Runs the provided closure with a mutable reference to the current
    /// value without notifying dependents.
    #[track_caller]
    fn update_untracked(&self, f: impl FnOnce(&mut T));

    /// Runs the provided closure with a mutable reference to the current
    /// value without notifying dependents and returns
    /// the value the closure returned.
    fn try_update_untracked<O>(&self, f: impl FnOnce(&mut T) -> O)
        -> Option<O>;
}

/// This trait allows converting a signal into a async [`Stream`].
pub trait SignalStream<T> {
    /// Generates a [`Stream`] that emits the new value of the signal
    /// whenever it changes.
    ///
    /// # Panics
    /// Panics if you try to access a signal that was created in a [`Scope`] that has been disposed.
    // We're returning an opaque type until impl trait in trait
    // positions are stabilized, and also so any underlying
    // changes are non-breaking
    #[track_caller]
    fn to_stream(&self, cx: Scope) -> Pin<Box<dyn Stream<Item = T>>>;
}

/// This trait allows disposing a signal before its [`Scope`] has been disposed.
pub trait SignalDispose {
    /// Disposes of the signal. This:
    /// 1. Detaches the signal from the reactive graph, preventing it from triggering
    ///    further updates; and
    /// 2. Drops the value contained in the signal.
    #[track_caller]
    fn dispose(self);
}

/// Creates a signal, the basic reactive primitive.
///
/// A signal is a piece of data that may change over time,
/// and notifies other code when it has changed. This is the
/// core primitive of Leptos’s reactive system.
///
/// Takes a reactive [`Scope`] and the initial value as arguments,
/// and returns a tuple containing a [`ReadSignal`] and a [`WriteSignal`],
/// each of which can be called as a function.
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (count, set_count) = create_signal(cx, 0);
///
/// // ✅ calling the getter clones and returns the value
/// //    this can be `count()` on nightly
/// assert_eq!(count.get(), 0);
///
/// // ✅ calling the setter sets the value
/// //    this can be `set_count(1)` on nightly
/// set_count.set(1);
/// assert_eq!(count.get(), 1);
///
/// // ❌ you could call the getter within the setter
/// // set_count.set(count.get() + 1);
///
/// // ✅ however it's more efficient to use .update() and mutate the value in place
/// set_count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count.get(), 2);
///
/// // ✅ you can create "derived signals" with a Fn() -> T interface
/// let double_count = move || count.get() * 2; // signals are `Copy` so you can `move` them anywhere
/// set_count.set(0);
/// assert_eq!(double_count(), 0);
/// set_count.set(1);
/// assert_eq!(double_count(), 2);
/// # }).dispose();
/// #
/// ```
#[cfg_attr(
 any(debug_assertions, features="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            scope = ?cx.id,
            ty = %std::any::type_name::<T>()
        )
    )
)]
#[track_caller]
pub fn create_signal<T>(
    cx: Scope,
    value: T,
) -> (ReadSignal<T>, WriteSignal<T>) {
    let s = cx.runtime.create_signal(value);
    cx.push_scope_property(ScopeProperty::Signal(s.0.id));
    s
}

/// Works exactly as [`create_signal`], but creates multiple signals at once.
#[cfg_attr(
 any(debug_assertions, features="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            scope = ?cx.id,
            ty = %std::any::type_name::<T>()
        )
    )
)]
#[track_caller]
pub fn create_many_signals<T>(
    cx: Scope,
    values: impl IntoIterator<Item = T>,
) -> Vec<(ReadSignal<T>, WriteSignal<T>)> {
    cx.runtime.create_many_signals_with_map(cx, values, |x| x)
}

/// Works exactly as [`create_many_signals`], but applies the map function to each signal pair.
#[cfg_attr(
 any(debug_assertions, features="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            scope = ?cx.id,
            ty = %std::any::type_name::<T>()
        )
    )
)]
#[track_caller]
pub fn create_many_signals_mapped<T, U>(
    cx: Scope,
    values: impl IntoIterator<Item = T>,
    map_fn: impl Fn((ReadSignal<T>, WriteSignal<T>)) -> U + 'static,
) -> Vec<U>
where
    T: 'static,
{
    cx.runtime.create_many_signals_with_map(cx, values, map_fn)
}

/// Creates a signal that always contains the most recent value emitted by a
/// [`Stream`](futures::stream::Stream).
/// If the stream has not yet emitted a value since the signal was created, the signal's
/// value will be `None`.
///
/// **Note**: If used on the server side during server rendering, this will return `None`
/// immediately and not begin driving the stream.
#[cfg_attr(
 any(debug_assertions, features="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            scope = ?cx.id,
        )
    )
)]
pub fn create_signal_from_stream<T>(
    cx: Scope,
    #[allow(unused_mut)] // allowed because needed for SSR
    mut stream: impl Stream<Item = T> + Unpin + 'static,
) -> ReadSignal<Option<T>> {
    cfg_if::cfg_if! {
        if #[cfg(feature = "ssr")] {
            _ = stream;
            let (read, _) = create_signal(cx, None);
            read
        } else {
            use crate::spawn_local;
            use futures::StreamExt;

            let (read, write) = create_signal(cx, None);
            spawn_local(async move {
                while let Some(value) = stream.next().await {
                    write.set(Some(value));
                }
            });
            read
        }
    }
}

/// The getter for a reactive signal.
///
/// A signal is a piece of data that may change over time,
/// and notifies other code when it has changed. This is the
/// core primitive of Leptos’s reactive system.
///
/// `ReadSignal` is also [`Copy`] and `'static`, so it can very easily moved into closures
/// or copied structs.
///
/// ## Core Trait Implementations
/// - [`.get()`](#impl-SignalGet<T>-for-ReadSignal<T>) (or calling the signal as a function) clones the current
///   value of the signal. If you call it within an effect, it will cause that effect
///   to subscribe to the signal, and to re-run whenever the value of the signal changes.
///   - [`.get_untracked()`](#impl-SignalGetUntracked<T>-for-ReadSignal<T>) clones the value of the signal
///   without reactively tracking it.
/// - [`.with()`](#impl-SignalWith<T>-for-ReadSignal<T>) allows you to reactively access the signal’s value without
///   cloning by applying a callback function.
///   - [`.with_untracked()`](#impl-SignalWithUntracked<T>-for-ReadSignal<T>) allows you to access the signal’s
///   value without reactively tracking it.
/// - [`.to_stream()`](#impl-SignalStream<T>-for-ReadSignal<T>) converts the signal to an `async` stream of values.
///
/// # Examples
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (count, set_count) = create_signal(cx, 0);
///
/// // ✅ calling the getter clones and returns the value
/// assert_eq!(count.get(), 0);
///
/// // ✅ calling the setter sets the value
/// set_count.set(1); // `set_count(1)` on nightly
/// assert_eq!(count.get(), 1);
///
/// // ❌ you could call the getter within the setter
/// // set_count.set(count.get() + 1);
///
/// // ✅ however it's more efficient to use .update() and mutate the value in place
/// set_count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count.get(), 2);
///
/// // ✅ you can create "derived signals" with the same Fn() -> T interface
/// let double_count = move || count.get() * 2; // signals are `Copy` so you can `move` them anywhere
/// set_count.set(0);
/// assert_eq!(double_count(), 0);
/// set_count.set(1);
/// assert_eq!(double_count(), 2);
/// # }).dispose();
/// #
/// ```
pub struct ReadSignal<T>
where
    T: 'static,
{
    pub(crate) runtime: RuntimeId,
    pub(crate) id: NodeId,
    pub(crate) ty: PhantomData<T>,
    #[cfg(any(debug_assertions, feature = "ssr"))]
    pub(crate) defined_at: &'static std::panic::Location<'static>,
}

impl<T: Clone> SignalGetUntracked<T> for ReadSignal<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "ReadSignal::get_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn get_untracked(&self) -> T {
        match with_runtime(self.runtime, |runtime| {
            self.id.try_with_no_subscription(runtime, T::clone)
        })
        .expect("runtime to be alive")
        {
            Ok(t) => t,
            Err(_) => panic_getting_dead_signal(
                #[cfg(any(debug_assertions, feature = "ssr"))]
                self.defined_at,
            ),
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "ReadSignal::try_get_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    fn try_get_untracked(&self) -> Option<T> {
        with_runtime(self.runtime, |runtime| {
            self.id.try_with_no_subscription(runtime, Clone::clone).ok()
        })
        .ok()
        .flatten()
    }
}

impl<T> SignalWithUntracked<T> for ReadSignal<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "ReadSignal::with_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[inline(always)]
    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        self.with_no_subscription(f)
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "ReadSignal::try_with_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    #[inline(always)]
    fn try_with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        let diagnostics = diagnostics!(self);

        match with_runtime(self.runtime, |runtime| {
            self.id.try_with(runtime, f, diagnostics)
        }) {
            Ok(Ok(o)) => Some(o),
            _ => None,
        }
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (name, set_name) = create_signal(cx, "Alice".to_string());
///
/// // ❌ unnecessarily clones the string
/// let first_char = move || name.get().chars().next().unwrap();
/// assert_eq!(first_char(), 'A');
///
/// // ✅ gets the first char without cloning the `String`
/// let first_char = move || name.with(|n| n.chars().next().unwrap());
/// assert_eq!(first_char(), 'A');
/// set_name.set("Bob".to_string());
/// assert_eq!(first_char(), 'B');
/// # });
/// ```
impl<T> SignalWith<T> for ReadSignal<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "ReadSignal::with()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    #[inline(always)]
    fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        let diagnostics = diagnostics!(self);

        match with_runtime(self.runtime, |runtime| {
            self.id.try_with(runtime, f, diagnostics)
        })
        .expect("runtime to be alive")
        {
            Ok(o) => o,
            Err(_) => panic_getting_dead_signal(
                #[cfg(any(debug_assertions, feature = "ssr"))]
                self.defined_at,
            ),
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "ReadSignal::try_with()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    #[inline(always)]
    fn try_with<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        let diagnostics = diagnostics!(self);

        with_runtime(self.runtime, |runtime| {
            self.id.try_with(runtime, f, diagnostics).ok()
        })
        .ok()
        .flatten()
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (count, set_count) = create_signal(cx, 0);
///
/// assert_eq!(count.get(), 0);
///
/// // count() is shorthand for count.get() on `nightly`
/// // assert_eq!(count.get(), 0);
/// # });
/// ```
impl<T: Clone> SignalGet<T> for ReadSignal<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "ReadSignal::get()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    fn get(&self) -> T {
        let diagnostics = diagnostics!(self);

        match with_runtime(self.runtime, |runtime| {
            self.id.try_with(runtime, T::clone, diagnostics)
        })
        .expect("runtime to be alive")
        {
            Ok(t) => t,
            Err(_) => panic_getting_dead_signal(
                #[cfg(any(debug_assertions, feature = "ssr"))]
                self.defined_at,
            ),
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "ReadSignal::try_get()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn try_get(&self) -> Option<T> {
        self.try_with(Clone::clone).ok()
    }
}

impl<T: Clone> SignalStream<T> for ReadSignal<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "ReadSignal::to_stream()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn to_stream(&self, cx: Scope) -> Pin<Box<dyn Stream<Item = T>>> {
        let (tx, rx) = futures::channel::mpsc::unbounded();

        let close_channel = tx.clone();

        on_cleanup(cx, move || close_channel.close_channel());

        let this = *self;

        create_effect(cx, move |_| {
            let _ = tx.unbounded_send(this.get());
        });

        Box::pin(rx)
    }
}

impl<T> SignalDispose for ReadSignal<T> {
    fn dispose(self) {
        _ = with_runtime(self.runtime, |runtime| runtime.dispose_node(self.id));
    }
}

impl<T> ReadSignal<T>
where
    T: 'static,
{
    #[inline(always)]
    pub(crate) fn with_no_subscription<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        self.id.with_no_subscription(self.runtime, f)
    }

    /// Applies the function to the current Signal, if it exists, and subscribes
    /// the running effect.
    #[track_caller]
    #[inline(always)]
    pub(crate) fn try_with<U>(
        &self,
        f: impl FnOnce(&T) -> U,
    ) -> Result<U, SignalError> {
        let diagnostics = diagnostics!(self);

        match with_runtime(self.runtime, |runtime| {
            self.id.try_with(runtime, f, diagnostics)
        }) {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(SignalError::RuntimeDisposed),
        }
    }
}

impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ReadSignal<T> {}

impl<T> fmt::Debug for ReadSignal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("ReadSignal");
        s.field("runtime", &self.runtime);
        s.field("id", &self.id);
        s.field("ty", &self.ty);
        #[cfg(any(debug_assertions, feature = "ssr"))]
        s.field("defined_at", &self.defined_at);
        s.finish()
    }
}

impl<T> Eq for ReadSignal<T> {}

impl<T> PartialEq for ReadSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.runtime == other.runtime && self.id == other.id
    }
}

impl<T> Hash for ReadSignal<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.runtime.hash(state);
        self.id.hash(state);
    }
}

/// The setter for a reactive signal.
///
/// A signal is a piece of data that may change over time,
/// and notifies other code when it has changed. This is the
/// core primitive of Leptos’s reactive system.
///
/// Calling [`WriteSignal::update`] will mutate the signal’s value in place,
/// and notify all subscribers that the signal’s value has changed.
///
/// `WriteSignal` implements [`Fn`], such that `set_value(new_value)` is equivalent to
/// `set_value.update(|value| *value = new_value)`.
///
/// `WriteSignal` is [`Copy`] and `'static`, so it can very easily moved into closures
/// or copied structs.
///
/// ## Core Trait Implementations
/// - [`.set()`](#impl-SignalSet<T>-for-WriteSignal<T>) (or calling the setter as a function)
///   sets the signal’s value, and notifies all subscribers that the signal’s value has changed.
///   to subscribe to the signal, and to re-run whenever the value of the signal changes.
///   - [`.set_untracked()`](#impl-SignalSetUntracked<T>-for-WriteSignal<T>) sets the signal’s value
///   without notifying its subscribers.
/// - [`.update()`](#impl-SignalUpdate<T>-for-WriteSignal<T>) mutates the signal’s value in place
///   and notifies all subscribers that the signal’s value has changed.
///   - [`.update_untracked()`](#impl-SignalUpdateUntracked<T>-for-WriteSignal<T>) mutates the signal’s value
///   in place without notifying its subscribers.
///
/// ## Examples
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (count, set_count) = create_signal(cx, 0);
///
/// // ✅ calling the setter sets the value
/// //    `set_count(1)` on nightly
/// set_count.set(1);
/// assert_eq!(count.get(), 1);
///
/// // ❌ you could call the getter within the setter
/// // set_count.set(count.get() + 1);
///
/// // ✅ however it's more efficient to use .update() and mutate the value in place
/// set_count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count.get(), 2);
/// # }).dispose();
/// #
/// ```
pub struct WriteSignal<T>
where
    T: 'static,
{
    pub(crate) runtime: RuntimeId,
    pub(crate) id: NodeId,
    pub(crate) ty: PhantomData<T>,
    #[cfg(any(debug_assertions, feature = "ssr"))]
    pub(crate) defined_at: &'static std::panic::Location<'static>,
}

impl<T> SignalSetUntracked<T> for WriteSignal<T>
where
    T: 'static,
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "WriteSignal::set_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn set_untracked(&self, new_value: T) {
        self.id.update_with_no_effect(
            self.runtime,
            |v| *v = new_value,
            #[cfg(debug_assertions)]
            Some(self.defined_at),
        );
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "WriteSignal::try_set_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn try_set_untracked(&self, new_value: T) -> Option<T> {
        let mut new_value = Some(new_value);

        self.id.update(
            self.runtime,
            |t| *t = new_value.take().unwrap(),
            #[cfg(debug_assertions)]
            None,
        );

        new_value
    }
}

impl<T> SignalUpdateUntracked<T> for WriteSignal<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "WriteSignal::updated_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[inline(always)]
    fn update_untracked(&self, f: impl FnOnce(&mut T)) {
        self.id.update_with_no_effect(
            self.runtime,
            f,
            #[cfg(debug_assertions)]
            Some(self.defined_at),
        );
    }

    #[inline(always)]
    fn try_update_untracked<O>(
        &self,
        f: impl FnOnce(&mut T) -> O,
    ) -> Option<O> {
        self.id.update_with_no_effect(
            self.runtime,
            f,
            #[cfg(debug_assertions)]
            None,
        )
    }
}

/// # Examples
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (count, set_count) = create_signal(cx, 0);
///
/// // notifies subscribers
/// set_count.update(|n| *n = 1); // it's easier just to call set_count.set(1), though!
/// assert_eq!(count.get(), 1);
///
/// // you can include arbitrary logic in this update function
/// // also notifies subscribers, even though the value hasn't changed
/// set_count.update(|n| if *n > 3 { *n += 1 });
/// assert_eq!(count.get(), 1);
/// # }).dispose();
/// ```
impl<T> SignalUpdate<T> for WriteSignal<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            name = "WriteSignal::update()",
            level = "trace",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[inline(always)]
    fn update(&self, f: impl FnOnce(&mut T)) {
        if self
            .id
            .update(
                self.runtime,
                f,
                #[cfg(debug_assertions)]
                Some(self.defined_at),
            )
            .is_none()
        {
            warn_updating_dead_signal(
                #[cfg(any(debug_assertions, feature = "ssr"))]
                self.defined_at,
            );
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            name = "WriteSignal::try_update()",
            level = "trace",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[inline(always)]
    fn try_update<O>(&self, f: impl FnOnce(&mut T) -> O) -> Option<O> {
        self.id.update(
            self.runtime,
            f,
            #[cfg(debug_assertions)]
            None,
        )
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (count, set_count) = create_signal(cx, 0);
///
/// // notifies subscribers
/// set_count.update(|n| *n = 1); // it's easier just to call set_count.set(1), though!
/// assert_eq!(count.get(), 1);
///
/// // you can include arbitrary logic in this update function
/// // also notifies subscribers, even though the value hasn't changed
/// set_count.update(|n| if *n > 3 { *n += 1 });
/// assert_eq!(count.get(), 1);
/// # }).dispose();
/// ```
impl<T> SignalSet<T> for WriteSignal<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "WriteSignal::set()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn set(&self, new_value: T) {
        self.id.update(
            self.runtime,
            |n| *n = new_value,
            #[cfg(debug_assertions)]
            Some(self.defined_at),
        );
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "WriteSignal::try_set()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn try_set(&self, new_value: T) -> Option<T> {
        let mut new_value = Some(new_value);

        self.id.update(
            self.runtime,
            |t| *t = new_value.take().unwrap(),
            #[cfg(debug_assertions)]
            None,
        );

        new_value
    }
}

impl<T> SignalDispose for WriteSignal<T> {
    fn dispose(self) {
        _ = with_runtime(self.runtime, |runtime| runtime.dispose_node(self.id));
    }
}

impl<T> Clone for WriteSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for WriteSignal<T> {}

impl<T> fmt::Debug for WriteSignal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("WriteSignal");
        s.field("runtime", &self.runtime);
        s.field("id", &self.id);
        s.field("ty", &self.ty);
        #[cfg(any(debug_assertions, feature = "ssr"))]
        s.field("defined_at", &self.defined_at);
        s.finish()
    }
}

impl<T> Eq for WriteSignal<T> {}

impl<T> PartialEq for WriteSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.runtime == other.runtime && self.id == other.id
    }
}

impl<T> Hash for WriteSignal<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.runtime.hash(state);
        self.id.hash(state);
    }
}

/// Creates a reactive signal with the getter and setter unified in one value.
/// You may prefer this style, or it may be easier to pass around in a context
/// or as a function argument.
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let count = create_rw_signal(cx, 0);
///
/// // ✅ set the value
/// count.set(1);
/// assert_eq!(count.get(), 1);
///
/// // ❌ you can call the getter within the setter
/// // count.set(count.get() + 1);
///
/// // ✅ however, it's more efficient to use .update() and mutate the value in place
/// count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count.get(), 2);
/// # }).dispose();
/// #
/// ```
#[cfg_attr(
 any(debug_assertions, features="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            ty = %std::any::type_name::<T>()
        )
    )
)]
#[track_caller]
pub fn create_rw_signal<T>(cx: Scope, value: T) -> RwSignal<T> {
    let s = cx.runtime.create_rw_signal(value);
    cx.push_scope_property(ScopeProperty::Signal(s.id));
    s
}

/// A signal that combines the getter and setter into one value, rather than
/// separating them into a [`ReadSignal`] and a [`WriteSignal`]. You may prefer this
/// its style, or it may be easier to pass around in a context or as a function argument.
///
/// ## Core Trait Implementations
/// - [`.get()`](#impl-SignalGet<T>-for-RwSignal<T>) clones the current
///   value of the signal. If you call it within an effect, it will cause that effect
///   to subscribe to the signal, and to re-run whenever the value of the signal changes.
///   - [`.get_untracked()`](#impl-SignalGetUntracked<T>-for-RwSignal<T>) clones the value of the signal
///   without reactively tracking it.
/// - [`.with()`](#impl-SignalWith<T>-for-RwSignal<T>) allows you to reactively access the signal’s value without
///   cloning by applying a callback function.
///   - [`.with_untracked()`](#impl-SignalWithUntracked<T>-for-RwSignal<T>) allows you to access the signal’s
///   value without reactively tracking it.
/// - [`.set()`](#impl-SignalSet<T>-for-RwSignal<T>) sets the signal’s value,
///   and notifies all subscribers that the signal’s value has changed.
///   to subscribe to the signal, and to re-run whenever the value of the signal changes.
///   - [`.set_untracked()`](#impl-SignalSetUntracked<T>-for-RwSignal<T>) sets the signal’s value
///   without notifying its subscribers.
/// - [`.update()`](#impl-SignalUpdate<T>-for-RwSignal<T>) mutates the signal’s value in place
///   and notifies all subscribers that the signal’s value has changed.
///   - [`.update_untracked()`](#impl-SignalUpdateUntracked<T>-for-RwSignal<T>) mutates the signal’s value
///   in place without notifying its subscribers.
/// - [`.to_stream()`](#impl-SignalStream<T>-for-RwSignal<T>) converts the signal to an `async` stream of values.
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let count = create_rw_signal(cx, 0);
///
/// // ✅ set the value
/// count.set(1);
/// assert_eq!(count.get(), 1);
///
/// // ❌ you can call the getter within the setter
/// // count.set(count.get() + 1);
///
/// // ✅ however, it's more efficient to use .update() and mutate the value in place
/// count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count.get(), 2);
/// # }).dispose();
/// #
/// ```
pub struct RwSignal<T>
where
    T: 'static,
{
    pub(crate) runtime: RuntimeId,
    pub(crate) id: NodeId,
    pub(crate) ty: PhantomData<T>,
    #[cfg(any(debug_assertions, feature = "ssr"))]
    pub(crate) defined_at: &'static std::panic::Location<'static>,
}

impl<T> Clone for RwSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for RwSignal<T> {}

impl<T> fmt::Debug for RwSignal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("RwSignal");
        s.field("runtime", &self.runtime);
        s.field("id", &self.id);
        s.field("ty", &self.ty);
        #[cfg(any(debug_assertions, feature = "ssr"))]
        s.field("defined_at", &self.defined_at);
        s.finish()
    }
}

impl<T> Eq for RwSignal<T> {}

impl<T> PartialEq for RwSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.runtime == other.runtime && self.id == other.id
    }
}

impl<T> Hash for RwSignal<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.runtime.hash(state);
        self.id.hash(state);
    }
}

impl<T: Clone> SignalGetUntracked<T> for RwSignal<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::get_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn get_untracked(&self) -> T {
        self.id.with_no_subscription(self.runtime, Clone::clone)
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::try_get_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn try_get_untracked(&self) -> Option<T> {
        match with_runtime(self.runtime, |runtime| {
            self.id.try_with_no_subscription(runtime, Clone::clone)
        })
        .expect("runtime to be alive")
        {
            Ok(t) => t,
            Err(_) => panic_getting_dead_signal(
                #[cfg(any(debug_assertions, feature = "ssr"))]
                self.defined_at,
            ),
        }
    }
}

impl<T> SignalWithUntracked<T> for RwSignal<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::with_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[inline(always)]
    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        self.id.with_no_subscription(self.runtime, f)
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::try_with_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    #[inline(always)]
    fn try_with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        let diagnostics = diagnostics!(self);

        match with_runtime(self.runtime, |runtime| {
            self.id.try_with(runtime, f, diagnostics)
        }) {
            Ok(Ok(o)) => Some(o),
            _ => None,
        }
    }
}

impl<T> SignalSetUntracked<T> for RwSignal<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::set_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn set_untracked(&self, new_value: T) {
        self.id.update_with_no_effect(
            self.runtime,
            |v| *v = new_value,
            #[cfg(debug_assertions)]
            Some(self.defined_at),
        );
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::try_set_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn try_set_untracked(&self, new_value: T) -> Option<T> {
        let mut new_value = Some(new_value);

        self.id.update(
            self.runtime,
            |t| *t = new_value.take().unwrap(),
            #[cfg(debug_assertions)]
            None,
        );

        new_value
    }
}

impl<T> SignalUpdateUntracked<T> for RwSignal<T> {
    #[cfg_attr(
 any(debug_assertions, features="ssr"),
    instrument(
        level = "trace",
        name = "RwSignal::update_untracked()",
        skip_all,
        fields(
            id = ?self.id,
            defined_at = %self.defined_at,
            ty = %std::any::type_name::<T>()
        )
    )
    )]
    #[inline(always)]
    fn update_untracked(&self, f: impl FnOnce(&mut T)) {
        self.id.update_with_no_effect(
            self.runtime,
            f,
            #[cfg(debug_assertions)]
            Some(self.defined_at),
        );
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::try_update_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[inline(always)]
    fn try_update_untracked<O>(
        &self,
        f: impl FnOnce(&mut T) -> O,
    ) -> Option<O> {
        self.id.update_with_no_effect(
            self.runtime,
            f,
            #[cfg(debug_assertions)]
            None,
        )
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let name = create_rw_signal(cx, "Alice".to_string());
///
/// // ❌ unnecessarily clones the string
/// let first_char = move || name.get().chars().next().unwrap();
/// assert_eq!(first_char(), 'A');
///
/// // ✅ gets the first char without cloning the `String`
/// let first_char = move || name.with(|n| n.chars().next().unwrap());
/// assert_eq!(first_char(), 'A');
/// name.set("Bob".to_string());
/// assert_eq!(first_char(), 'B');
/// # }).dispose();
/// #
/// ```
impl<T> SignalWith<T> for RwSignal<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::with()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    #[inline(always)]
    fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        let diagnostics = diagnostics!(self);

        match with_runtime(self.runtime, |runtime| {
            self.id.try_with(runtime, f, diagnostics)
        })
        .expect("runtime to be alive")
        {
            Ok(o) => o,
            Err(_) => panic_getting_dead_signal(
                #[cfg(any(debug_assertions, feature = "ssr"))]
                self.defined_at,
            ),
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::try_with()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    #[inline(always)]
    fn try_with<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        let diagnostics = diagnostics!(self);

        with_runtime(self.runtime, |runtime| {
            self.id.try_with(runtime, f, diagnostics).ok()
        })
        .ok()
        .flatten()
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let count = create_rw_signal(cx, 0);
///
/// assert_eq!(count.get(), 0);
///
/// // count() is shorthand for count.get() on `nightly`
/// // assert_eq!(count(), 0);
/// # }).dispose();
/// #
/// ```
impl<T: Clone> SignalGet<T> for RwSignal<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::get()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    fn get(&self) -> T
    where
        T: Clone,
    {
        let diagnostics = diagnostics!(self);

        match with_runtime(self.runtime, |runtime| {
            self.id.try_with(runtime, T::clone, diagnostics)
        })
        .expect("runtime to be alive")
        {
            Ok(t) => t,
            Err(_) => panic_getting_dead_signal(
                #[cfg(any(debug_assertions, feature = "ssr"))]
                self.defined_at,
            ),
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::try_get()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    fn try_get(&self) -> Option<T> {
        let diagnostics = diagnostics!(self);

        with_runtime(self.runtime, |runtime| {
            self.id.try_with(runtime, Clone::clone, diagnostics).ok()
        })
        .ok()
        .flatten()
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let count = create_rw_signal(cx, 0);
///
/// // notifies subscribers
/// count.update(|n| *n = 1); // it's easier just to call set_count.set(1), though!
/// assert_eq!(count.get(), 1);
///
/// // you can include arbitrary logic in this update function
/// // also notifies subscribers, even though the value hasn't changed
/// count.update(|n| {
///     if *n > 3 {
///         *n += 1
///     }
/// });
/// assert_eq!(count.get(), 1);
/// # }).dispose();
/// ```
impl<T> SignalUpdate<T> for RwSignal<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::update()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[inline(always)]
    fn update(&self, f: impl FnOnce(&mut T)) {
        if self
            .id
            .update(
                self.runtime,
                f,
                #[cfg(debug_assertions)]
                Some(self.defined_at),
            )
            .is_none()
        {
            warn_updating_dead_signal(
                #[cfg(any(debug_assertions, feature = "ssr"))]
                self.defined_at,
            );
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::try_update()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[inline(always)]
    fn try_update<O>(&self, f: impl FnOnce(&mut T) -> O) -> Option<O> {
        self.id.update(
            self.runtime,
            f,
            #[cfg(debug_assertions)]
            None,
        )
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let count = create_rw_signal(cx, 0);
///
/// assert_eq!(count.get(), 0);
/// count.set(1);
/// assert_eq!(count.get(), 1);
/// # }).dispose();
/// ```
impl<T> SignalSet<T> for RwSignal<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::set()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn set(&self, value: T) {
        self.id.update(
            self.runtime,
            |n| *n = value,
            #[cfg(debug_assertions)]
            Some(self.defined_at),
        );
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::try_set()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn try_set(&self, new_value: T) -> Option<T> {
        let mut new_value = Some(new_value);

        self.id.update(
            self.runtime,
            |t| *t = new_value.take().unwrap(),
            #[cfg(debug_assertions)]
            None,
        );

        new_value
    }
}

impl<T: Clone> SignalStream<T> for RwSignal<T> {
    fn to_stream(&self, cx: Scope) -> Pin<Box<dyn Stream<Item = T>>> {
        let (tx, rx) = futures::channel::mpsc::unbounded();

        let close_channel = tx.clone();

        on_cleanup(cx, move || close_channel.close_channel());

        let this = *self;

        create_effect(cx, move |_| {
            let _ = tx.unbounded_send(this.get());
        });

        Box::pin(rx)
    }
}

impl<T> SignalDispose for RwSignal<T> {
    fn dispose(self) {
        _ = with_runtime(self.runtime, |runtime| runtime.dispose_node(self.id));
    }
}

impl<T> RwSignal<T> {
    /// Returns a read-only handle to the signal.
    ///
    /// Useful if you're trying to give read access to another component but ensure that it can't write
    /// to the signal and cause other parts of the DOM to update.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let count = create_rw_signal(cx, 0);
    /// let read_count = count.read_only();
    /// assert_eq!(count.get(), 0);
    /// assert_eq!(read_count.get(), 0);
    /// count.set(1);
    /// assert_eq!(count.get(), 1);
    /// assert_eq!(read_count.get(), 1);
    /// # }).dispose();
    /// ```
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::read_only()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    pub fn read_only(&self) -> ReadSignal<T> {
        ReadSignal {
            runtime: self.runtime,
            id: self.id,
            ty: PhantomData,
            #[cfg(any(debug_assertions, feature = "ssr"))]
            defined_at: std::panic::Location::caller(),
        }
    }

    /// Returns a write-only handle to the signal.
    ///
    /// Useful if you're trying to give write access to another component, or split an
    /// [`RwSignal`] into a [`ReadSignal`] and a [`WriteSignal`].
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let count = create_rw_signal(cx, 0);
    /// let set_count = count.write_only();
    /// assert_eq!(count.get(), 0);
    /// set_count.set(1);
    /// assert_eq!(count.get(), 1);
    /// # }).dispose();
    /// ```
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::write_only()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    pub fn write_only(&self) -> WriteSignal<T> {
        WriteSignal {
            runtime: self.runtime,
            id: self.id,
            ty: PhantomData,
            #[cfg(any(debug_assertions, feature = "ssr"))]
            defined_at: std::panic::Location::caller(),
        }
    }

    /// Splits an `RwSignal` into its getter and setter.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let count = create_rw_signal(cx, 0);
    /// let (get_count, set_count) = count.split();
    /// assert_eq!(count.get(), 0);
    /// assert_eq!(get_count.get(), 0);
    /// set_count.set(1);
    /// assert_eq!(count.get(), 1);
    /// assert_eq!(get_count.get(), 1);
    /// # }).dispose();
    /// ```
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "RwSignal::split()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    pub fn split(&self) -> (ReadSignal<T>, WriteSignal<T>) {
        (
            ReadSignal {
                runtime: self.runtime,
                id: self.id,
                ty: PhantomData,
                #[cfg(any(debug_assertions, feature = "ssr"))]
                defined_at: std::panic::Location::caller(),
            },
            WriteSignal {
                runtime: self.runtime,
                id: self.id,
                ty: PhantomData,
                #[cfg(any(debug_assertions, feature = "ssr"))]
                defined_at: std::panic::Location::caller(),
            },
        )
    }
}

#[derive(Debug, Error)]
pub(crate) enum SignalError {
    #[error("tried to access a signal in a runtime that had been disposed")]
    RuntimeDisposed,
    #[error("tried to access a signal that had been disposed")]
    Disposed,
    #[error("error casting signal to type {0}")]
    Type(&'static str),
}

impl NodeId {
    #[track_caller]
    pub(crate) fn subscribe(
        &self,
        runtime: &Runtime,
        #[allow(unused)] diagnostics: AccessDiagnostics,
    ) {
        // add subscriber
        if let Some(observer) = runtime.observer.get() {
            // add this observer to this node's dependencies (to allow notification)
            let mut subs = runtime.node_subscribers.borrow_mut();
            if let Some(subs) = subs.entry(*self) {
                subs.or_default().borrow_mut().insert(observer);
            }

            // add this node to the observer's sources (to allow cleanup)
            let mut sources = runtime.node_sources.borrow_mut();
            if let Some(sources) = sources.entry(observer) {
                let sources = sources.or_default();
                sources.borrow_mut().insert(*self);
            }
        } else {
            #[cfg(all(debug_assertions, not(feature = "ssr")))]
            {
                if !SpecialNonReactiveZone::is_inside() {
                    let AccessDiagnostics {
                        called_at,
                        defined_at,
                    } = diagnostics;
                    crate::macros::debug_warn!(
                        "At {called_at}, you access a signal or memo (defined \
                         at {defined_at}) outside a reactive tracking \
                         context. This might mean your app is not responding \
                         to changes in signal values in the way you \
                         expect.\n\nHere’s how to fix it:\n\n1. If this is \
                         inside a `view!` macro, make sure you are passing a \
                         function, not a value.\n  ❌ NO  <p>{{x.get() * \
                         2}}</p>\n  ✅ YES <p>{{move || x.get() * \
                         2}}</p>\n\n2. If it’s in the body of a component, \
                         try wrapping this access in a closure: \n  ❌ NO  \
                         let y = x.get() * 2\n  ✅ YES let y = move || \
                         x.get() * 2.\n\n3. If you’re *trying* to access the \
                         value without tracking, use `.get_untracked()` or \
                         `.with_untracked()` instead."
                    );
                }
            }
        }
    }

    fn try_with_no_subscription_inner(
        &self,
        runtime: &Runtime,
    ) -> Result<Rc<RefCell<dyn Any>>, SignalError> {
        runtime.update_if_necessary(*self);
        let nodes = runtime.nodes.borrow();
        let node = nodes.get(*self).ok_or(SignalError::Disposed)?;
        Ok(node.value())
    }

    #[track_caller]
    #[inline(always)]
    pub(crate) fn try_with_no_subscription<T, U>(
        &self,
        runtime: &Runtime,
        f: impl FnOnce(&T) -> U,
    ) -> Result<U, SignalError>
    where
        T: 'static,
    {
        let value = self.try_with_no_subscription_inner(runtime)?;
        let value = value.borrow();
        let value = value
            .downcast_ref::<T>()
            .ok_or_else(|| SignalError::Type(std::any::type_name::<T>()))
            .expect("to downcast signal type");
        Ok(f(value))
    }

    #[track_caller]
    #[inline(always)]
    pub(crate) fn try_with<T, U>(
        &self,
        runtime: &Runtime,
        f: impl FnOnce(&T) -> U,
        diagnostics: AccessDiagnostics,
    ) -> Result<U, SignalError>
    where
        T: 'static,
    {
        self.subscribe(runtime, diagnostics);

        self.try_with_no_subscription(runtime, f)
    }

    #[inline(always)]
    pub(crate) fn with_no_subscription<T, U>(
        &self,
        runtime: RuntimeId,
        f: impl FnOnce(&T) -> U,
    ) -> U
    where
        T: 'static,
    {
        with_runtime(runtime, |runtime| {
            self.try_with_no_subscription(runtime, f).unwrap()
        })
        .expect("runtime to be alive")
    }

    #[inline(always)]
    fn update_value<T, U>(
        &self,
        runtime: RuntimeId,
        f: impl FnOnce(&mut T) -> U,
        #[cfg(debug_assertions)] defined_at: Option<
            &'static std::panic::Location<'static>,
        >,
    ) -> Option<U>
    where
        T: 'static,
    {
        with_runtime(runtime, |runtime| {
            if let Some(value) = runtime.get_value(*self) {
                let mut value = value.borrow_mut();
                if let Some(value) = value.downcast_mut::<T>() {
                    Some(f(value))
                } else {
                    debug_warn!(
                        "[Signal::update] failed when downcasting to \
                         Signal<{}>",
                        std::any::type_name::<T>()
                    );
                    None
                }
            } else {
                #[cfg(debug_assertions)]
                {
                    if let Some(defined_at) = defined_at {
                        debug_warn!(
                            "[Signal::update] You’re trying to update a \
                             Signal<{}> (defined at {defined_at}) that has \
                             already been disposed of. This is probably a \
                             logic error in a component that creates and \
                             disposes of scopes. If it does not cause any \
                             issues, it is safe to ignore this warning, which \
                             occurs only in debug mode.",
                            std::any::type_name::<T>()
                        );
                    }
                }
                None
            }
        })
        .unwrap_or_default()
    }

    #[inline(always)]
    pub(crate) fn update<T, U>(
        &self,
        runtime_id: RuntimeId,
        f: impl FnOnce(&mut T) -> U,
        #[cfg(debug_assertions)] defined_at: Option<
            &'static std::panic::Location<'static>,
        >,
    ) -> Option<U>
    where
        T: 'static,
    {
        with_runtime(runtime_id, |runtime| {
            let updated = if let Some(value) = runtime.get_value(*self) {
                let mut value = value.borrow_mut();
                if let Some(value) = value.downcast_mut::<T>() {
                    Some(f(value))
                } else {
                    debug_warn!(
                        "[Signal::update] failed when downcasting to \
                         Signal<{}>",
                        std::any::type_name::<T>()
                    );
                    None
                }
            } else {
                #[cfg(debug_assertions)]
                {
                    if let Some(defined_at) = defined_at {
                        debug_warn!(
                            "[Signal::update] You’re trying to update a \
                             Signal<{}> (defined at {defined_at}) that has \
                             already been disposed of. This is probably a \
                             logic error in a component that creates and \
                             disposes of scopes. If it does not cause any \
                             issues, it is safe to ignore this warning, which \
                             occurs only in debug mode.",
                            std::any::type_name::<T>()
                        );
                    }
                }
                None
            };

            // notify subscribers
            if updated.is_some() {
                // mark descendants dirty
                runtime.mark_dirty(*self);

                runtime.run_effects();
            }

            updated
        })
        .unwrap_or_default()
    }

    #[inline(always)]
    pub(crate) fn update_with_no_effect<T, U>(
        &self,
        runtime: RuntimeId,
        f: impl FnOnce(&mut T) -> U,
        #[cfg(debug_assertions)] defined_at: Option<
            &'static std::panic::Location<'static>,
        >,
    ) -> Option<U>
    where
        T: 'static,
    {
        // update the value
        self.update_value(
            runtime,
            f,
            #[cfg(debug_assertions)]
            defined_at,
        )
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn format_signal_warning(
    msg: &str,
    #[cfg(any(debug_assertions, feature = "ssr"))]
    defined_at: &'static std::panic::Location<'static>,
) -> String {
    let location = std::panic::Location::caller();

    let defined_at_msg = {
        #[cfg(any(debug_assertions, feature = "ssr"))]
        {
            format!("signal created here: {defined_at}\n")
        }

        #[cfg(not(any(debug_assertions, feature = "ssr")))]
        {
            String::default()
        }
    };

    format!("{msg}\n{defined_at_msg}warning happened here: {location}",)
}

#[cold]
#[inline(never)]
#[track_caller]
pub(crate) fn panic_getting_dead_signal(
    #[cfg(any(debug_assertions, feature = "ssr"))]
    defined_at: &'static std::panic::Location<'static>,
) -> ! {
    panic!(
        "{}",
        format_signal_warning(
            "Attempted to get a signal after it was disposed.",
            #[cfg(any(debug_assertions, feature = "ssr"))]
            defined_at,
        )
    )
}

#[cold]
#[inline(never)]
#[track_caller]
pub(crate) fn warn_updating_dead_signal(
    #[cfg(any(debug_assertions, feature = "ssr"))]
    defined_at: &'static std::panic::Location<'static>,
) {
    console_warn(&format_signal_warning(
        "Attempted to update a signal after it was disposed.",
        #[cfg(any(debug_assertions, feature = "ssr"))]
        defined_at,
    ));
}
