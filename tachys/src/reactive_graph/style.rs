use super::{ReactiveFunction, SharedReactiveFunction, Suspend};
use crate::{html::style::IntoStyle, renderer::Rndr};
use futures::FutureExt;
use reactive_graph::effect::RenderEffect;
use std::{borrow::Cow, cell::RefCell, future::Future, rc::Rc};

impl<F, S> IntoStyle for (&'static str, F)
where
    F: ReactiveFunction<Output = S>,
    S: Into<Cow<'static, str>> + 'static,
{
    type AsyncOutput = Self;
    type State = RenderEffect<(
        crate::renderer::types::CssStyleDeclaration,
        Cow<'static, str>,
    )>;
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

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let (name, mut f) = self;
        let name = Rndr::intern(name);
        // TODO FROM_SERVER vs template
        let style = Rndr::style(el);
        RenderEffect::new(move |prev| {
            let value = f.invoke().into();
            if let Some(mut state) = prev {
                let (style, prev): &mut (
                    crate::renderer::types::CssStyleDeclaration,
                    Cow<'static, str>,
                ) = &mut state;
                if &value != prev {
                    Rndr::set_css_property(style, name, &value);
                }
                *prev = value;
                state
            } else {
                // only set the style in template mode
                // in server mode, it's already been set
                if !FROM_SERVER {
                    Rndr::set_css_property(&style, name, &value);
                }
                (style.clone(), value)
            }
        })
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let (name, mut f) = self;
        let name = Rndr::intern(name);
        let style = Rndr::style(el);
        RenderEffect::new(move |prev| {
            let value = f.invoke().into();
            if let Some(mut state) = prev {
                let (style, prev): &mut (
                    crate::renderer::types::CssStyleDeclaration,
                    Cow<'static, str>,
                ) = &mut state;
                if &value != prev {
                    Rndr::set_css_property(style, name, &value);
                }
                *prev = value;
                state
            } else {
                // always set the style initially without checking
                Rndr::set_css_property(&style, name, &value);
                (style.clone(), value)
            }
        })
    }

    fn rebuild(self, state: &mut Self::State) {
        let (name, mut f) = self;
        let prev_value = state.take_value();
        *state = RenderEffect::new_with_value(
            move |prev| {
                let value = f.invoke().into();
                if let Some(mut state) = prev {
                    let (style, prev) = &mut state;
                    if &value != prev {
                        Rndr::set_css_property(style, name, &value);
                    }
                    *prev = value;
                    state
                } else {
                    unreachable!()
                }
            },
            prev_value,
        );
    }

    fn into_cloneable(self) -> Self::Cloneable {
        (self.0, self.1.into_shared())
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        (self.0, self.1.into_shared())
    }

    fn dry_resolve(&mut self) {
        self.1.invoke();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl<F, C> IntoStyle for F
where
    F: ReactiveFunction<Output = C>,
    C: IntoStyle + 'static,
    C::State: 'static,
{
    type AsyncOutput = C::AsyncOutput;
    type State = RenderEffect<C::State>;
    type Cloneable = SharedReactiveFunction<C>;
    type CloneableOwned = SharedReactiveFunction<C>;

    fn to_html(mut self, style: &mut String) {
        let value = self.invoke();
        value.to_html(style);
    }

    fn hydrate<const FROM_SERVER: bool>(
        mut self,
        el: &crate::renderer::types::Element,
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

    fn build(mut self, el: &crate::renderer::types::Element) -> Self::State {
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

#[cfg(not(feature = "nightly"))]
mod stable {
    macro_rules! style_signal {
        ($sig:ident) => {
            impl<C> IntoStyle for $sig<C>
            where
                $sig<C>: Get<Value = C>,
                C: IntoStyle + Clone + Send + Sync + 'static,
                C::State: 'static,
            {
                type AsyncOutput = Self;
                type State = RenderEffect<C::State>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn to_html(self, style: &mut String) {
                    let value = self.get();
                    value.to_html(style);
                }

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

            impl<S> IntoStyle for (&'static str, $sig<S>)
            where
                $sig<S>: Get<Value = S>,
                S: Into<Cow<'static, str>> + Send + Sync + Clone + 'static,
            {
                type AsyncOutput = Self;
                type State = RenderEffect<(
                    crate::renderer::types::CssStyleDeclaration,
                    Cow<'static, str>,
                )>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn to_html(self, style: &mut String) {
                    IntoStyle::to_html((self.0, move || self.1.get()), style)
                }

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &crate::renderer::types::Element,
                ) -> Self::State {
                    IntoStyle::hydrate::<FROM_SERVER>(
                        (self.0, move || self.1.get()),
                        el,
                    )
                }

                fn build(
                    self,
                    el: &crate::renderer::types::Element,
                ) -> Self::State {
                    IntoStyle::build((self.0, move || self.1.get()), el)
                }

                fn rebuild(self, state: &mut Self::State) {
                    IntoStyle::rebuild((self.0, move || self.1.get()), state)
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

    macro_rules! style_signal_arena {
        ($sig:ident) => {
            impl<C, S> IntoStyle for $sig<C, S>
            where
                $sig<C, S>: Get<Value = C>,
                S: Storage<C> + Storage<Option<C>>,
                S: Send + Sync + 'static,
                C: IntoStyle + Send + Sync + Clone + 'static,
                C::State: 'static,
            {
                type AsyncOutput = Self;
                type State = RenderEffect<C::State>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn to_html(self, style: &mut String) {
                    let value = self.get();
                    value.to_html(style);
                }

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

            impl<S, St> IntoStyle for (&'static str, $sig<S, St>)
            where
                $sig<S, St>: Get<Value = S>,
                St: Send + Sync + 'static,
                St: Storage<S> + Storage<Option<S>>,
                S: Into<Cow<'static, str>> + Send + Sync + Clone + 'static,
            {
                type AsyncOutput = Self;
                type State = RenderEffect<(
                    crate::renderer::types::CssStyleDeclaration,
                    Cow<'static, str>,
                )>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn to_html(self, style: &mut String) {
                    IntoStyle::to_html((self.0, move || self.1.get()), style)
                }

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &crate::renderer::types::Element,
                ) -> Self::State {
                    IntoStyle::hydrate::<FROM_SERVER>(
                        (self.0, move || self.1.get()),
                        el,
                    )
                }

                fn build(
                    self,
                    el: &crate::renderer::types::Element,
                ) -> Self::State {
                    IntoStyle::build((self.0, move || self.1.get()), el)
                }

                fn rebuild(self, state: &mut Self::State) {
                    IntoStyle::rebuild((self.0, move || self.1.get()), state)
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

    use super::RenderEffect;
    use crate::html::style::IntoStyle;
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        owner::Storage,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, MaybeSignal, Signal},
    };
    use std::borrow::Cow;

    style_signal_arena!(RwSignal);
    style_signal_arena!(ReadSignal);
    style_signal_arena!(Memo);
    style_signal_arena!(Signal);
    style_signal_arena!(MaybeSignal);
    style_signal!(ArcRwSignal);
    style_signal!(ArcReadSignal);
    style_signal!(ArcMemo);
    style_signal!(ArcSignal);
}

impl<Fut> IntoStyle for Suspend<Fut>
where
    Fut: Clone + Future + Send + 'static,
    Fut::Output: IntoStyle,
{
    type AsyncOutput = Fut::Output;
    type State = Rc<RefCell<Option<<Fut::Output as IntoStyle>::State>>>;
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn to_html(self, style: &mut String) {
        if let Some(inner) = self.inner.now_or_never() {
            inner.to_html(style);
        } else {
            panic!("You cannot use Suspend on an attribute outside Suspense");
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let el = el.to_owned();
        let state = Rc::new(RefCell::new(None));
        reactive_graph::spawn_local_scoped({
            let state = Rc::clone(&state);
            async move {
                *state.borrow_mut() =
                    Some(self.inner.await.hydrate::<FROM_SERVER>(&el));
                self.subscriber.forward();
            }
        });
        state
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let el = el.to_owned();
        let state = Rc::new(RefCell::new(None));
        reactive_graph::spawn_local_scoped({
            let state = Rc::clone(&state);
            async move {
                *state.borrow_mut() = Some(self.inner.await.build(&el));
                self.subscriber.forward();
            }
        });
        state
    }

    fn rebuild(self, state: &mut Self::State) {
        reactive_graph::spawn_local_scoped({
            let state = Rc::clone(state);
            async move {
                let value = self.inner.await;
                let mut state = state.borrow_mut();
                if let Some(state) = state.as_mut() {
                    value.rebuild(state);
                }
                self.subscriber.forward();
            }
        });
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self.inner.await
    }
}
