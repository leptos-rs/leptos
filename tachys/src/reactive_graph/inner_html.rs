use crate::{
    html::element::InnerHtmlValue,
    renderer::{DomRenderer, Renderer},
};
use reactive_graph::effect::RenderEffect;

impl<F, V, R> InnerHtmlValue<R> for F
where
    F: FnMut() -> V + Send + 'static,
    V: InnerHtmlValue<R>,
    V::State: 'static,
    R: DomRenderer,
{
    type State = RenderEffect<V::State>;

    fn html_len(&self) -> usize {
        0
    }

    fn to_html(mut self, buf: &mut String) {
        let value = self();
        value.to_html(buf);
    }

    fn to_template(_buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        mut self,
        el: &<R as Renderer>::Element,
    ) -> Self::State {
        let el = el.to_owned();
        RenderEffect::new(move |prev| {
            let value = self();
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
            let value = self();
            if let Some(mut state) = prev {
                value.rebuild(&mut state);
                state
            } else {
                value.build(&el)
            }
        })
    }

    fn rebuild(self, _state: &mut Self::State) {}
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
        wrappers::read::{ArcSignal, Signal},
    };

    macro_rules! inner_html_signal {
        ($sig:ident) => {
            impl<V, R> InnerHtmlValue<R> for $sig<V>
            where
                V: InnerHtmlValue<R> + Send + Sync + Clone + 'static,
                V::State: 'static,
                R: DomRenderer,
            {
                type State = RenderEffect<V::State>;

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
            }
        };
    }

    macro_rules! inner_html_signal_unsend {
        ($sig:ident) => {
            impl<V, R> InnerHtmlValue<R> for $sig<V>
            where
                V: InnerHtmlValue<R> + Send + Sync + Clone + 'static,
                V::State: 'static,
                R: DomRenderer,
            {
                type State = RenderEffect<V::State>;

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
            }
        };
    }

    inner_html_signal!(RwSignal);
    inner_html_signal!(ReadSignal);
    inner_html_signal!(Memo);
    inner_html_signal!(Signal);
    inner_html_signal_unsend!(ArcRwSignal);
    inner_html_signal_unsend!(ArcReadSignal);
    inner_html_signal!(ArcMemo);
    inner_html_signal!(ArcSignal);
}
