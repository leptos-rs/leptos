use super::{ReactiveFunction, SharedReactiveFunction};
use crate::{html::style::IntoStyle, renderer::DomRenderer};
use reactive_graph::effect::RenderEffect;
use std::borrow::Cow;

impl<F, S, R> IntoStyle<R> for (&'static str, F)
where
    F: ReactiveFunction<Output = S>,
    S: Into<Cow<'static, str>> + 'static,
    R: DomRenderer,
{
    type State = RenderEffect<(R::CssStyleDeclaration, Cow<'static, str>)>;
    type Cloneable = (&'static str, SharedReactiveFunction<S>);
    type CloneableOwned = (&'static str, SharedReactiveFunction<S>);

    fn to_html(self, style: &mut String) {
        let (name, mut f) = self;
        let value = f.invoke();
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
            let value = f.invoke().into();
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
    }

    fn build(self, el: &R::Element) -> Self::State {
        let (name, mut f) = self;
        let name = R::intern(name);
        let style = R::style(el);
        RenderEffect::new(move |prev| {
            let value = f.invoke().into();
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
    }

    fn rebuild(self, _state: &mut Self::State) {
        // TODO rebuild
    }

    fn into_cloneable(self) -> Self::Cloneable {
        (self.0, self.1.into_shared())
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        (self.0, self.1.into_shared())
    }
}

impl<F, C, R> IntoStyle<R> for F
where
    F: ReactiveFunction<Output = C>,
    C: IntoStyle<R> + 'static,
    C::State: 'static,
    R: DomRenderer,
{
    type State = RenderEffect<C::State>;
    type Cloneable = SharedReactiveFunction<C>;
    type CloneableOwned = SharedReactiveFunction<C>;

    fn to_html(mut self, style: &mut String) {
        let value = self.invoke();
        value.to_html(style);
    }

    fn hydrate<const FROM_SERVER: bool>(
        mut self,
        el: &R::Element,
    ) -> Self::State {
        // TODO FROM_SERVER vs template
        let el = el.clone();
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

    fn build(mut self, el: &R::Element) -> Self::State {
        let el = el.clone();
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
}

#[cfg(not(feature = "nightly"))]
mod stable {
    macro_rules! style_signal {
        ($sig:ident) => {
            impl<C, R> IntoStyle<R> for $sig<C>
            where
                C: IntoStyle<R> + Clone + Send + Sync + 'static,
                C::State: 'static,
                R: DomRenderer,
            {
                type State = RenderEffect<C::State>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn to_html(self, style: &mut String) {
                    let value = self.get();
                    value.to_html(style);
                }

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &R::Element,
                ) -> Self::State {
                    (move || self.get()).hydrate::<FROM_SERVER>(el)
                }

                fn build(self, el: &R::Element) -> Self::State {
                    (move || self.get()).build(el)
                }

                fn rebuild(self, _state: &mut Self::State) {
                    // TODO rebuild here?
                }

                fn into_cloneable(self) -> Self::Cloneable {
                    self
                }

                fn into_cloneable_owned(self) -> Self::CloneableOwned {
                    self
                }
            }

            impl<R, S> IntoStyle<R> for (&'static str, $sig<S>)
            where
                S: Into<Cow<'static, str>> + Send + Sync + Clone + 'static,
                R: DomRenderer,
            {
                type State =
                    RenderEffect<(R::CssStyleDeclaration, Cow<'static, str>)>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn to_html(self, style: &mut String) {
                    IntoStyle::<R>::to_html(
                        (self.0, move || self.1.get()),
                        style,
                    )
                }

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &R::Element,
                ) -> Self::State {
                    IntoStyle::<R>::hydrate::<FROM_SERVER>(
                        (self.0, move || self.1.get()),
                        el,
                    )
                }

                fn build(self, el: &R::Element) -> Self::State {
                    IntoStyle::<R>::build((self.0, move || self.1.get()), el)
                }

                fn rebuild(self, _state: &mut Self::State) {
                    // TODO rebuild here?
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

    macro_rules! style_signal_unsend {
        ($sig:ident) => {
            impl<C, R> IntoStyle<R> for $sig<C>
            where
                C: IntoStyle<R> + Send + Sync + Clone + 'static,
                C::State: 'static,
                R: DomRenderer,
            {
                type State = RenderEffect<C::State>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn to_html(self, style: &mut String) {
                    let value = self.get();
                    value.to_html(style);
                }

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &R::Element,
                ) -> Self::State {
                    (move || self.get()).hydrate::<FROM_SERVER>(el)
                }

                fn build(self, el: &R::Element) -> Self::State {
                    (move || self.get()).build(el)
                }

                fn rebuild(self, _state: &mut Self::State) {
                    // TODO rebuild here?
                }

                fn into_cloneable(self) -> Self::Cloneable {
                    self
                }

                fn into_cloneable_owned(self) -> Self::CloneableOwned {
                    self
                }
            }

            impl<R, S> IntoStyle<R> for (&'static str, $sig<S>)
            where
                S: Into<Cow<'static, str>> + Send + Sync + Clone + 'static,
                R: DomRenderer,
            {
                type State =
                    RenderEffect<(R::CssStyleDeclaration, Cow<'static, str>)>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn to_html(self, style: &mut String) {
                    IntoStyle::<R>::to_html(
                        (self.0, move || self.1.get()),
                        style,
                    )
                }

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &R::Element,
                ) -> Self::State {
                    IntoStyle::<R>::hydrate::<FROM_SERVER>(
                        (self.0, move || self.1.get()),
                        el,
                    )
                }

                fn build(self, el: &R::Element) -> Self::State {
                    IntoStyle::<R>::build((self.0, move || self.1.get()), el)
                }

                fn rebuild(self, _state: &mut Self::State) {
                    // TODO rebuild here?
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

    use super::RenderEffect;
    use crate::{html::style::IntoStyle, renderer::DomRenderer};
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, Signal},
    };
    use std::borrow::Cow;

    style_signal!(RwSignal);
    style_signal!(ReadSignal);
    style_signal!(Memo);
    style_signal!(Signal);
    style_signal_unsend!(ArcRwSignal);
    style_signal_unsend!(ArcReadSignal);
    style_signal!(ArcMemo);
    style_signal!(ArcSignal);
}
