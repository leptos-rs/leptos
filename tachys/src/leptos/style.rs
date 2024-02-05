use crate::{html::style::IntoStyle, renderer::DomRenderer};
use leptos_reactive::{create_render_effect, Effect};
use std::borrow::Cow;

impl<F, S, R> IntoStyle<R> for (&'static str, F)
where
    F: Fn() -> S + 'static,
    S: Into<Cow<'static, str>>,
    R: DomRenderer,
    R::CssStyleDeclaration: Clone + 'static,
{
    type State = Effect<(R::CssStyleDeclaration, Cow<'static, str>)>;

    fn to_html(self, style: &mut String) {
        let (name, f) = self;
        let value = f();
        style.push_str(name);
        style.push(':');
        style.push_str(&value.into());
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        let (name, f) = self;
        // TODO FROM_SERVER vs template
        let style = R::style(el);
        create_render_effect(move |prev| {
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
                (style.clone(), value)
            }
        })
    }

    fn build(self, el: &R::Element) -> Self::State {
        todo!()
    }

    fn rebuild(self, state: &mut Self::State) {}
}

impl<F, C, R> IntoStyle<R> for F
where
    F: Fn() -> C + 'static,
    C: IntoStyle<R> + 'static,
    C::State: 'static,
    R: DomRenderer,
    R::Element: Clone + 'static,
    R::CssStyleDeclaration: Clone + 'static,
{
    type State = Effect<C::State>;

    fn to_html(self, class: &mut String) {
        let value = self();
        value.to_html(class);
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        // TODO FROM_SERVER vs template
        let el = el.clone();
        create_render_effect(move |prev| {
            let value = self();
            if let Some(mut state) = prev {
                value.rebuild(&mut state);
                state
            } else {
                value.hydrate::<FROM_SERVER>(&el)
            }
        })
    }

    fn build(self, el: &R::Element) -> Self::State {
        todo!()
    }

    fn rebuild(self, state: &mut Self::State) {}
}
