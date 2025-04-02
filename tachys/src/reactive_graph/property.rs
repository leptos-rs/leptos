use super::{ReactiveFunction, SharedReactiveFunction};
use crate::{html::property::IntoProperty, renderer::Rndr};
use reactive_graph::effect::RenderEffect;

// These do update during hydration because properties don't exist in the DOM
impl<F, V> IntoProperty for F
where
    F: ReactiveFunction<Output = V>,
    V: IntoProperty + 'static,
    V::State: 'static,
{
    type State = RenderEffect<V::State>;
    type Cloneable = SharedReactiveFunction<V>;
    type CloneableOwned = SharedReactiveFunction<V>;

    fn hydrate<const FROM_SERVER: bool>(
        mut self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State {
        let key = Rndr::intern(key);
        let key = key.to_owned();
        let el = el.to_owned();

        RenderEffect::new(move |prev| {
            let value = self.invoke();
            if let Some(mut state) = prev {
                value.rebuild(&mut state, &key);
                state
            } else {
                value.hydrate::<FROM_SERVER>(&el, &key)
            }
        })
    }

    fn build(
        mut self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State {
        let key = Rndr::intern(key);
        let key = key.to_owned();
        let el = el.to_owned();

        RenderEffect::new(move |prev| {
            let value = self.invoke();
            if let Some(mut state) = prev {
                value.rebuild(&mut state, &key);
                state
            } else {
                value.build(&el, &key)
            }
        })
    }

    fn rebuild(mut self, state: &mut Self::State, key: &str) {
        let prev_value = state.take_value();
        let key = key.to_owned();
        *state = RenderEffect::new_with_value(
            move |prev| {
                let value = self.invoke();
                if let Some(mut state) = prev {
                    value.rebuild(&mut state, &key);
                    state
                } else {
                    unreachable!()
                }
            },
            prev_value,
        );
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into_shared()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into_shared()
    }
}

macro_rules! property_reactive {
    ($name:ident, <$($gen:ident),*>, $v:ty, $( $where_clause:tt )*) =>
    {
        #[allow(deprecated)]
        impl<$($gen),*> IntoProperty for $name<$($gen),*>
        where
            $v: IntoProperty + Clone + Send + Sync + 'static,
            <$v as IntoProperty>::State: 'static,
            $($where_clause)*
        {
            type State = RenderEffect<<$v as IntoProperty>::State>;
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn hydrate<const FROM_SERVER: bool>(
                self,
                el: &crate::renderer::types::Element,
                key: &str,
            ) -> Self::State {
                (move || self.get()).hydrate::<FROM_SERVER>(el, key)
            }

            fn build(
                self,
                el: &crate::renderer::types::Element,
                key: &str,
            ) -> Self::State {
                (move || self.get()).build(el, key)
            }

            fn rebuild(self, state: &mut Self::State, key: &str) {
                (move || self.get()).rebuild(state, key)
            }

            fn into_cloneable(self) -> Self::Cloneable {
                self
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                self
            }
        }
    };
}

#[cfg(not(feature = "nightly"))]
mod stable {
    use crate::html::property::IntoProperty;
    #[allow(deprecated)]
    use reactive_graph::wrappers::read::MaybeSignal;
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        effect::RenderEffect,
        owner::Storage,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, Signal},
    };

    property_reactive!(
        RwSignal,
        <V, S>,
        V,
        RwSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    property_reactive!(
        ReadSignal,
        <V, S>,
        V,
        ReadSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    property_reactive!(
        Memo,
        <V, S>,
        V,
        Memo<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    property_reactive!(
        Signal,
        <V, S>,
        V,
        Signal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    property_reactive!(
        MaybeSignal,
        <V, S>,
        V,
        MaybeSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    property_reactive!(ArcRwSignal, <V>, V, ArcRwSignal<V>: Get<Value = V>);
    property_reactive!(ArcReadSignal, <V>, V, ArcReadSignal<V>: Get<Value = V>);
    property_reactive!(ArcMemo, <V>, V, ArcMemo<V>: Get<Value = V>);
    property_reactive!(ArcSignal, <V>, V, ArcSignal<V>: Get<Value = V>);
}

#[cfg(feature = "reactive_stores")]
mod reactive_stores {
    use crate::html::property::IntoProperty;
    #[allow(deprecated)]
    use reactive_graph::{effect::RenderEffect, owner::Storage, traits::Get};
    use reactive_stores::{
        ArcField, ArcStore, AtIndex, AtKeyed, DerefedField, Field,
        KeyedSubfield, Store, StoreField, Subfield,
    };
    use std::ops::{Deref, DerefMut, Index, IndexMut};

    property_reactive!(
        Subfield,
        <Inner, Prev, V>,
        V,
        Subfield<Inner, Prev, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
    );

    property_reactive!(
        AtKeyed,
        <Inner, Prev, K, V>,
        V,
        AtKeyed<Inner, Prev, K, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
        K: Send + Sync + std::fmt::Debug + Clone + 'static,
        for<'a> &'a V: IntoIterator,
    );

    property_reactive!(
        KeyedSubfield,
        <Inner, Prev, K, V>,
        V,
        KeyedSubfield<Inner, Prev, K, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
        K: Send + Sync + std::fmt::Debug + Clone + 'static,
        for<'a> &'a V: IntoIterator,
    );

    property_reactive!(
        DerefedField,
        <S>,
        <S::Value as Deref>::Target,
        S: Clone + StoreField + Send + Sync + 'static,
        <S as StoreField>::Value: Deref + DerefMut
    );

    property_reactive!(
        AtIndex,
        <Inner, Prev>,
        <Prev as Index<usize>>::Output,
        AtIndex<Inner, Prev>: Get<Value = Prev::Output>,
        Prev: Send + Sync + IndexMut<usize> + 'static,
        Inner: Send + Sync + Clone + 'static,
    );
    property_reactive!(
        Store,
        <V, S>,
        V,
        Store<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    property_reactive!(
        Field,
        <V, S>,
        V,
        Field<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    property_reactive!(ArcStore, <V>, V, ArcStore<V>: Get<Value = V>);
    property_reactive!(ArcField, <V>, V, ArcField<V>: Get<Value = V>);
}
