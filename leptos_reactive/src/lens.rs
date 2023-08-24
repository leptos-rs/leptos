// Paths are fn pointers. They can be safely cast to usize but not back.

use crate::{
    create_trigger, runtime::FxIndexMap, store_value, Signal, StoredValue,
    Trigger,
};
use std::{any::Any, fmt::Debug};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct PathId(usize);

pub struct StoreInner<T>
where
    T: 'static,
{
    value: StoredValue<T>,
    lenses: FxIndexMap<PathId, Trigger>,
}

impl<T: Debug> Debug for StoreInner<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoreInner")
            .field("value", &self.value)
            .finish()
    }
}

impl<T> StoreInner<T>
where
    T: 'static,
{
    pub fn new(value: T) -> Self {
        Self {
            value: store_value(value),
            lenses: Default::default(),
        }
    }

    pub fn try_update<U, V>(
        &mut self,
        lens: fn(&mut T) -> &mut U,
        setter: impl FnOnce(&mut U) -> V + 'static,
    ) -> Option<V> {
        // get or create trigger
        let id = PathId(lens as usize);
        let trigger = *self.lenses.entry(id).or_default();

        // run update function
        let result = self.value.try_update_value(|value| {
            let zone = lens(value);
            setter(zone)
        })?;

        // notify trigger
        if trigger.try_notify() {
            Some(result)
        } else {
            None
        }
    }

    pub fn try_read<U: 'static, V>(
        &mut self,
        lens: fn(&mut T) -> &mut U,
        getter: impl Fn(&U) -> V + 'static,
    ) -> Signal<Option<V>> {
        // get or create trigger
        let id = PathId(lens as usize);
        let trigger = *self.lenses.entry(id).or_default();

        let value = self.value;

        // run update function
        Signal::derive(move || {
            trigger.track();
            value.try_update_value(|value| {
                let zone = lens(value);
                getter(&*zone)
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::StoreInner;
    use crate::{
        create_effect, create_runtime, SignalGet, SignalGetUntracked,
        SignalWith, SignalWithUntracked,
    };
    use std::{cell::Cell, rc::Rc};
    #[derive(Default)]
    struct SomeComplexType {
        a: NonCloneableUsize,
        b: NonCloneableString,
    }

    #[derive(Default, Debug, PartialEq, Eq)]
    struct NonCloneableUsize(usize);

    #[derive(Default, Debug, PartialEq, Eq)]
    struct NonCloneableString(String);

    #[test]
    pub fn create_lens() {
        let rt = create_runtime();

        // create the store
        let mut store = StoreInner::new(SomeComplexType::default());

        // create two signal lenses
        fn lens_a(store: &mut SomeComplexType) -> &mut NonCloneableUsize {
            &mut store.a
        }
        fn lens_b(store: &mut SomeComplexType) -> &mut NonCloneableString {
            &mut store.b
        }
        let read_a = store.try_read(lens_a, |a| a.0);
        read_a.with_untracked(|val| assert_eq!(val, &Some(0)));
        assert_eq!(read_a.get_untracked(), Some(0));
        let read_b = store.try_read(lens_b, |b| b.0.len());
        assert_eq!(read_b.get_untracked(), Some(0));

        // track how many times each variable notifies
        let reads_on_a = Rc::new(Cell::new(0));
        let reads_on_b = Rc::new(Cell::new(0));
        create_effect({
            let reads_on_a = Rc::clone(&reads_on_a);
            move |_| {
                read_a.track();
                reads_on_a.set(reads_on_a.get() + 1);
            }
        });
        create_effect({
            let reads_on_b = Rc::clone(&reads_on_b);
            move |_| {
                read_b.track();
                reads_on_b.set(reads_on_b.get() + 1);
            }
        });
        assert_eq!(reads_on_a.get(), 1);
        assert_eq!(reads_on_b.get(), 1);

        // update each one once
        store.try_update(lens_a, |a| *a = NonCloneableUsize(42));
        assert_eq!(read_a.get_untracked(), Some(42));

        store.try_update(lens_b, |b| b.0.push_str("hello, world!"));
        assert_eq!(read_b.get_untracked(), Some(13));

        // each effect has only run once
        // none of the values has been cloned (they can't)
        assert_eq!(reads_on_a.get(), 2);
        assert_eq!(reads_on_b.get(), 2);

        rt.dispose();
    }
}
