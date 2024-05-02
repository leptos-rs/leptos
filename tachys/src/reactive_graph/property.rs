use super::{ReactiveFunction, SharedReactiveFunction};
use crate::{
    html::property::IntoProperty,
    renderer::{DomRenderer, Renderer},
};
use reactive_graph::effect::RenderEffect;

// These do update during hydration because properties don't exist in the DOM
impl<F, V, R> IntoProperty<R> for F
where
    F: ReactiveFunction<Output = V>,
    V: IntoProperty<R> + 'static,
    V::State: 'static,
    R: DomRenderer,
{
    type State = RenderEffect<V::State>;
    type Cloneable = SharedReactiveFunction<V>;

    fn hydrate<const FROM_SERVER: bool>(
        mut self,
        el: &<R as Renderer>::Element,
        key: &str,
    ) -> Self::State {
        let key = R::intern(key);
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
        el: &<R as Renderer>::Element,
        key: &str,
    ) -> Self::State {
        let key = R::intern(key);
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

    fn rebuild(self, _state: &mut Self::State, _key: &str) {
        // TODO rebuild
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into_shared()
    }
}

#[cfg(not(feature = "nightly"))]
mod stable {
    use crate::{
        html::property::IntoProperty,
        renderer::{DomRenderer, Renderer},
    };
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        effect::RenderEffect,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, Signal},
    };

    macro_rules! property_signal {
        ($sig:ident) => {
            impl<V, R> IntoProperty<R> for $sig<V>
            where
                V: IntoProperty<R> + Send + Sync + Clone + 'static,
                V::State: 'static,
                R: DomRenderer,
            {
                type State = RenderEffect<V::State>;

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &<R as Renderer>::Element,
                    key: &str,
                ) -> Self::State {
                    (move || self.get()).hydrate::<FROM_SERVER>(el, key)
                }

                fn build(
                    self,
                    el: &<R as Renderer>::Element,
                    key: &str,
                ) -> Self::State {
                    (move || self.get()).build(el, key)
                }

                fn rebuild(self, _state: &mut Self::State, _key: &str) {
                    // TODO rebuild
                }
            }
        };
    }

    macro_rules! property_signal_unsend {
        ($sig:ident) => {
            impl<V, R> IntoProperty<R> for $sig<V>
            where
                V: IntoProperty<R> + Clone + 'static,
                V::State: 'static,
                R: DomRenderer,
            {
                type State = RenderEffect<V::State>;

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &<R as Renderer>::Element,
                    key: &str,
                ) -> Self::State {
                    (move || self.get()).hydrate::<FROM_SERVER>(el, key)
                }

                fn build(
                    self,
                    el: &<R as Renderer>::Element,
                    key: &str,
                ) -> Self::State {
                    (move || self.get()).build(el, key)
                }

                fn rebuild(self, _state: &mut Self::State, _key: &str) {
                    // TODO rebuild
                }
            }
        };
    }

    property_signal!(RwSignal);
    property_signal!(ReadSignal);
    property_signal!(Memo);
    property_signal!(Signal);
    property_signal_unsend!(ArcRwSignal);
    property_signal_unsend!(ArcReadSignal);
    property_signal!(ArcMemo);
    property_signal!(ArcSignal);
}
