use crate::{
    path::{StorePath, StorePathSegment},
    store_field::StoreField,
    KeyMap, StoreFieldTrigger,
};
use reactive_graph::{
    signal::{
        guards::{Mapped, MappedMut, WriteGuard},
        ArcTrigger,
    },
    traits::{
        DefinedAt, Get as _, IsDisposed, Notify, ReadUntracked, Track,
        UntrackableGuard, Write,
    },
    wrappers::read::Signal,
};
use std::{iter, marker::PhantomData, ops::DerefMut, panic::Location};

/// Accesses a single field of a reactive structure.
#[derive(Debug)]
pub struct Subfield<Inner, Prev, T> {
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    defined_at: &'static Location<'static>,
    path_segment: StorePathSegment,
    inner: Inner,
    read: fn(&Prev) -> &T,
    write: fn(&mut Prev) -> &mut T,
    ty: PhantomData<T>,
}

impl<Inner, Prev, T> Clone for Subfield<Inner, Prev, T>
where
    Inner: Clone,
{
    fn clone(&self) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            path_segment: self.path_segment,
            inner: self.inner.clone(),
            read: self.read,
            write: self.write,
            ty: self.ty,
        }
    }
}

impl<Inner, Prev, T> Copy for Subfield<Inner, Prev, T> where Inner: Copy {}

impl<Inner, Prev, T> Subfield<Inner, Prev, T> {
    /// Creates an accessor for a single field of the inner structure.
    #[track_caller]
    pub fn new(
        inner: Inner,
        path_segment: StorePathSegment,
        read: fn(&Prev) -> &T,
        write: fn(&mut Prev) -> &mut T,
    ) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            inner,
            path_segment,
            read,
            write,
            ty: PhantomData,
        }
    }
}

impl<Inner, Prev, T> StoreField for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
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

    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner.get_trigger(path)
    }

    fn reader(&self) -> Option<Self::Reader> {
        let inner = self.inner.reader()?;
        Some(Mapped::new_with_guard(inner, self.read))
    }

    fn writer(&self) -> Option<Self::Writer> {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        let mut parent = self.inner.writer()?;
        parent.untrack();

        let mut full_path = self.path().into_iter().collect::<StorePath>();
        full_path.pop();
        let mut triggers = Vec::with_capacity(full_path.len());
        triggers.push(trigger.this.clone());
        loop {
            let inner = self.get_trigger(full_path.clone());
            triggers.push(inner.children.clone());
            if full_path.is_empty() {
                break;
            }
            full_path.pop();
        }
        let guard = WriteGuard::new(triggers, parent);

        Some(MappedMut::new(guard, self.read, self.write))
    }

    #[inline(always)]
    fn keys(&self) -> Option<KeyMap> {
        self.inner.keys()
    }

    #[track_caller]
    fn track_field(&self) {
        let mut full_path = self.path().into_iter().collect::<StorePath>();
        // tracks `this` for all ancestors: i.e., it will track any change that is made
        // directly to one of its ancestors, but not a change made to a *child* of an ancestor
        // (which would end up with every subfield tracking its own siblings, because they are
        // children of its parent)
        loop {
            let inner = self.get_trigger(full_path.clone());
            inner.this.track();
            if full_path.is_empty() {
                break;
            }
            full_path.pop();
        }
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.this.track();
        trigger.children.track();
    }
}

impl<Inner, Prev, T> DefinedAt for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Value = Prev>,
{
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

impl<Inner, Prev, T> IsDisposed for Subfield<Inner, Prev, T>
where
    Inner: IsDisposed,
{
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}

impl<Inner, Prev, T> Notify for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
{
    #[track_caller]
    fn notify(&self) {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.this.notify();
        trigger.children.notify();
    }
}

impl<Inner, Prev, T> Track for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Value = Prev> + Track + 'static,
    Prev: 'static,
    T: 'static,
{
    #[track_caller]
    fn track(&self) {
        self.track_field();
    }
}

impl<Inner, Prev, T> ReadUntracked for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
{
    type Value = <Self as StoreField>::Reader;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.reader()
    }
}

impl<Inner, Prev, T> Write for Subfield<Inner, Prev, T>
where
    T: 'static,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
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

impl<Inner, Prev, T> From<Subfield<Inner, Prev, T>> for Signal<T>
where
    Inner: StoreField<Value = Prev> + Track + Send + Sync + 'static,
    Prev: 'static,
    T: Send + Sync + Clone + 'static,
{
    fn from(subfield: Subfield<Inner, Prev, T>) -> Self {
        Signal::derive(move || subfield.get())
    }
}
