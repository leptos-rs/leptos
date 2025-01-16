use crate::{
    path::{StorePath, StorePathSegment},
    store_field::StoreField,
    KeyMap, StoreFieldTrigger,
};
use reactive_graph::{
    signal::guards::{Mapped, MappedMut},
    traits::{
        DefinedAt, IsDisposed, Notify, ReadUntracked, Track, UntrackableGuard,
        Write,
    },
};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    panic::Location,
};

/// TODO
pub trait Unbox: Sized {
    /// TODO
    fn unbox(self) -> Unboxed<Self>;
}

impl<S> Unbox for S {
    #[track_caller]
    fn unbox(self) -> Unboxed<Self> {
        Unboxed {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: self,
        }
    }
}

/// TODO
#[derive(Debug, Copy, Clone)]
pub struct Unboxed<S> {
    inner: S,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<S> StoreField for Unboxed<S>
where
    S: StoreField,
    S::Value: Deref + DerefMut,
    <S::Value as Deref>::Target: Sized + 'static,
{
    type Value = <S::Value as Deref>::Target;
    type Reader = Mapped<S::Reader, Self::Value>;
    type Writer = MappedMut<S::Writer, Self::Value>;

    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner.get_trigger(path)
    }
    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner.path()
    }
    fn reader(&self) -> Option<Self::Reader> {
        let inner = self.inner.reader()?;
        Some(Mapped::new_with_guard(inner, |n| n.deref()))
    }
    fn writer(&self) -> Option<Self::Writer> {
        let inner = self.inner.writer()?;
        Some(MappedMut::new(inner, |n| n.deref(), |n| n.deref_mut()))
    }
    #[inline(always)]
    fn keys(&self) -> Option<KeyMap> {
        self.inner.keys()
    }
}

impl<S> DefinedAt for Unboxed<S>
where
    S: StoreField,
    S::Value: Deref + DerefMut,
    <S::Value as Deref>::Target: Sized + 'static,
{
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
impl<S> IsDisposed for Unboxed<S>
where
    S: IsDisposed,
{
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}
impl<S> Notify for Unboxed<S>
where
    S: StoreField,
    S::Value: Deref + DerefMut,
    <S::Value as Deref>::Target: Sized + 'static,
{
    fn notify(&self) {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.this.notify();
        trigger.children.notify();
    }
}
impl<S> Track for Unboxed<S>
where
    S: StoreField,
    S::Value: Deref + DerefMut,
    <S::Value as Deref>::Target: Sized + 'static,
{
    fn track(&self) {
        self.track_field();
    }
}
impl<S> ReadUntracked for Unboxed<S>
where
    S: StoreField,
    S::Value: Deref + DerefMut,
    <S::Value as Deref>::Target: Sized + 'static,
{
    type Value = <Self as StoreField>::Reader;
    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.reader()
    }
}
impl<S> Write for Unboxed<S>
where
    S: StoreField,
    S::Value: Deref + DerefMut,
    <S::Value as Deref>::Target: Sized + 'static,
{
    type Value = <S::Value as Deref>::Target;

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
