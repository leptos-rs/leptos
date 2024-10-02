use crate::{StoreField, Subfield};
use reactive_graph::traits::Read;
use std::ops::Deref;

pub trait OptionStoreExt
where
    Self: StoreField<Value = Option<Self::Output>>,
{
    type Output;

    fn unwrap(self) -> Subfield<Self, Option<Self::Output>, Self::Output>;

    fn map<U>(
        self,
        map_fn: impl FnOnce(Subfield<Self, Option<Self::Output>, Self::Output>) -> U,
    ) -> Option<U>;
}

impl<T, S> OptionStoreExt for S
where
    S: StoreField<Value = Option<T>> + Read,
    <S as Read>::Value: Deref<Target = Option<T>>,
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
        if self.read().is_some() {
            Some(map_fn(self.unwrap()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as reactive_stores, Store};
    use reactive_graph::{
        effect::Effect,
        traits::{Get, Read, ReadUntracked, Set, Write},
    };
    use reactive_stores_macro::Store;
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
}
