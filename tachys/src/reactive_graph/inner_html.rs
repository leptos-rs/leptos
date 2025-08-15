use super::{ReactiveFunction, SharedReactiveFunction};
use crate::html::element::InnerHtmlValue;
use reactive_graph::effect::RenderEffect;

impl<F, V> InnerHtmlValue for F
where
    F: ReactiveFunction<Output = V>,
    V: InnerHtmlValue + 'static,
    V::State: 'static,
{
    type AsyncOutput = V::AsyncOutput;
    type State = RenderEffect<V::State>;
    type Cloneable = SharedReactiveFunction<V>;
    type CloneableOwned = SharedReactiveFunction<V>;

    fn html_len(&self) -> usize {
        0
    }

    fn to_html(mut self, buf: &mut String) {
        let value = self.invoke();
        value.to_html(buf);
    }

    fn to_template(_buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        mut self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let el = el.to_owned();
        RenderEffect::new(move |prev| {
            let value = self.invoke();
            if let Some(mut state) = prev {
                value.rebuild(&mut state);
                state
            } else {
                value.hydrate::<FROM_SERVER>(&el)
            }
        })
    }

    fn build(mut self, el: &crate::renderer::types::Element) -> Self::State {
        let el = el.to_owned();
        RenderEffect::new(move |prev| {
            let value = self.invoke();
            if let Some(mut state) = prev {
                value.rebuild(&mut state);
                state
            } else {
                value.build(&el)
            }
        })
    }

    fn rebuild(mut self, state: &mut Self::State) {
        let prev_value = state.take_value();
        *state = RenderEffect::new_with_value(
            move |prev| {
                let value = self.invoke();
                if let Some(mut state) = prev {
                    value.rebuild(&mut state);
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

    fn dry_resolve(&mut self) {
        self.invoke();
    }

    async fn resolve(mut self) -> Self::AsyncOutput {
        self.invoke().resolve().await
    }
}

macro_rules! inner_html_reactive {
    ($name:ident, <$($gen:ident),*>, $v:ty, $( $where_clause:tt )*) =>
    {
        #[allow(deprecated)]
        impl<$($gen),*> InnerHtmlValue for $name<$($gen),*>
        where
            $v: InnerHtmlValue + Clone + Send + Sync + 'static,
            <$v as InnerHtmlValue>::State: 'static,
            $($where_clause)*
        {
            type AsyncOutput = Self;
            type State = RenderEffect<<$v as InnerHtmlValue>::State>;
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn html_len(&self) -> usize {
                0
            }

            fn to_html(self, buf: &mut String) {
                let value = self.get();
                value.to_html(buf);
            }

            fn to_template(_buf: &mut String) {}

            fn hydrate<const FROM_SERVER: bool>(
                self,
                el: &crate::renderer::types::Element,
            ) -> Self::State {
                (move || self.get()).hydrate::<FROM_SERVER>(el)
            }

            fn build(
                self,
                el: &crate::renderer::types::Element,
            ) -> Self::State {
                (move || self.get()).build(el)
            }

            fn rebuild(self, state: &mut Self::State) {
                (move || self.get()).rebuild(state)
            }

            fn into_cloneable(self) -> Self::Cloneable {
                self
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                self
            }

            fn dry_resolve(&mut self) {}

            async fn resolve(self) -> Self::AsyncOutput {
                self
            }
        }
    };
}

#[cfg(not(feature = "nightly"))]
mod stable {
    use crate::html::element::InnerHtmlValue;
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

    inner_html_reactive!(
        RwSignal,
        <V, S>,
        V,
        RwSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    inner_html_reactive!(
        ReadSignal,
        <V, S>,
        V,
        ReadSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    inner_html_reactive!(
        Memo,
        <V, S>,
        V,
        Memo<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    inner_html_reactive!(
        Signal,
        <V, S>,
        V,
        Signal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    inner_html_reactive!(
        MaybeSignal,
        <V, S>,
        V,
        MaybeSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    inner_html_reactive!(ArcRwSignal, <V>, V, ArcRwSignal<V>: Get<Value = V>);
    inner_html_reactive!(ArcReadSignal, <V>, V, ArcReadSignal<V>: Get<Value = V>);
    inner_html_reactive!(ArcMemo, <V>, V, ArcMemo<V>: Get<Value = V>);
    inner_html_reactive!(ArcSignal, <V>, V, ArcSignal<V>: Get<Value = V>);
}

#[cfg(feature = "reactive_stores")]
mod reactive_stores {
    use crate::html::element::InnerHtmlValue;
    #[allow(deprecated)]
    use reactive_graph::{effect::RenderEffect, owner::Storage, traits::Get};
    use reactive_stores::{
        ArcField, ArcStore, AtIndex, AtKeyed, DerefedField, Field,
        KeyedSubfield, Store, StoreField, Subfield,
    };
    use std::ops::{Deref, DerefMut, Index, IndexMut};

    inner_html_reactive!(
        Subfield,
        <Inner, Prev, V>,
        V,
        Subfield<Inner, Prev, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
    );

    inner_html_reactive!(
        AtKeyed,
        <Inner, Prev, K, V>,
        V,
        AtKeyed<Inner, Prev, K, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
        K: Send + Sync + std::fmt::Debug + Clone + 'static,
        for<'a> &'a V: IntoIterator,
    );

    inner_html_reactive!(
        KeyedSubfield,
        <Inner, Prev, K, V>,
        V,
        KeyedSubfield<Inner, Prev, K, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
        K: Send + Sync + std::fmt::Debug + Clone + 'static,
        for<'a> &'a V: IntoIterator,
    );

    inner_html_reactive!(
        DerefedField,
        <S>,
        <S::Value as Deref>::Target,
        S: Clone + StoreField + Send + Sync + 'static,
        <S as StoreField>::Value: Deref + DerefMut
    );

    inner_html_reactive!(
        AtIndex,
        <Inner, Prev>,
        <Prev as Index<usize>>::Output,
        AtIndex<Inner, Prev>: Get<Value = Prev::Output>,
        Prev: Send + Sync + IndexMut<usize> + 'static,
        Inner: Send + Sync + Clone + 'static,
    );
    inner_html_reactive!(
        Store,
        <V, S>,
        V,
        Store<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    inner_html_reactive!(
        Field,
        <V, S>,
        V,
        Field<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    inner_html_reactive!(ArcStore, <V>, V, ArcStore<V>: Get<Value = V>);
    inner_html_reactive!(ArcField, <V>, V, ArcField<V>: Get<Value = V>);
}
