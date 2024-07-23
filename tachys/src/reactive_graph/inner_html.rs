use super::{ReactiveFunction, SharedReactiveFunction};
use crate::{
    html::element::InnerHtmlValue,
    renderer::{DomRenderer, Renderer},
};
use reactive_graph::effect::RenderEffect;

impl<F, V, R> InnerHtmlValue<R> for F
where
    F: ReactiveFunction<Output = V>,
    V: InnerHtmlValue<R> + 'static,
    V::State: 'static,
    R: DomRenderer,
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
        el: &<R as Renderer>::Element,
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

    fn build(mut self, el: &<R as Renderer>::Element) -> Self::State {
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

    fn rebuild(self, _state: &mut Self::State) {}

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

#[cfg(not(feature = "nightly"))]
mod stable {
    use crate::{
        html::element::InnerHtmlValue,
        renderer::{DomRenderer, Renderer},
    };
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        effect::RenderEffect,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, MaybeSignal, Signal},
    };

    macro_rules! inner_html_signal {
        ($sig:ident) => {
            impl<V, R> InnerHtmlValue<R> for $sig<V>
            where
                $sig<V>: Get<Value = V>,
                V: InnerHtmlValue<R> + Send + Sync + Clone + 'static,
                V::State: 'static,
                R: DomRenderer,
            {
                type AsyncOutput = Self;
                type State = RenderEffect<V::State>;
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
                    el: &<R as Renderer>::Element,
                ) -> Self::State {
                    (move || self.get()).hydrate::<FROM_SERVER>(el)
                }

                fn build(self, el: &<R as Renderer>::Element) -> Self::State {
                    (move || self.get()).build(el)
                }

                fn rebuild(self, _state: &mut Self::State) {}

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

    macro_rules! inner_html_signal_arena {
        ($sig:ident) => {
            impl<V, R, S> InnerHtmlValue<R> for $sig<V, S>
            where
                $sig<V, S>: Get<Value = V>,
                S: Send + Sync + 'static,
                V: InnerHtmlValue<R> + Send + Sync + Clone + 'static,
                V::State: 'static,
                R: DomRenderer,
            {
                type AsyncOutput = Self;
                type State = RenderEffect<V::State>;
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
                    el: &<R as Renderer>::Element,
                ) -> Self::State {
                    (move || self.get()).hydrate::<FROM_SERVER>(el)
                }

                fn build(self, el: &<R as Renderer>::Element) -> Self::State {
                    (move || self.get()).build(el)
                }

                fn rebuild(self, _state: &mut Self::State) {}

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

    inner_html_signal_arena!(RwSignal);
    inner_html_signal_arena!(ReadSignal);
    inner_html_signal_arena!(Memo);
    inner_html_signal_arena!(Signal);
    inner_html_signal_arena!(MaybeSignal);
    inner_html_signal!(ArcRwSignal);
    inner_html_signal!(ArcReadSignal);
    inner_html_signal!(ArcMemo);
    inner_html_signal!(ArcSignal);
}
