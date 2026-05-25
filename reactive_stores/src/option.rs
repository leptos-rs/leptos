use crate::{
    KeyMap, StoreField, StoreFieldTrigger,
    path::{StorePath, StorePathSegment},
};
use reactive_graph::{
    signal::{
        ArcTrigger,
        guards::{Mapped, MappedMut, WriteGuard},
    },
    traits::{
        DefinedAt, FlattenOptionRefOption, IsDisposed, Notify, Read,
        ReadUntracked, Track, UntrackableGuard, Write,
    },
};
use std::{
    iter,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    panic::Location,
};

/// Accesses the inner value of an `Option`-typed store field.
///
/// Unlike a plain [`Subfield`](crate::Subfield), this projection is fallible:
/// its [`reader`](StoreField::reader)/[`writer`](StoreField::writer) return
/// `None` when the underlying field is currently `None`, instead of handing
/// back a guard that panics when dereferenced.
pub struct OptionSubfield<Inner, T> {
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    defined_at: &'static Location<'static>,
    path_segment: StorePathSegment,
    inner: Inner,
    ty: PhantomData<T>,
}

impl<Inner, T> Clone for OptionSubfield<Inner, T>
where
    Inner: Clone,
{
    fn clone(&self) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            path_segment: self.path_segment,
            inner: self.inner.clone(),
            ty: self.ty,
        }
    }
}

impl<Inner, T> Copy for OptionSubfield<Inner, T> where Inner: Copy {}

impl<Inner, T> OptionSubfield<Inner, T> {
    /// Creates an accessor for the inner value of an `Option`-typed field.
    #[track_caller]
    pub fn new(inner: Inner) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            path_segment: 0.into(),
            inner,
            ty: PhantomData,
        }
    }
}

impl<Inner, T> StoreField for OptionSubfield<Inner, T>
where
    Inner: StoreField<Value = Option<T>>,
    T: 'static,
{
    type Value = T;
    type Reader = Mapped<Inner::Reader, T>;
    type Writer = MappedMut<WriteGuard<Vec<ArcTrigger>, Inner::Writer>, T>;

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner
            .path()
            .into_iter()
            .chain(iter::once(self.path_segment))
    }

    fn path_unkeyed(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner
            .path_unkeyed()
            .into_iter()
            .chain(iter::once(self.path_segment))
    }

    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner.get_trigger(path)
    }

    fn get_trigger_unkeyed(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner.get_trigger_unkeyed(path)
    }

    fn reader(&self) -> Option<Self::Reader> {
        let inner = self.inner.reader()?;
        // The reader holds the inner lock for its whole lifetime, so the
        // value cannot toggle to `None` before the (lazy) projection runs on
        // deref. Bail out here instead of handing back a guard that would
        // panic in `as_ref().unwrap()`.
        if inner.is_none() {
            return None;
        }
        Some(Mapped::new_with_guard(inner, |t| t.as_ref().unwrap()))
    }

    fn writer(&self) -> Option<Self::Writer> {
        let mut parent = self.inner.writer()?;
        // See `reader`: the write guard holds the inner lock, so a single
        // check here keeps the `as_mut().unwrap()` projection panic-free.
        if parent.is_none() {
            return None;
        }
        // untrack the parent so it doesn't notify its `this` trigger (which
        // would notify siblings); the path triggers are included below.
        parent.untrack();
        let triggers = self.triggers_for_current_path();
        let guard = WriteGuard::new(triggers, parent);
        Some(MappedMut::new(
            guard,
            |t| t.as_ref().unwrap(),
            |t| t.as_mut().unwrap(),
        ))
    }

    #[inline(always)]
    fn keys(&self) -> Option<KeyMap> {
        self.inner.keys()
    }

    #[track_caller]
    fn track_field(&self) {
        let mut full_path = self.path().into_iter().collect::<StorePath>();
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.this.track();
        trigger.children.track();

        while !full_path.is_empty() {
            full_path.pop();
            let inner = self.get_trigger(full_path.clone());
            inner.this.track();
        }
    }
}

impl<Inner, T> DefinedAt for OptionSubfield<Inner, T> {
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        {
            Some(self.defined_at)
        }
        #[cfg(not(any(debug_assertions, leptos_debuginfo)))]
        {
            None
        }
    }
}

impl<Inner, T> IsDisposed for OptionSubfield<Inner, T>
where
    Inner: IsDisposed,
{
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}

impl<Inner, T> Notify for OptionSubfield<Inner, T>
where
    Inner: StoreField<Value = Option<T>>,
    T: 'static,
{
    #[track_caller]
    fn notify(&self) {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.this.notify();
        trigger.children.notify();
    }
}

impl<Inner, T> Track for OptionSubfield<Inner, T>
where
    Inner: StoreField<Value = Option<T>> + Track + 'static,
    T: 'static,
{
    #[track_caller]
    fn track(&self) {
        self.track_field();
    }
}

impl<Inner, T> ReadUntracked for OptionSubfield<Inner, T>
where
    Inner: StoreField<Value = Option<T>>,
    T: 'static,
{
    type Value = <Self as StoreField>::Reader;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.reader()
    }
}

impl<Inner, T> Write for OptionSubfield<Inner, T>
where
    Inner: StoreField<Value = Option<T>>,
    T: 'static,
{
    type Value = T;

    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>> {
        self.writer()
    }

    fn try_write_untracked(
        &self,
    ) -> Option<impl DerefMut<Target = Self::Value>> {
        self.writer().map(|mut writer| {
            writer.untrack();
            writer
        })
    }
}

/// Extends optional store fields, with the ability to unwrap or map over them.
pub trait OptionStoreExt
where
    Self: StoreField<Value = Option<Self::Output>>,
{
    /// The inner type of the `Option<_>` this field holds.
    type Output;

    /// Provides access to the inner value, as a subfield, unwrapping the outer value.
    ///
    /// The returned field reads and writes fallibly: if the outer value becomes
    /// `None` before the projection runs, its reader/writer yield `None` rather
    /// than panicking.
    fn unwrap(self) -> OptionSubfield<Self, Self::Output>;

    /// Inverts a subfield of an `Option` to an `Option` of a subfield.
    fn invert(self) -> Option<OptionSubfield<Self, Self::Output>> {
        self.map(|f| f)
    }

    /// Reactively maps over the field.
    ///
    /// This returns `None` if the subfield is currently `None`,
    /// and a new store subfield with the inner value if it is `Some`. This can be used in some
    /// other reactive context, which will cause it to re-run if the field toggles between `None`
    /// and `Some(_)`.
    fn map<U>(
        self,
        map_fn: impl FnOnce(OptionSubfield<Self, Self::Output>) -> U,
    ) -> Option<U>;

    /// Unreactively maps over the field.
    ///
    /// This returns `None` if the subfield is currently `None`,
    /// and a new store subfield with the inner value if it is `Some`. This is an unreactive variant of
    /// `[OptionStoreExt::map]`, and will not cause the reactive context to re-run if the field changes.
    fn map_untracked<U>(
        self,
        map_fn: impl FnOnce(OptionSubfield<Self, Self::Output>) -> U,
    ) -> Option<U>;
}

impl<T, S> OptionStoreExt for S
where
    S: StoreField<Value = Option<T>> + Read + ReadUntracked,
    <S as Read>::Value: Deref<Target = Option<T>>,
    <S as ReadUntracked>::Value: Deref<Target = Option<T>>,
{
    type Output = T;

    fn unwrap(self) -> OptionSubfield<Self, Self::Output> {
        OptionSubfield::new(self)
    }

    fn map<U>(
        self,
        map_fn: impl FnOnce(OptionSubfield<S, T>) -> U,
    ) -> Option<U> {
        if self.try_read().as_deref().flatten().is_some() {
            Some(map_fn(self.unwrap()))
        } else {
            None
        }
    }

    fn map_untracked<U>(
        self,
        map_fn: impl FnOnce(OptionSubfield<S, T>) -> U,
    ) -> Option<U> {
        if self.try_read_untracked().as_deref().flatten().is_some() {
            Some(map_fn(self.unwrap()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as reactive_stores, Patch as _, Store};
    use any_spawner::Executor;
    use reactive_graph::{
        effect::Effect,
        traits::{Get, Read, ReadUntracked, Set, Write},
    };
    use reactive_stores_macro::Patch;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    pub async fn tick() {
        Executor::tick().await;
    }

    #[derive(Debug, Clone, Store)]
    pub struct User {
        pub name: Option<Name>,
    }

    #[derive(Debug, Clone, Store)]
    pub struct Name {
        pub first_name: Option<String>,
    }

    #[tokio::test]
    async fn substores_reachable_through_option() {
        use crate::OptionStoreExt;

        _ = any_spawner::Executor::init_tokio();

        let combined_count = Arc::new(AtomicUsize::new(0));

        let store = Store::new(User { name: None });

        Effect::new_sync({
            let combined_count = Arc::clone(&combined_count);
            move |prev: Option<()>| {
                if prev.is_none() {
                    println!("first run");
                } else {
                    println!("next run");
                }

                if store.name().read().is_some() {
                    println!(
                        "inner value = {:?}",
                        *store.name().unwrap().first_name().read()
                    );
                } else {
                    println!("no inner value");
                }

                combined_count.fetch_add(1, Ordering::Relaxed);
            }
        });

        tick().await;
        store.name().set(Some(Name {
            first_name: Some("Greg".into()),
        }));
        tick().await;
        store.name().set(None);
        tick().await;
        store.name().set(Some(Name {
            first_name: Some("Bob".into()),
        }));
        tick().await;
        store
            .name()
            .unwrap()
            .first_name()
            .write()
            .as_mut()
            .unwrap()
            .push_str("!!!");
        tick().await;
        assert_eq!(combined_count.load(Ordering::Relaxed), 5);
        assert_eq!(
            store
                .name()
                .read_untracked()
                .as_ref()
                .unwrap()
                .first_name
                .as_ref()
                .unwrap(),
            "Bob!!!"
        );
    }

    #[tokio::test]
    async fn mapping_over_optional_store_field() {
        use crate::OptionStoreExt;

        _ = any_spawner::Executor::init_tokio();

        let parent_count = Arc::new(AtomicUsize::new(0));
        let inner_count = Arc::new(AtomicUsize::new(0));

        let store = Store::new(User { name: None });

        Effect::new_sync({
            let parent_count = Arc::clone(&parent_count);
            move |prev: Option<()>| {
                if prev.is_none() {
                    println!("parent: first run");
                } else {
                    println!("parent: next run");
                }

                println!("  is_some = {}", store.name().read().is_some());
                parent_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        Effect::new_sync({
            let inner_count = Arc::clone(&inner_count);
            move |prev: Option<()>| {
                if prev.is_none() {
                    println!("inner: first run");
                } else {
                    println!("inner: next run");
                }

                println!(
                    "store inner value length = {:?}",
                    store.name().map(|inner| inner
                        .first_name()
                        .get()
                        .unwrap_or_default()
                        .len())
                );
                inner_count.fetch_add(1, Ordering::Relaxed);
            }
        });

        tick().await;
        assert_eq!(parent_count.load(Ordering::Relaxed), 1);
        assert_eq!(inner_count.load(Ordering::Relaxed), 1);

        store.name().set(Some(Name {
            first_name: Some("Greg".into()),
        }));
        tick().await;
        assert_eq!(parent_count.load(Ordering::Relaxed), 2);
        assert_eq!(inner_count.load(Ordering::Relaxed), 2);

        println!("\nUpdating first name only");
        store
            .name()
            .unwrap()
            .first_name()
            .write()
            .as_mut()
            .unwrap()
            .push_str("!!!");

        tick().await;
        assert_eq!(parent_count.load(Ordering::Relaxed), 3);
        assert_eq!(inner_count.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn patch() {
        use crate::OptionStoreExt;

        _ = any_spawner::Executor::init_tokio();

        #[derive(Debug, Clone, Store, Patch)]
        struct Outer {
            inner: Option<Inner>,
        }

        #[derive(Debug, Clone, Store, Patch)]
        struct Inner {
            first: String,
            second: String,
        }

        let store = Store::new(Outer {
            inner: Some(Inner {
                first: "A".to_owned(),
                second: "B".to_owned(),
            }),
        });

        let parent_count = Arc::new(AtomicUsize::new(0));
        let inner_first_count = Arc::new(AtomicUsize::new(0));
        let inner_second_count = Arc::new(AtomicUsize::new(0));

        Effect::new_sync({
            let parent_count = Arc::clone(&parent_count);
            move |prev: Option<()>| {
                if prev.is_none() {
                    println!("parent: first run");
                } else {
                    println!("parent: next run");
                }

                println!("  value = {:?}", store.inner().get());
                parent_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        Effect::new_sync({
            let inner_first_count = Arc::clone(&inner_first_count);
            move |prev: Option<()>| {
                if prev.is_none() {
                    println!("inner_first: first run");
                } else {
                    println!("inner_first: next run");
                }

                // note: we specifically want to test whether using `.patch()`
                // correctly limits notifications on the first field when only the second
                // field has changed
                //
                // `.map()` would also track the parent field (to track when it changed from Some
                // to None), which would mean the notification numbers were always the same
                //
                // so here, we'll do `.map_untracked()`, but in general in a real case you'd want
                // to use `.map()` so that if the parent switches to None you do track that
                println!(
                    "  value = {:?}",
                    store.inner().map_untracked(|inner| inner.first().get())
                );
                inner_first_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        Effect::new_sync({
            let inner_second_count = Arc::clone(&inner_second_count);
            move |prev: Option<()>| {
                if prev.is_none() {
                    println!("inner_second: first run");
                } else {
                    println!("inner_second: next run");
                }

                println!(
                    "  value = {:?}",
                    store.inner().map(|inner| inner.second().get())
                );
                inner_second_count.fetch_add(1, Ordering::Relaxed);
            }
        });

        tick().await;
        assert_eq!(parent_count.load(Ordering::Relaxed), 1);
        assert_eq!(inner_first_count.load(Ordering::Relaxed), 1);
        assert_eq!(inner_second_count.load(Ordering::Relaxed), 1);

        println!("\npatching with A/C");
        store.patch(Outer {
            inner: Some(Inner {
                first: "A".to_string(),
                second: "C".to_string(),
            }),
        });

        tick().await;
        assert_eq!(parent_count.load(Ordering::Relaxed), 2);
        assert_eq!(inner_first_count.load(Ordering::Relaxed), 1);
        assert_eq!(inner_second_count.load(Ordering::Relaxed), 2);

        store.patch(Outer { inner: None });

        tick().await;
        assert_eq!(parent_count.load(Ordering::Relaxed), 3);
        assert_eq!(inner_first_count.load(Ordering::Relaxed), 2);
        assert_eq!(inner_second_count.load(Ordering::Relaxed), 3);

        println!("\npatching with A/B");
        store.patch(Outer {
            inner: Some(Inner {
                first: "A".to_string(),
                second: "B".to_string(),
            }),
        });

        tick().await;
        assert_eq!(parent_count.load(Ordering::Relaxed), 4);
        assert_eq!(inner_first_count.load(Ordering::Relaxed), 2);
        assert_eq!(inner_second_count.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn unwrap_reads_as_none_after_option_is_cleared() {
        use crate::OptionStoreExt;
        use reactive_graph::owner::Owner;

        #[derive(Debug, Clone, Store)]
        struct State {
            value: Option<i32>,
        }

        let owner = Owner::new();
        owner.set();

        let store = Store::new(State { value: Some(1) });

        // Capture the unwrapped inner field while the option is `Some`.
        let inner = OptionStoreExt::unwrap(store.value());
        assert_eq!(inner.try_read_untracked().map(|g| *g), Some(1));

        // Another writer clears the option after the inner field was captured.
        *store.value().write() = None;

        // The previously valid projection must now read as `None` instead of
        // panicking with "called `Option::unwrap()` on a `None` value".
        let guard = inner.try_read_untracked();
        assert!(guard.is_none());
        // Forcing the (lazy) projection must not panic.
        assert!(guard.map(|g| *g).is_none());

        // The writer side is fallible in the same way.
        assert!(inner.try_write().is_none());
    }
}
