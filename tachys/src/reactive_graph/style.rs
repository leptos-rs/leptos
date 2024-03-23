use super::RenderEffectState;
use crate::{html::style::IntoStyle, renderer::DomRenderer};
use reactive_graph::effect::RenderEffect;
use std::borrow::Cow;

impl<F, S, R> IntoStyle<R> for (&'static str, F)
where
    F: FnMut() -> S + 'static,
    S: Into<Cow<'static, str>>,
    R: DomRenderer,
{
    type State = RenderEffectState<(R::CssStyleDeclaration, Cow<'static, str>)>;

    fn to_html(self, style: &mut String) {
        let (name, mut f) = self;
        let value = f();
        style.push_str(name);
        style.push(':');
        style.push_str(&value.into());
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        let (name, mut f) = self;
        let name = R::intern(name);
        // TODO FROM_SERVER vs template
        let style = R::style(el);
        RenderEffect::new(move |prev| {
            let value = f().into();
            if let Some(mut state) = prev {
                let (style, prev): &mut (
                    R::CssStyleDeclaration,
                    Cow<'static, str>,
                ) = &mut state;
                if &value != prev {
                    R::set_css_property(style, name, &value);
                }
                *prev = value;
                state
            } else {
                // only set the style in template mode
                // in server mode, it's already been set
                if !FROM_SERVER {
                    R::set_css_property(&style, name, &value);
                }
                (style.clone(), value)
            }
        })
        .into()
    }

    fn build(self, el: &R::Element) -> Self::State {
        let (name, mut f) = self;
        let name = R::intern(name);
        let style = R::style(el);
        RenderEffect::new(move |prev| {
            let value = f().into();
            if let Some(mut state) = prev {
                let (style, prev): &mut (
                    R::CssStyleDeclaration,
                    Cow<'static, str>,
                ) = &mut state;
                if &value != prev {
                    R::set_css_property(style, name, &value);
                }
                *prev = value;
                state
            } else {
                // always set the style initially without checking
                R::set_css_property(&style, name, &value);
                (style.clone(), value)
            }
        })
        .into()
    }

    fn rebuild(self, _state: &mut Self::State) {
        // TODO — knowing how and whether to rebuild effects like this is tricky
        // it's the one place I've run into "stale values" when experimenting with this model

        /* let (name, mut f) = self;
        let prev_effect = std::mem::take(&mut state.0);
        let prev_value = prev_effect.as_ref().and_then(|e| e.take_value());
        drop(prev_effect);
        *state = RenderEffect::new_with_value(
            move |prev| {
                let value = f().into();
                if let Some(mut state) = prev {
                    let (style, prev) = &mut state;
                    if &value != prev {
                        R::set_css_property(&style, name, &value);
                    }
                    *prev = value;
                    state
                } else {
                    todo!()
                }
            },
            prev_value,
        )
        .into(); */
    }
}

impl<F, C, R> IntoStyle<R> for F
where
    F: FnMut() -> C + 'static,
    C: IntoStyle<R> + 'static,
    C::State: 'static,
    R: DomRenderer,
{
    type State = RenderEffect<C::State>;

    fn to_html(mut self, class: &mut String) {
        let value = self();
        value.to_html(class);
    }

    fn hydrate<const FROM_SERVER: bool>(
        mut self,
        el: &R::Element,
    ) -> Self::State {
        // TODO FROM_SERVER vs template
        let el = el.clone();
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

    fn build(self, _el: &R::Element) -> Self::State {
        todo!()
    }

    fn rebuild(self, _state: &mut Self::State) {}
}
