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

#[cfg(not(feature = "nightly"))]
mod stable {
    use crate::{html::property::IntoProperty, renderer::Rndr};
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
    #[cfg(feature = "reactive_stores")]
    use {
        reactive_stores::{
            ArcField, ArcStore, AtIndex, AtKeyed, DerefedField, Field,
            KeyedSubfield, Store, StoreField, Subfield,
        },
        std::ops::{Deref, DerefMut, Index, IndexMut},
    };

    macro_rules! property_signal {
        ($sig:ident) => {
            impl<V> IntoProperty for $sig<V>
            where
                $sig<V>: Get<Value = V>,
                V: IntoProperty + Send + Sync + Clone + 'static,
                V::State: 'static,
            {
                type State = RenderEffect<Option<V::State>>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &crate::renderer::types::Element,
                    key: &str,
                ) -> Self::State {
                    let key = Rndr::intern(key);
                    let key = key.to_owned();
                    let el = el.to_owned();

                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        // Outer Some means there was a previous state
                        // Inner Some means the previous state was valid
                        // (i.e., the signal was successfully accessed)
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state, &key);
                                Some(state)
                            }
                            (None, Some(value)) => {
                                Some(value.hydrate::<FROM_SERVER>(&el, &key))
                            }
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => {
                                Some(value.hydrate::<FROM_SERVER>(&el, &key))
                            }
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn build(
                    self,
                    el: &crate::renderer::types::Element,
                    key: &str,
                ) -> Self::State {
                    let key = Rndr::intern(key);
                    let key = key.to_owned();
                    let el = el.to_owned();

                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state, &key);
                                Some(state)
                            }
                            (None, Some(value)) => Some(value.build(&el, &key)),
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => {
                                Some(value.build(&el, &key))
                            }
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn rebuild(self, state: &mut Self::State, key: &str) {
                    let prev_value = state.take_value();
                    let key = key.to_owned();
                    *state = RenderEffect::new_with_value(
                        move |prev| {
                            let value = self.try_get();
                            match (prev, value) {
                                (Some(Some(mut state)), Some(value)) => {
                                    value.rebuild(&mut state, &key);
                                    Some(state)
                                }
                                (Some(Some(state)), None) => Some(state),
                                (Some(None), Some(_)) => None,
                                (Some(None), None) => None,
                                (None, Some(_)) => None, // unreachable!()
                                (None, None) => None,    // unreachable!()
                            }
                        },
                        prev_value,
                    );
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

    macro_rules! property_signal_arena {
        ($sig:ident) => {
            #[allow(deprecated)]
            impl<V, S> IntoProperty for $sig<V, S>
            where
                $sig<V, S>: Get<Value = V>,
                S: Send + Sync + 'static,
                S: Storage<V> + Storage<Option<V>>,
                V: IntoProperty + Send + Sync + Clone + 'static,
                V::State: 'static,
            {
                type State = RenderEffect<Option<V::State>>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &crate::renderer::types::Element,
                    key: &str,
                ) -> Self::State {
                    let key = Rndr::intern(key);
                    let key = key.to_owned();
                    let el = el.to_owned();

                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        // Outer Some means there was a previous state
                        // Inner Some means the previous state was valid
                        // (i.e., the signal was successfully accessed)
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state, &key);
                                Some(state)
                            }
                            (None, Some(value)) => {
                                Some(value.hydrate::<FROM_SERVER>(&el, &key))
                            }
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => {
                                Some(value.hydrate::<FROM_SERVER>(&el, &key))
                            }
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }
                fn build(
                    self,
                    el: &crate::renderer::types::Element,
                    key: &str,
                ) -> Self::State {
                    let key = Rndr::intern(key);
                    let key = key.to_owned();
                    let el = el.to_owned();

                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state, &key);
                                Some(state)
                            }
                            (None, Some(value)) => Some(value.build(&el, &key)),
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => {
                                Some(value.build(&el, &key))
                            }
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn rebuild(self, state: &mut Self::State, key: &str) {
                    let prev_value = state.take_value();
                    let key = key.to_owned();
                    *state = RenderEffect::new_with_value(
                        move |prev| {
                            let value = self.try_get();
                            match (prev, value) {
                                (Some(Some(mut state)), Some(value)) => {
                                    value.rebuild(&mut state, &key);
                                    Some(state)
                                }
                                (Some(Some(state)), None) => Some(state),
                                (Some(None), Some(_)) => None,
                                (Some(None), None) => None,
                                (None, Some(_)) => None, // unreachable!()
                                (None, None) => None,    // unreachable!()
                            }
                        },
                        prev_value,
                    );
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

    #[cfg(feature = "reactive_stores")]
    macro_rules! property_store_field {
        ($name:ident, <$($gen:ident),*>, $v:ty, $( $where_clause:tt )*) =>
        {
            impl<$($gen),*> IntoProperty for $name<$($gen),*>
            where
                $v: IntoProperty + Send + Sync + Clone + 'static,
                <$v as IntoProperty>::State: 'static,
                $($where_clause)*
            {
                type State = RenderEffect<Option<<$v as IntoProperty>::State>>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &crate::renderer::types::Element,
                    key: &str,
                ) -> Self::State {
                    let key = Rndr::intern(key);
                    let key = key.to_owned();
                    let el = el.to_owned();

                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        // Outer Some means there was a previous state
                        // Inner Some means the previous state was valid
                        // (i.e., the signal was successfully accessed)
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state, &key);
                                Some(state)
                            }
                            (None, Some(value)) => {
                                Some(value.hydrate::<FROM_SERVER>(&el, &key))
                            }
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => {
                                Some(value.hydrate::<FROM_SERVER>(&el, &key))
                            }
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn build(
                    self,
                    el: &crate::renderer::types::Element,
                    key: &str,
                ) -> Self::State {
                    let key = Rndr::intern(key);
                    let key = key.to_owned();
                    let el = el.to_owned();

                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state, &key);
                                Some(state)
                            }
                            (None, Some(value)) => Some(value.build(&el, &key)),
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => Some(value.build(&el, &key)),
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn rebuild(self, state: &mut Self::State, key: &str) {
                    let prev_value = state.take_value();
                    let key = key.to_owned();
                    *state = RenderEffect::new_with_value(
                        move |prev| {
                            let value = self.try_get();
                            match (prev, value) {
                                (Some(Some(mut state)), Some(value)) => {
                                    value.rebuild(&mut state, &key);
                                    Some(state)
                                }
                                (Some(Some(state)), None) => Some(state),
                                (Some(None), Some(_)) => None,
                                (Some(None), None) => None,
                                (None, Some(_)) => None, // unreachable!()
                                (None, None) => None,    // unreachable!()
                            }
                        },
                        prev_value,
                    );
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

    #[cfg(feature = "reactive_stores")]
    property_store_field!(
        Subfield,
        <Inner, Prev, V>,
        V,
        Subfield<Inner, Prev, V>: Get<Value = V>,
        Prev: 'static,
        Inner: Clone + 'static,
    );

    #[cfg(feature = "reactive_stores")]
    property_store_field!(
        AtKeyed,
        <Inner, Prev, K, V>,
        V,
        AtKeyed<Inner, Prev, K, V>: Get<Value = V>,
        Prev: 'static,
        Inner: Clone + 'static,
        K: std::fmt::Debug + Clone + 'static,
        for<'a> &'a V: IntoIterator,
    );

    #[cfg(feature = "reactive_stores")]
    property_store_field!(
        KeyedSubfield,
        <Inner, Prev, K, V>,
        V,
        KeyedSubfield<Inner, Prev, K, V>: Get<Value = V>,
        Prev: 'static,
        Inner: Clone + 'static,
        K: std::fmt::Debug + Clone + 'static,
        for<'a> &'a V: IntoIterator,
    );

    #[cfg(feature = "reactive_stores")]
    property_store_field!(
        DerefedField,
        <S>,
        <S::Value as Deref>::Target,
        S: Clone + StoreField + Send + Sync + 'static,
        <S as StoreField>::Value: Deref + DerefMut
    );
    
    #[cfg(feature = "reactive_stores")]
    property_store_field!(
        AtIndex,
        <Inner, Prev>,
        <Prev as Index<usize>>::Output,
        Prev: IndexMut<usize> + 'static,
        AtIndex<Inner, Prev>: Get<Value = Prev::Output>,
        Prev: 'static,
        Inner: Clone + 'static,
    );

    #[cfg(feature = "reactive_stores")]
    property_signal_arena!(Store);
    #[cfg(feature = "reactive_stores")]
    property_signal_arena!(Field);
    property_signal_arena!(RwSignal);
    property_signal_arena!(ReadSignal);
    property_signal_arena!(Memo);
    property_signal_arena!(Signal);
    property_signal_arena!(MaybeSignal);
    #[cfg(feature = "reactive_stores")]
    property_signal!(ArcStore);
    #[cfg(feature = "reactive_stores")]
    property_signal!(ArcField);
    property_signal!(ArcRwSignal);
    property_signal!(ArcReadSignal);
    property_signal!(ArcMemo);
    property_signal!(ArcSignal);
}
