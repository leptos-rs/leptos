use super::RenderEffectState;
use crate::{html::class::IntoClass, renderer::DomRenderer};
use reactive_graph::{effect::RenderEffect, signal::guards::ReadGuard};
use std::{borrow::Borrow, ops::Deref};

impl<F, C, R> IntoClass<R> for F
where
    F: FnMut() -> C + 'static,
    C: IntoClass<R> + 'static,
    C::State: 'static,
    R: DomRenderer,
{
    type State = RenderEffectState<C::State>;

    fn html_len(&self) -> usize {
        0
    }

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
        .into()
    }

    fn build(mut self, el: &R::Element) -> Self::State {
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
        .into()
    }

    fn rebuild(mut self, state: &mut Self::State) {
        let prev_effect = std::mem::take(&mut state.0);
        let prev_value = prev_effect.as_ref().and_then(|e| e.take_value());
        drop(prev_effect);
        *state = RenderEffect::new_with_value(
            move |prev| {
                let value = self();
                if let Some(mut state) = prev {
                    value.rebuild(&mut state);
                    state
                } else {
                    todo!()
                }
            },
            prev_value,
        )
        .into();
    }
}

impl<F, T, R> IntoClass<R> for (&'static str, F)
where
    F: FnMut() -> T + 'static,
    T: Borrow<bool>,
    R: DomRenderer,
{
    type State = RenderEffectState<(R::ClassList, bool)>;

    fn html_len(&self) -> usize {
        self.0.len()
    }

    fn to_html(self, class: &mut String) {
        let (name, mut f) = self;
        let include = *f().borrow();
        if include {
            <&str as IntoClass<R>>::to_html(name, class);
        }
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        // TODO FROM_SERVER vs template
        let (name, mut f) = self;
        let class_list = R::class_list(el);
        let name = R::intern(name);

        RenderEffect::new(move |prev: Option<(R::ClassList, bool)>| {
            let include = *f().borrow();
            if let Some((class_list, prev)) = prev {
                if include {
                    if !prev {
                        R::add_class(&class_list, name);
                    }
                } else if prev {
                    R::remove_class(&class_list, name);
                }
            }
            (class_list.clone(), include)
        })
        .into()
    }

    fn build(self, el: &R::Element) -> Self::State {
        let (name, mut f) = self;
        let class_list = R::class_list(el);
        let name = R::intern(name);

        RenderEffect::new(move |prev: Option<(R::ClassList, bool)>| {
            let include = *f().borrow();
            match prev {
                Some((class_list, prev)) => {
                    if include {
                        if !prev {
                            R::add_class(&class_list, name);
                        }
                    } else if prev {
                        R::remove_class(&class_list, name);
                    }
                }
                None => {
                    if include {
                        R::add_class(&class_list, name);
                    }
                }
            }
            (class_list.clone(), include)
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
                let include = f();
                match prev {
                    Some((class_list, prev)) => {
                        if include {
                            if !prev {
                                R::add_class(&class_list, name);
                            }
                        } else if prev {
                            R::remove_class(&class_list, name);
                        }
                        (class_list.clone(), include)
                    }
                    None => unreachable!(),
                }
            },
            prev_value,
        )
        .into(); */
    }
}

impl<G, R> IntoClass<R> for ReadGuard<String, G>
where
    G: Deref<Target = String>,
    R: DomRenderer,
{
    type State = <String as IntoClass<R>>::State;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, class: &mut String) {
        <&str as IntoClass<R>>::to_html(self.deref().as_str(), class);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &<R>::Element,
    ) -> Self::State {
        <String as IntoClass<R>>::hydrate::<FROM_SERVER>(
            self.deref().to_owned(),
            el,
        )
    }

    fn build(self, el: &<R>::Element) -> Self::State {
        <String as IntoClass<R>>::build(self.deref().to_owned(), el)
    }

    fn rebuild(self, state: &mut Self::State) {
        <String as IntoClass<R>>::rebuild(self.deref().to_owned(), state)
    }
}

impl<G, R> IntoClass<R> for (&'static str, ReadGuard<bool, G>)
where
    G: Deref<Target = bool>,
    R: DomRenderer,
{
    type State = <(&'static str, bool) as IntoClass<R>>::State;

    fn html_len(&self) -> usize {
        self.0.len()
    }

    fn to_html(self, class: &mut String) {
        <(&'static str, bool) as IntoClass<R>>::to_html(
            (self.0, *self.1.deref()),
            class,
        );
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &<R>::Element,
    ) -> Self::State {
        <(&'static str, bool) as IntoClass<R>>::hydrate::<FROM_SERVER>(
            (self.0, *self.1.deref()),
            el,
        )
    }

    fn build(self, el: &<R>::Element) -> Self::State {
        <(&'static str, bool) as IntoClass<R>>::build(
            (self.0, *self.1.deref()),
            el,
        )
    }

    fn rebuild(self, state: &mut Self::State) {
        <(&'static str, bool) as IntoClass<R>>::rebuild(
            (self.0, *self.1.deref()),
            state,
        )
    }
}
