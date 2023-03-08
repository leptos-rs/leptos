#![forbid(unsafe_code)]
use crate::{
    console_warn, create_effect,
    macros::debug_warn,
    node::{NodeId, ReactiveNodeType},
    on_cleanup, queue_microtask,
    runtime::{with_runtime, RuntimeId},
    Runtime, Scope, ScopeProperty,
};
use cfg_if::cfg_if;
use futures::Stream;
use std::{fmt::Debug, marker::PhantomData, pin::Pin, rc::Rc};
use thiserror::Error;

macro_rules! impl_get_fn_traits {
    ($($ty:ident $(($method_name:ident))?),*) => {
        $(
            #[cfg(not(feature = "stable"))]
            impl<T: Clone> FnOnce<()> for $ty<T> {
                type Output = T;

                extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
                    impl_get_fn_traits!(@method_name self $($method_name)?)
                }
            }

            #[cfg(not(feature = "stable"))]
            impl<T: Clone> FnMut<()> for $ty<T> {
                extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
                    impl_get_fn_traits!(@method_name self $($method_name)?)
                }
            }

            #[cfg(not(feature = "stable"))]
            impl<T: Clone> Fn<()> for $ty<T> {
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
            #[cfg(not(feature = "stable"))]
            impl<T> FnOnce<(T,)> for $ty<T> {
                type Output = ();

                extern "rust-call" fn call_once(self, args: (T,)) -> Self::Output {
                    impl_set_fn_traits!(@method_name self $($method_name)? args)
                }
            }

            #[cfg(not(feature = "stable"))]
            impl<T> FnMut<(T,)> for $ty<T> {
                extern "rust-call" fn call_mut(&mut self, args: (T,)) -> Self::Output {
                    impl_set_fn_traits!(@method_name self $($method_name)? args)
                }
            }

            #[cfg(not(feature = "stable"))]
            impl<T> Fn<(T,)> for $ty<T> {
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
    /// Panics if you try to access a signal that was created in a [Scope] that has been disposed.
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
    /// Panics if you try to access a signal that was created in a [Scope] that has been disposed.
    #[track_caller]
    fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O;

    /// Applies a function to the current value of the signal, and subscribes
    /// the running effect to this signal. Returns [`Some`] if the signal is
    /// valid and the function ran, otherwise returns [`None`].
    fn try_with<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O>;
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
    #[deprecated = "Please use `try_update` instead. This method will be \
                    removed in a future version of this crate"]
    fn update_returning<O>(&self, f: impl FnOnce(&mut T) -> O) -> Option<O> {
        self.try_update(f)
    }

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
    /// Panics if you try to access a signal that was created in a [Scope] that has been disposed.
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
    /// Panics if you try to access a signal that was created in a [Scope] that has been disposed.
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
    #[deprecated = "Please use `try_update_untracked` instead. This method \
                    will be removed in a future version of `leptos`"]
    fn update_returning_untracked<U>(
        &self,
        f: impl FnOnce(&mut T) -> U,
    ) -> Option<U> {
        self.try_update_untracked(f)
    }

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
    /// Panics if you try to access a signal that was created in a [Scope] that has been disposed.
    // We're returning an opaque type until impl trait in trait
    // positions are stabilized, and also so any underlying
    // changes are non-breaking
    #[track_caller]
    fn to_stream(&self, cx: Scope) -> Pin<Box<dyn Stream<Item = T>>>;
}

/// Creates a signal, the basic reactive primitive.
///
/// A signal is a piece of data that may change over time,
/// and notifies other code when it has changed. This is the
/// core primitive of Leptos’s reactive system.
///
/// Takes a reactive [Scope] and the initial value as arguments,
/// and returns a tuple containing a [ReadSignal] and a [WriteSignal],
/// each of which can be called as a function.
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (count, set_count) = create_signal(cx, 0);
///
/// // ✅ calling the getter clones and returns the value
/// assert_eq!(count(), 0);
///
/// // ✅ calling the setter sets the value
/// set_count(1);
/// assert_eq!(count(), 1);
///
/// // ❌ don't try to call the getter within the setter
/// // set_count(count() + 1);
///
/// // ✅ instead, use .update() to mutate the value in place
/// set_count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count(), 2);
///
/// // ✅ you can create "derived signals" with the same Fn() -> T interface
/// let double_count = move || count() * 2; // signals are `Copy` so you can `move` them anywhere
/// set_count(0);
/// assert_eq!(double_count(), 0);
/// set_count(1);
/// assert_eq!(double_count(), 2);
/// # }).dispose();
/// #
/// ```
#[cfg_attr(
    debug_assertions,
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
    eprintln!("created signal {:?}", s.0.id);
    cx.with_scope_property(|prop| prop.push(ScopeProperty::Signal(s.0.id)));
    s
}

/// Works exactly as [create_signal], but creates multiple signals at once.
#[cfg_attr(
    debug_assertions,
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

/// Works exactly as [create_many_signals], but applies the map function to each signal pair.
#[cfg_attr(
    debug_assertions,
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
/// [Stream](futures::stream::Stream).
/// If the stream has not yet emitted a value since the signal was created, the signal's
/// value will be `None`.
///
/// **Note**: If used on the server side during server rendering, this will return `None`
/// immediately and not begin driving the stream.
#[cfg_attr(
    debug_assertions,
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
    cfg_if! {
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
/// `ReadSignal` is also [Copy] and `'static`, so it can very easily moved into closures
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
/// assert_eq!(count(), 0);
///
/// // ✅ calling the setter sets the value
/// set_count(1);
/// assert_eq!(count(), 1);
///
/// // ❌ don't try to call the getter within the setter
/// // set_count(count() + 1);
///
/// // ✅ instead, use .update() to mutate the value in place
/// set_count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count(), 2);
///
/// // ✅ you can create "derived signals" with the same Fn() -> T interface
/// let double_count = move || count() * 2; // signals are `Copy` so you can `move` them anywhere
/// set_count(0);
/// assert_eq!(double_count(), 0);
/// set_count(1);
/// assert_eq!(double_count(), 2);
/// # }).dispose();
/// #
/// ```
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ReadSignal<T>
where
    T: 'static,
{
    pub(crate) runtime: RuntimeId,
    pub(crate) id: NodeId,
    pub(crate) ty: PhantomData<T>,
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static std::panic::Location<'static>,
}

impl<T: Clone> SignalGetUntracked<T> for ReadSignal<T> {
    #[cfg_attr(
        debug_assertions,
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
                #[cfg(debug_assertions)]
                self.defined_at,
            ),
        }
    }

    #[cfg_attr(
        debug_assertions,
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
        debug_assertions,
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
    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        self.with_no_subscription(f)
    }

    #[cfg_attr(
        debug_assertions,
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
    fn try_with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        with_runtime(self.runtime, |runtime| self.id.try_with(runtime, f))
            .ok()
            .transpose()
            .ok()
            .flatten()
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
/// let first_char = move || name().chars().next().unwrap();
/// assert_eq!(first_char(), 'A');
///
/// // ✅ gets the first char without cloning the `String`
/// let first_char = move || name.with(|n| n.chars().next().unwrap());
/// assert_eq!(first_char(), 'A');
/// set_name("Bob".to_string());
/// assert_eq!(first_char(), 'B');
/// # });
/// ```
impl<T> SignalWith<T> for ReadSignal<T> {
    #[cfg_attr(
        debug_assertions,
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
    fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        match with_runtime(self.runtime, |runtime| self.id.try_with(runtime, f))
            .expect("runtime to be alive ")
        {
            Ok(o) => o,
            Err(_) => panic_getting_dead_signal(
                #[cfg(debug_assertions)]
                self.defined_at,
            ),
        }
    }

    #[cfg_attr(
        debug_assertions,
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
    fn try_with<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        with_runtime(self.runtime, |runtime| self.id.try_with(runtime, f).ok())
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
/// // count() is shorthand for count.get()
/// assert_eq!(count(), 0);
/// # });
/// ```
impl<T: Clone> SignalGet<T> for ReadSignal<T> {
    #[cfg_attr(
        debug_assertions,
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
    fn get(&self) -> T {
        match with_runtime(self.runtime, |runtime| {
            self.id.try_with(runtime, T::clone)
        })
        .expect("runtime to be alive")
        {
            Ok(t) => t,
            Err(_) => panic_getting_dead_signal(
                #[cfg(debug_assertions)]
                self.defined_at,
            ),
        }
    }

    #[cfg_attr(
        debug_assertions,
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
        debug_assertions,
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

impl<T> ReadSignal<T>
where
    T: 'static,
{
    pub(crate) fn with_no_subscription<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        self.id.with_no_subscription(self.runtime, f)
    }

    #[cfg(feature = "hydrate")]
    pub(crate) fn subscribe(&self) {
        _ = with_runtime(self.runtime, |runtime| self.id.subscribe(runtime))
    }

    /// Applies the function to the current Signal, if it exists, and subscribes
    /// the running effect.
    pub(crate) fn try_with<U>(
        &self,
        f: impl FnOnce(&T) -> U,
    ) -> Result<U, SignalError> {
        match with_runtime(self.runtime, |runtime| self.id.try_with(runtime, f))
        {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(SignalError::RuntimeDisposed),
        }
    }
}

impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime,
            id: self.id,
            ty: PhantomData,
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
        }
    }
}

impl<T> Copy for ReadSignal<T> {}

/// The setter for a reactive signal.
///
/// A signal is a piece of data that may change over time,
/// and notifies other code when it has changed. This is the
/// core primitive of Leptos’s reactive system.
///
/// Calling [WriteSignal::update] will mutate the signal’s value in place,
/// and notify all subscribers that the signal’s value has changed.
///
/// `WriteSignal` implements [Fn], such that `set_value(new_value)` is equivalent to
/// `set_value.update(|value| *value = new_value)`.
///
/// `WriteSignal` is [Copy] and `'static`, so it can very easily moved into closures
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
/// set_count(1);
/// assert_eq!(count(), 1);
///
/// // ❌ don't try to call the getter within the setter
/// // set_count(count() + 1);
///
/// // ✅ instead, use .update() to mutate the value in place
/// set_count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count(), 2);
/// # }).dispose();
/// #
/// ```
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct WriteSignal<T>
where
    T: 'static,
{
    pub(crate) runtime: RuntimeId,
    pub(crate) id: NodeId,
    pub(crate) ty: PhantomData<T>,
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static std::panic::Location<'static>,
}

impl<T> SignalSetUntracked<T> for WriteSignal<T>
where
    T: 'static,
{
    #[cfg_attr(
        debug_assertions,
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
        self.id
            .update_with_no_effect(self.runtime, |v| *v = new_value);
    }

    #[cfg_attr(
        debug_assertions,
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

        self.id
            .update(self.runtime, |t| *t = new_value.take().unwrap());

        new_value
    }
}

impl<T> SignalUpdateUntracked<T> for WriteSignal<T> {
    #[cfg_attr(
        debug_assertions,
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
    fn update_untracked(&self, f: impl FnOnce(&mut T)) {
        self.id.update_with_no_effect(self.runtime, f);
    }

    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "WriteSignal::update_returning_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn update_returning_untracked<U>(
        &self,
        f: impl FnOnce(&mut T) -> U,
    ) -> Option<U> {
        self.id.update_with_no_effect(self.runtime, f)
    }

    fn try_update_untracked<O>(
        &self,
        f: impl FnOnce(&mut T) -> O,
    ) -> Option<O> {
        self.id.update_with_no_effect(self.runtime, f)
    }
}

/// # Examples
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (count, set_count) = create_signal(cx, 0);
///
/// // notifies subscribers
/// set_count.update(|n| *n = 1); // it's easier just to call set_count(1), though!
/// assert_eq!(count(), 1);
///
/// // you can include arbitrary logic in this update function
/// // also notifies subscribers, even though the value hasn't changed
/// set_count.update(|n| if *n > 3 { *n += 1 });
/// assert_eq!(count(), 1);
/// # }).dispose();
/// ```
impl<T> SignalUpdate<T> for WriteSignal<T> {
    #[cfg_attr(
        debug_assertions,
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
    fn update(&self, f: impl FnOnce(&mut T)) {
        if self.id.update(self.runtime, f).is_none() {
            warn_updating_dead_signal(
                #[cfg(debug_assertions)]
                self.defined_at,
            );
        }
    }

    #[cfg_attr(
        debug_assertions,
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
    fn try_update<O>(&self, f: impl FnOnce(&mut T) -> O) -> Option<O> {
        self.id.update(self.runtime, f)
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
/// set_count.update(|n| *n = 1); // it's easier just to call set_count(1), though!
/// assert_eq!(count(), 1);
///
/// // you can include arbitrary logic in this update function
/// // also notifies subscribers, even though the value hasn't changed
/// set_count.update(|n| if *n > 3 { *n += 1 });
/// assert_eq!(count(), 1);
/// # }).dispose();
/// ```
impl<T> SignalSet<T> for WriteSignal<T> {
    #[cfg_attr(
        debug_assertions,
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
        self.id.update(self.runtime, |n| *n = new_value);
    }

    #[cfg_attr(
        debug_assertions,
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

        self.id
            .update(self.runtime, |t| *t = new_value.take().unwrap());

        new_value
    }
}

impl<T> Clone for WriteSignal<T> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime,
            id: self.id,
            ty: PhantomData,
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
        }
    }
}

impl<T> Copy for WriteSignal<T> {}

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
/// assert_eq!(count(), 1);
///
/// // ❌ don't try to call the getter within the setter
/// // count.set(count.get() + 1);
///
/// // ✅ instead, use .update() to mutate the value in place
/// count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count(), 2);
/// # }).dispose();
/// #
/// ```
#[cfg_attr(
    debug_assertions,
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
    cx.with_scope_property(|prop| prop.push(ScopeProperty::Signal(s.id)));
    s
}

/// A signal that combines the getter and setter into one value, rather than
/// separating them into a [ReadSignal] and a [WriteSignal]. You may prefer this
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
/// assert_eq!(count(), 1);
///
/// // ❌ don't try to call the getter within the setter
/// // count.set(count.get() + 1);
///
/// // ✅ instead, use .update() to mutate the value in place
/// count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count(), 2);
/// # }).dispose();
/// #
/// ```
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct RwSignal<T>
where
    T: 'static,
{
    pub(crate) runtime: RuntimeId,
    pub(crate) id: NodeId,
    pub(crate) ty: PhantomData<T>,
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static std::panic::Location<'static>,
}

impl<T> Clone for RwSignal<T> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime,
            id: self.id,
            ty: self.ty,
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
        }
    }
}

impl<T> Copy for RwSignal<T> {}

impl<T: Clone> SignalGetUntracked<T> for RwSignal<T> {
    #[cfg_attr(
        debug_assertions,
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
        debug_assertions,
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
                #[cfg(debug_assertions)]
                self.defined_at,
            ),
        }
    }
}

impl<T> SignalWithUntracked<T> for RwSignal<T> {
    #[cfg_attr(
        debug_assertions,
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
    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        self.id.with_no_subscription(self.runtime, f)
    }

    #[cfg_attr(
        debug_assertions,
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
    fn try_with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        with_runtime(self.runtime, |runtime| self.id.try_with(runtime, f))
            .ok()
            .transpose()
            .ok()
            .flatten()
    }
}

impl<T> SignalSetUntracked<T> for RwSignal<T> {
    #[cfg_attr(
        debug_assertions,
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
        self.id
            .update_with_no_effect(self.runtime, |v| *v = new_value);
    }

    #[cfg_attr(
        debug_assertions,
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

        self.id
            .update(self.runtime, |t| *t = new_value.take().unwrap());

        new_value
    }
}

impl<T> SignalUpdateUntracked<T> for RwSignal<T> {
    #[cfg_attr(
    debug_assertions,
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
    fn update_untracked(&self, f: impl FnOnce(&mut T)) {
        self.id.update_with_no_effect(self.runtime, f);
    }

    #[cfg_attr(
    debug_assertions,
    instrument(
        level = "trace",
        name = "RwSignal::update_returning_untracked()",
        skip_all,
        fields(
            id = ?self.id,
            defined_at = %self.defined_at,
            ty = %std::any::type_name::<T>()
        )
    )
    )]
    fn update_returning_untracked<U>(
        &self,
        f: impl FnOnce(&mut T) -> U,
    ) -> Option<U> {
        self.id.update_with_no_effect(self.runtime, f)
    }

    #[cfg_attr(
        debug_assertions,
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
    fn try_update_untracked<O>(
        &self,
        f: impl FnOnce(&mut T) -> O,
    ) -> Option<O> {
        self.id.update_with_no_effect(self.runtime, f)
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
/// let first_char = move || name().chars().next().unwrap();
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
        debug_assertions,
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
    fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        match with_runtime(self.runtime, |runtime| self.id.try_with(runtime, f))
            .expect("runtime to be alive")
        {
            Ok(o) => o,
            Err(_) => panic_getting_dead_signal(
                #[cfg(debug_assertions)]
                self.defined_at,
            ),
        }
    }

    #[cfg_attr(
        debug_assertions,
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
    fn try_with<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        with_runtime(self.runtime, |runtime| self.id.try_with(runtime, f).ok())
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
/// // count() is shorthand for count.get()
/// assert_eq!(count(), 0);
/// # }).dispose();
/// #
/// ```
impl<T: Clone> SignalGet<T> for RwSignal<T> {
    #[cfg_attr(
        debug_assertions,
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
    fn get(&self) -> T
    where
        T: Clone,
    {
        match with_runtime(self.runtime, |runtime| {
            self.id.try_with(runtime, T::clone)
        })
        .expect("runtime to be alive")
        {
            Ok(t) => t,
            Err(_) => panic_getting_dead_signal(
                #[cfg(debug_assertions)]
                self.defined_at,
            ),
        }
    }

    #[cfg_attr(
        debug_assertions,
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
    fn try_get(&self) -> Option<T> {
        with_runtime(self.runtime, |runtime| {
            self.id.try_with(runtime, Clone::clone).ok()
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
/// count.update(|n| *n = 1); // it's easier just to call set_count(1), though!
/// assert_eq!(count(), 1);
///
/// // you can include arbitrary logic in this update function
/// // also notifies subscribers, even though the value hasn't changed
/// count.update(|n| {
///     if *n > 3 {
///         *n += 1
///     }
/// });
/// assert_eq!(count(), 1);
/// # }).dispose();
/// ```
impl<T> SignalUpdate<T> for RwSignal<T> {
    #[cfg_attr(
        debug_assertions,
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
    fn update(&self, f: impl FnOnce(&mut T)) {
        if self.id.update(self.runtime, f).is_none() {
            warn_updating_dead_signal(
                #[cfg(debug_assertions)]
                self.defined_at,
            );
        }
    }

    #[cfg_attr(
        debug_assertions,
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
    fn try_update<O>(&self, f: impl FnOnce(&mut T) -> O) -> Option<O> {
        self.id.update(self.runtime, f)
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let count = create_rw_signal(cx, 0);
///
/// assert_eq!(count(), 0);
/// count.set(1);
/// assert_eq!(count(), 1);
/// # }).dispose();
/// ```
impl<T> SignalSet<T> for RwSignal<T> {
    #[cfg_attr(
        debug_assertions,
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
        self.id.update(self.runtime, |n| *n = value);
    }

    #[cfg_attr(
        debug_assertions,
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

        self.id
            .update(self.runtime, |t| *t = new_value.take().unwrap());

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
    /// assert_eq!(count(), 0);
    /// assert_eq!(read_count(), 0);
    /// count.set(1);
    /// assert_eq!(count(), 1);
    /// assert_eq!(read_count(), 1);
    /// # }).dispose();
    /// ```
    #[cfg_attr(
        debug_assertions,
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
            #[cfg(debug_assertions)]
            defined_at: std::panic::Location::caller(),
        }
    }

    /// Returns a write-only handle to the signal.
    ///
    /// Useful if you're trying to give write access to another component, or split an
    /// `RwSignal` into a [ReadSignal] and a [WriteSignal].
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let count = create_rw_signal(cx, 0);
    /// let set_count = count.write_only();
    /// assert_eq!(count(), 0);
    /// set_count(1);
    /// assert_eq!(count(), 1);
    /// # }).dispose();
    /// ```
    #[cfg_attr(
        debug_assertions,
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
            #[cfg(debug_assertions)]
            defined_at: std::panic::Location::caller(),
        }
    }

    /// Splits an `RwSignal` into its getter and setter.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let count = create_rw_signal(cx, 0);
    /// let (get_count, set_count) = count.split();
    /// assert_eq!(count(), 0);
    /// assert_eq!(get_count(), 0);
    /// set_count(1);
    /// assert_eq!(count(), 1);
    /// assert_eq!(get_count(), 1);
    /// # }).dispose();
    /// ```
    #[cfg_attr(
        debug_assertions,
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
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            },
            WriteSignal {
                runtime: self.runtime,
                id: self.id,
                ty: PhantomData,
                #[cfg(debug_assertions)]
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
    pub(crate) fn subscribe(&self, runtime: &Runtime) {
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
        }
    }

    pub(crate) fn try_with_no_subscription<T, U>(
        &self,
        runtime: &Runtime,
        f: impl FnOnce(&T) -> U,
    ) -> Result<U, SignalError>
    where
        T: 'static,
    {
        runtime.update_if_necessary(*self);
        let nodes = runtime.nodes.borrow();
        let node = nodes.get(*self).ok_or(SignalError::Disposed)?;
        let value = Rc::clone(&node.value);

        let value = value.borrow();
        let value = value
            .downcast_ref::<T>()
            .ok_or_else(|| SignalError::Type(std::any::type_name::<T>()))
            .expect("downcast issue");
        Ok(f(value))
    }

    pub(crate) fn try_with<T, U>(
        &self,
        runtime: &Runtime,
        f: impl FnOnce(&T) -> U,
    ) -> Result<U, SignalError>
    where
        T: 'static,
    {
        self.subscribe(runtime);

        self.try_with_no_subscription(runtime, f)
    }

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
        .expect("tried to access a signal in a runtime that has been disposed")
    }

    fn update_value<T, U>(
        &self,
        runtime: RuntimeId,
        f: impl FnOnce(&mut T) -> U,
    ) -> Option<U>
    where
        T: 'static,
    {
        with_runtime(runtime, |runtime| {
            let value = {
                let signals = runtime.nodes.borrow();
                signals.get(*self).map(|node| Rc::clone(&node.value))
            };
            if let Some(value) = value {
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
                debug_warn!(
                    "[Signal::update] You’re trying to update a Signal<{}> \
                     that has already been disposed of. This is probably \
                     either a logic error in a component that creates and \
                     disposes of scopes, or a Resource resolving after its \
                     scope has been dropped without having been cleaned up.",
                    std::any::type_name::<T>()
                );
                None
            }
        })
        .unwrap_or_default()
    }

    pub(crate) fn update<T, U>(
        &self,
        runtime_id: RuntimeId,
        f: impl FnOnce(&mut T) -> U,
    ) -> Option<U>
    where
        T: 'static,
    {
        with_runtime(runtime_id, |runtime| {
            eprintln!("\nupdating signal");
            // update the value
            // let updated = self.update_value(runtime_id, f);

            let value = {
                let signals = runtime.nodes.borrow();
                signals.get(*self).map(|node| Rc::clone(&node.value))
            };
            let updated = if let Some(value) = value {
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
                debug_warn!(
                    "[Signal::update] You’re trying to update a Signal<{}> \
                     that has already been disposed of. This is probably \
                     either a logic error in a component that creates and \
                     disposes of scopes, or a Resource resolving after its \
                     scope has been dropped without having been cleaned up.",
                    std::any::type_name::<T>()
                );
                None
            };
            // mark descendants dirty
            eprintln!("marking children of {self:?}");
            runtime.mark_dirty(*self);

            // notify subscribers
            if updated.is_some() {
                //queue_microtask(move || {
                Runtime::run_effects(runtime_id);
                //});
            };
            updated
        })
        .unwrap_or_default()
    }

    pub(crate) fn update_with_no_effect<T, U>(
        &self,
        runtime: RuntimeId,
        f: impl FnOnce(&mut T) -> U,
    ) -> Option<U>
    where
        T: 'static,
    {
        // update the value
        self.update_value(runtime, f)
    }
}

#[track_caller]
fn format_signal_warning(
    msg: &str,
    #[cfg(debug_assertions)] defined_at: &'static std::panic::Location<'static>,
) -> String {
    let location = std::panic::Location::caller();

    let defined_at_msg = {
        #[cfg(debug_assertions)]
        {
            format!("signal created here: {defined_at}\n")
        }

        #[cfg(not(debug_assertions))]
        {
            String::default()
        }
    };

    format!("{msg}\n{defined_at_msg}warning happened here: {location}",)
}

#[track_caller]
pub(crate) fn panic_getting_dead_signal(
    #[cfg(debug_assertions)] defined_at: &'static std::panic::Location<'static>,
) -> ! {
    panic!(
        "{}",
        format_signal_warning(
            "Attempted to get a signal after it was disposed.",
            #[cfg(debug_assertions)]
            defined_at,
        )
    )
}

#[track_caller]
pub(crate) fn warn_updating_dead_signal(
    #[cfg(debug_assertions)] defined_at: &'static std::panic::Location<'static>,
) {
    console_warn(&format_signal_warning(
        "Attempted to update a signal after it was disposed.",
        #[cfg(debug_assertions)]
        defined_at,
    ));
}
