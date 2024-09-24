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
    use crate::html::property::IntoProperty;
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        effect::RenderEffect,
        owner::Storage,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, MaybeSignal, Signal},
    };

    macro_rules! property_signal {
        ($sig:ident) => {
            impl<V> IntoProperty for $sig<V>
            where
                $sig<V>: Get<Value = V>,
                V: IntoProperty + Send + Sync + Clone + 'static,
                V::State: 'static,
            {
                type State = RenderEffect<V::State>;
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

    macro_rules! property_signal_arena {
        ($sig:ident) => {
            impl<V, S> IntoProperty for $sig<V, S>
            where
                $sig<V, S>: Get<Value = V>,
                S: Send + Sync + 'static,
                S: Storage<V> + Storage<Option<V>>,
                V: IntoProperty + Send + Sync + Clone + 'static,
                V::State: 'static,
            {
                type State = RenderEffect<V::State>;
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

    property_signal_arena!(RwSignal);
    property_signal_arena!(ReadSignal);
    property_signal_arena!(Memo);
    property_signal_arena!(Signal);
    property_signal_arena!(MaybeSignal);
    property_signal!(ArcRwSignal);
    property_signal!(ArcReadSignal);
    property_signal!(ArcMemo);
    property_signal!(ArcSignal);
}
