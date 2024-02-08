use super::RenderEffectState;
use crate::{html::class::IntoClass, renderer::DomRenderer};
use reactive_graph::effect::RenderEffect;

impl<F, C, R> IntoClass<R> for F
where
    F: FnMut() -> C + 'static,
    C: IntoClass<R> + 'static,
    C::State: 'static,
    R: DomRenderer,
    R::ClassList: 'static,
    R::Element: Clone + 'static,
{
    type State = RenderEffectState<C::State>;

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
                crate::log("rebuilt class");
                let value = self();
                if let Some(mut state) = prev {
                    crate::log("  rebuilt it");
                    value.rebuild(&mut state);
                    state
                } else {
                    crate::log("  oh no!");
                    todo!()
                }
            },
            prev_value,
        )
        .into();
    }
}

impl<F, R> IntoClass<R> for (&'static str, F)
where
    F: FnMut() -> bool + 'static,
    R: DomRenderer,
    R::ClassList: Clone + 'static,
    R::Element: Clone,
{
    type State = RenderEffectState<(R::ClassList, bool)>;

    fn to_html(self, class: &mut String) {
        let (name, mut f) = self;
        let include = f();
        if include {
            <&str as IntoClass<R>>::to_html(name, class);
        }
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        // TODO FROM_SERVER vs template
        let (name, mut f) = self;
        let class_list = R::class_list(el);
        RenderEffect::new(move |prev: Option<(R::ClassList, bool)>| {
            let include = f();
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
        RenderEffect::new(move |prev: Option<(R::ClassList, bool)>| {
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

    fn rebuild(self, state: &mut Self::State) {
        // TODO
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
