use crate::{StoreField, Subfield};
use reactive_graph::traits::{FlattenOptionRefOption, Read, ReadUntracked};
use std::ops::Deref;

/// Extends optional store fields, with the ability to unwrap or map over them.
pub trait OptionStoreExt
where
    Self: StoreField<Value = Option<Self::Output>>,
{
    /// The inner type of the `Option<_>` this field holds.
    type Output;

    /// Provides access to the inner value, as a subfield, unwrapping the outer value.
    fn unwrap(self) -> Subfield<Self, Option<Self::Output>, Self::Output>;

    /// Inverts a subfield of an `Option` to an `Option` of a subfield.
    fn invert(
        self,
    ) -> Option<Subfield<Self, Option<Self::Output>, Self::Output>> {
        self.map(|f| f)
    }

    /// Reactively maps over the field.
    ///
    /// This returns `None` if the subfield is currently `None`,
    /// and a new store subfield with the inner value if it is `Some`. This can be used in some  
    /// other reactive context, which will cause it to re-run if the field toggles betwen `None`
    /// and `Some(_)`.
    fn map<U>(
        self,
        map_fn: impl FnOnce(Subfield<Self, Option<Self::Output>, Self::Output>) -> U,
    ) -> Option<U>;

    /// Unreactively maps over the field.
    ///
    /// This returns `None` if the subfield is currently `None`,
    /// and a new store subfield with the inner value if it is `Some`. This is an unreactive variant of
    /// `[OptionStoreExt::map]`, and will not cause the reactive context to re-run if the field changes.
    fn map_untracked<U>(
        self,
        map_fn: impl FnOnce(Subfield<Self, Option<Self::Output>, Self::Output>) -> U,
    ) -> Option<U>;
}

impl<T, S> OptionStoreExt for S
where
    S: StoreField<Value = Option<T>> + Read + ReadUntracked,
    <S as Read>::Value: Deref<Target = Option<T>>,
    <S as ReadUntracked>::Value: Deref<Target = Option<T>>,
{
    type Output = T;

    fn unwrap(self) -> Subfield<Self, Option<Self::Output>, Self::Output> {
        Subfield::new(
            self,
            0.into(),
            |t| t.as_ref().unwrap(),
            |t| t.as_mut().unwrap(),
        )
    }

    fn map<U>(
        self,
        map_fn: impl FnOnce(Subfield<S, Option<T>, T>) -> U,
    ) -> Option<U> {
        if self.try_read().as_deref().flatten().is_some() {
            Some(map_fn(self.unwrap()))
        } else {
            None
        }
    }

    fn map_untracked<U>(
        self,
        map_fn: impl FnOnce(Subfield<S, Option<T>, T>) -> U,
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
    use reactive_graph::{
        effect::Effect,
        traits::{Get, Read, ReadUntracked, Set, Write},
    };
    use reactive_stores_macro::Patch;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    pub async fn tick() {
        tokio::time::sleep(std::time::Duration::from_micros(1)).await;
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

        _ = any_spawner::Executor::init_tokio();

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

                println!(
                    "  value = {:?}",
                    store.inner().map(|inner| inner.first().get())
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

        store.patch(Outer {
            inner: Some(Inner {
                first: "A".to_string(),
                second: "C".to_string(),
            }),
        });

        tick().await;
        assert_eq!(parent_count.load(Ordering::Relaxed), 1);
        assert_eq!(inner_first_count.load(Ordering::Relaxed), 1);
        assert_eq!(inner_second_count.load(Ordering::Relaxed), 2);

        store.patch(Outer { inner: None });

        tick().await;
        assert_eq!(parent_count.load(Ordering::Relaxed), 2);
        assert_eq!(inner_first_count.load(Ordering::Relaxed), 2);
        assert_eq!(inner_second_count.load(Ordering::Relaxed), 3);

        store.patch(Outer {
            inner: Some(Inner {
                first: "A".to_string(),
                second: "B".to_string(),
            }),
        });

        tick().await;
        assert_eq!(parent_count.load(Ordering::Relaxed), 3);
        assert_eq!(inner_first_count.load(Ordering::Relaxed), 3);
        assert_eq!(inner_second_count.load(Ordering::Relaxed), 4);
    }
}
