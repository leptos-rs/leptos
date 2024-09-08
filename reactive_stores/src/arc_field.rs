use crate::{
    path::{StorePath, StorePathSegment},
    AtIndex, StoreField, Subfield,
};
use reactive_graph::{
    owner::{LocalStorage, SyncStorage},
    signal::ArcTrigger,
    traits::{
        DefinedAt, Get, IsDisposed, ReadUntracked, Track, Trigger,
        UntrackableGuard,
    },
    wrappers::read::{MaybeSignal, Signal},
};
use std::{
    ops::{Deref, DerefMut, IndexMut},
    panic::Location,
    sync::Arc,
};

pub struct ArcField<T>
where
    T: 'static,
{
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    path: StorePath,
    trigger: ArcTrigger,
    get_trigger: Arc<dyn Fn(StorePath) -> ArcTrigger + Send + Sync>,
    read: Arc<dyn Fn() -> Option<StoreFieldReader<T>> + Send + Sync>,
    write: Arc<dyn Fn() -> Option<StoreFieldWriter<T>> + Send + Sync>,
}

pub struct StoreFieldReader<T>(Box<dyn Deref<Target = T>>);

impl<T> StoreFieldReader<T> {
    pub fn new(inner: impl Deref<Target = T> + 'static) -> Self {
        Self(Box::new(inner))
    }
}

impl<T> Deref for StoreFieldReader<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

pub struct StoreFieldWriter<T>(Box<dyn UntrackableGuard<Target = T>>);

impl<T> StoreFieldWriter<T> {
    pub fn new(inner: impl UntrackableGuard<Target = T> + 'static) -> Self {
        Self(Box::new(inner))
    }
}

impl<T> Deref for StoreFieldWriter<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T> DerefMut for StoreFieldWriter<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

impl<T> UntrackableGuard for StoreFieldWriter<T> {
    fn untrack(&mut self) {
        self.0.untrack();
    }
}

impl<T> StoreField for ArcField<T> {
    type Value = T;
    type Reader = StoreFieldReader<T>;
    type Writer = StoreFieldWriter<T>;
    type UntrackedWriter = StoreFieldWriter<T>;

    fn get_trigger(&self, path: StorePath) -> ArcTrigger {
        (self.get_trigger)(path)
    }

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.path.clone()
    }

    fn reader(&self) -> Option<Self::Reader> {
        (self.read)().map(StoreFieldReader::new)
    }

    fn writer(&self) -> Option<Self::Writer> {
        (self.write)().map(StoreFieldWriter::new)
    }

    fn untracked_writer(&self) -> Option<Self::UntrackedWriter> {
        let mut writer = (self.write)().map(StoreFieldWriter::new)?;
        writer.untrack();
        Some(writer)
    }
}

impl<Inner, Prev, T> From<Subfield<Inner, Prev, T>> for ArcField<T>
where
    T: Send + Sync,
    Subfield<Inner, Prev, T>: Clone,
    Inner: StoreField<Value = Prev> + Send + Sync + 'static,
    Prev: 'static,
{
    #[track_caller]
    fn from(value: Subfield<Inner, Prev, T>) -> Self {
        ArcField {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            path: value.path().into_iter().collect(),
            trigger: value.get_trigger(value.path().into_iter().collect()),
            get_trigger: Arc::new({
                let value = value.clone();
                move |path| value.get_trigger(path)
            }),
            read: Arc::new({
                let value = value.clone();
                move || value.reader().map(StoreFieldReader::new)
            }),
            write: Arc::new({
                let value = value.clone();
                move || value.writer().map(StoreFieldWriter::new)
            }),
        }
    }
}

impl<Inner, Prev> From<AtIndex<Inner, Prev>> for ArcField<Prev::Output>
where
    AtIndex<Inner, Prev>: Clone,
    Inner: StoreField<Value = Prev> + Send + Sync + 'static,
    Prev: IndexMut<usize> + Send + Sync + 'static,
    Prev::Output: Sized + Send + Sync,
{
    #[track_caller]
    fn from(value: AtIndex<Inner, Prev>) -> Self {
        ArcField {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            path: value.path().into_iter().collect(),
            trigger: value.get_trigger(value.path().into_iter().collect()),
            get_trigger: Arc::new({
                let value = value.clone();
                move |path| value.get_trigger(path)
            }),
            read: Arc::new({
                let value = value.clone();
                move || value.reader().map(StoreFieldReader::new)
            }),
            write: Arc::new({
                let value = value.clone();
                move || value.writer().map(StoreFieldWriter::new)
            }),
        }
    }
}

impl<T> Clone for ArcField<T> {
    fn clone(&self) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
            path: self.path.clone(),
            trigger: self.trigger.clone(),
            get_trigger: Arc::clone(&self.get_trigger),
            read: Arc::clone(&self.read),
            write: Arc::clone(&self.write),
        }
    }
}

impl<T> DefinedAt for ArcField<T> {
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(debug_assertions)]
        {
            Some(self.defined_at)
        }
        #[cfg(not(debug_assertions))]
        {
            None
        }
    }
}

impl<T> Trigger for ArcField<T> {
    fn trigger(&self) {
        self.trigger.trigger();
    }
}

impl<T> Track for ArcField<T> {
    fn track(&self) {
        self.trigger.track();
    }
}

impl<T> ReadUntracked for ArcField<T> {
    type Value = StoreFieldReader<T>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        (self.read)()
    }
}

impl<T> IsDisposed for ArcField<T> {
    fn is_disposed(&self) -> bool {
        false
    }
}

impl<T> From<ArcField<T>> for Signal<T, LocalStorage>
where
    T: Clone + 'static,
{
    fn from(value: ArcField<T>) -> Self {
        Self::derive_local(move || value.get())
    }
}

impl<T> From<ArcField<T>> for Signal<T, SyncStorage>
where
    T: Clone + Send + Sync + 'static,
{
    fn from(value: ArcField<T>) -> Self {
        Self::derive(move || value.get())
    }
}

impl<T> From<ArcField<T>> for MaybeSignal<T, LocalStorage>
where
    T: Clone + 'static,
{
    fn from(value: ArcField<T>) -> Self {
        Self::Dynamic(value.into())
    }
}

impl<T> From<ArcField<T>> for MaybeSignal<T, SyncStorage>
where
    T: Clone + Send + Sync + 'static,
{
    fn from(value: ArcField<T>) -> Self {
        Self::Dynamic(value.into())
    }
}
