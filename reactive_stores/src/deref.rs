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
    ops::{Deref, DerefMut},
    panic::Location,
};

/// Maps a store field that is a smart pointer to a subfield of the dereferenced inner type.
pub trait DerefField
where
    Self: StoreField,
    Self::Value: Deref + DerefMut,
    <Self::Value as Deref>::Target: Sized + 'static,
{
    /// Returns a new store field with the value mapped to the target type of dereferencing this
    /// field
    ///
    /// For example, if you have a store field with a `Box<T>`, `.deref_field()` will return a
    /// new store field containing a `T`.
    fn deref_field(self) -> DerefedField<Self>;
}

impl<S> DerefField for S
where
    S: StoreField,
    S::Value: Deref + DerefMut,
    <S::Value as Deref>::Target: Sized + 'static,
{
    #[track_caller]
    fn deref_field(self) -> DerefedField<Self> {
        DerefedField {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: self,
        }
    }
}

/// A wrapper from a store field containing a smart pointer to a store field containing the
/// dereferenced target type of that smart pointer.
#[derive(Debug, Copy, Clone)]
pub struct DerefedField<S> {
    inner: S,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<S> StoreField for DerefedField<S>
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

impl<S> DefinedAt for DerefedField<S>
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
impl<S> IsDisposed for DerefedField<S>
where
    S: IsDisposed,
{
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}
impl<S> Notify for DerefedField<S>
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
impl<S> Track for DerefedField<S>
where
    S: StoreField,
    S::Value: Deref + DerefMut,
    <S::Value as Deref>::Target: Sized + 'static,
{
    fn track(&self) {
        self.track_field();
    }
}
impl<S> ReadUntracked for DerefedField<S>
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
impl<S> Write for DerefedField<S>
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
