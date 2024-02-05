use crate::{
    async_views::Suspend,
    html::{attribute::AttributeValue, property::IntoProperty},
    hydration::Cursor,
    renderer::{DomRenderer, Renderer},
    ssr::StreamBuilder,
    view::{
        FallibleRender, InfallibleRender, Mountable, Position, PositionState,
        Render, RenderHtml, ToTemplate,
    },
};
use std::mem;
use tachy_reaccy::{async_signal::ScopedFuture, render_effect::RenderEffect};

mod class;
pub mod node_ref;
mod style;

impl<F, V> ToTemplate for F
where
    F: FnMut() -> V,
    V: ToTemplate,
{
    const TEMPLATE: &'static str = V::TEMPLATE;

    fn to_template(
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
        position: &mut Position,
    ) {
        // FIXME this seems wrong
        V::to_template(buf, class, style, inner_html, position)
    }
}

impl<F, V, R> Render<R> for F
where
    F: FnMut() -> V + 'static,
    V: Render<R>,
    V::State: 'static,
    R: Renderer,
{
    type State = RenderEffectState<V::State>;

    #[track_caller]
    fn build(mut self) -> Self::State {
        RenderEffect::new(move |prev| {
            let value = self();
            if let Some(mut state) = prev {
                value.rebuild(&mut state);
                state
            } else {
                value.build()
            }
        })
        .into()
    }

    #[track_caller]
    fn rebuild(mut self, state: &mut Self::State) {
        // TODO
        /* let prev_effect = mem::take(&mut state.0);
        let prev_value = prev_effect.as_ref().and_then(|e| e.take_value());
        drop(prev_effect);
        *state = RenderEffect::new_with_value(
            move |prev| {
                let value = self();
                if let Some(mut state) = prev {
                    value.rebuild(&mut state);
                    state
                } else {
                    value.build()
                }
            },
            prev_value,
        )
        .into(); */
    }
}

pub struct RenderEffectState<T: 'static>(Option<RenderEffect<T>>);

impl<T> From<RenderEffect<T>> for RenderEffectState<T> {
    fn from(value: RenderEffect<T>) -> Self {
        Self(Some(value))
    }
}

impl<T, R> Mountable<R> for RenderEffectState<T>
where
    T: Mountable<R>,
    R: Renderer,
{
    fn unmount(&mut self) {
        if let Some(ref mut inner) = self.0 {
            inner.unmount();
        }
    }

    fn mount(&mut self, parent: &R::Element, marker: Option<&R::Node>) {
        if let Some(ref mut inner) = self.0 {
            inner.mount(parent, marker);
        }
    }

    fn insert_before_this(
        &self,
        parent: &R::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool {
        if let Some(inner) = &self.0 {
            inner.insert_before_this(parent, child)
        } else {
            false
        }
    }
}

impl<F, V> InfallibleRender for F where F: FnMut() -> V + 'static {}

/* impl<F, V, R> FallibleRender<R> for F
where
    F: FnMut() -> V + 'static,
    V: FallibleRender<R>,
    V::State: 'static,
    R: Renderer,
{
    type FallibleState = V::FallibleState;
    type Error = V::Error;

    fn try_build(self) -> Result<Self::FallibleState, Self::Error> {
        todo!()
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> Result<(), Self::Error> {
        todo!()
    }
} */

impl<F, V, R> RenderHtml<R> for F
where
    F: FnMut() -> V + 'static,
    V: RenderHtml<R>,
    V::State: 'static,
    R: Renderer + 'static,
    R::Node: Clone,
    R::Element: Clone,
{
    const MIN_LENGTH: usize = 0;

    fn to_html_with_buf(mut self, buf: &mut String, position: &mut Position) {
        let value = self();
        value.to_html_with_buf(buf, position)
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        mut self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        let value = self();
        value.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
    }

    fn hydrate<const FROM_SERVER: bool>(
        mut self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        let cursor = cursor.clone();
        let position = position.clone();
        RenderEffect::new(move |prev| {
            let value = self();
            if let Some(mut state) = prev {
                value.rebuild(&mut state);
                state
            } else {
                value.hydrate::<FROM_SERVER>(&cursor, &position)
            }
        })
        .into()
    }
}

impl<M, R> Mountable<R> for RenderEffect<M>
where
    M: Mountable<R> + 'static,
    R: Renderer,
{
    fn unmount(&mut self) {
        self.with_value_mut(|state| state.unmount());
    }

    fn mount(
        &mut self,
        parent: &<R as Renderer>::Element,
        marker: Option<&<R as Renderer>::Node>,
    ) {
        self.with_value_mut(|state| {
            state.mount(parent, marker);
        });
    }

    fn insert_before_this(
        &self,
        parent: &<R as Renderer>::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool {
        self.with_value_mut(|value| value.insert_before_this(parent, child))
            .unwrap_or(false)
    }
}

// Extends to track suspense
impl<const TRANSITION: bool, Fal, Fut> Suspend<TRANSITION, Fal, Fut> {
    pub fn track(self) -> Suspend<TRANSITION, Fal, ScopedFuture<Fut>> {
        let Suspend { fallback, fut } = self;
        Suspend {
            fallback,
            fut: ScopedFuture::new(fut),
        }
    }
}

// Dynamic attributes
impl<F, V, R> AttributeValue<R> for F
where
    F: FnMut() -> V + 'static,
    V: AttributeValue<R>,
    V::State: 'static,
    R: Renderer,
    R::Element: Clone + 'static,
{
    type State = RenderEffectState<V::State>;

    fn to_html(mut self, key: &str, buf: &mut String) {
        let value = self();
        value.to_html(key, buf);
    }

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        mut self,
        key: &str,
        el: &<R as Renderer>::Element,
    ) -> Self::State {
        let key = key.to_owned();
        let el = el.to_owned();
        RenderEffect::new(move |prev| {
            let value = self();
            if let Some(mut state) = prev {
                value.rebuild(&key, &mut state);
                state
            } else {
                value.hydrate::<FROM_SERVER>(&key, &el)
            }
        })
        .into()
    }

    fn build(
        mut self,
        el: &<R as Renderer>::Element,
        key: &str,
    ) -> Self::State {
        let key = key.to_owned();
        let el = el.to_owned();
        RenderEffect::new(move |prev| {
            let value = self();
            if let Some(mut state) = prev {
                value.rebuild(&key, &mut state);
                state
            } else {
                value.build(&el, &key)
            }
        })
        .into()
    }

    fn rebuild(mut self, key: &str, state: &mut Self::State) {
        // TODO
        /* let prev_effect = mem::take(&mut state.0);
        let prev_value = prev_effect.as_ref().and_then(|e| e.take_value());
        drop(prev_effect);
        let key = key.to_owned();
        *state = RenderEffect::new_with_value(
            move |prev| {
                crate::log(&format!(
                    "inside here, prev is some? {}",
                    prev.is_some()
                ));
                let value = self();
                if let Some(mut state) = prev {
                    value.rebuild(&key, &mut state);
                    state
                } else {
                    unreachable!()
                }
            },
            prev_value,
        )
        .into(); */
    }

    /*     fn build(self) -> Self::State {
        RenderEffect::new(move |prev| {
            let value = self();
            if let Some(mut state) = prev {
                value.rebuild(&mut state);
                state
            } else {
                value.build()
            }
        })
    }

    #[track_caller]
    fn rebuild(self, state: &mut Self::State) {
        /* crate::log(&format!(
            "[REBUILDING EFFECT] Is this a mistake? {}",
            std::panic::Location::caller(),
        )); */
        let old_effect = std::mem::replace(state, self.build());
    } */
}

// Dynamic properties
// These do update during hydration because properties don't exist in the DOM
impl<F, V, R> IntoProperty<R> for F
where
    F: FnMut() -> V + 'static,
    V: IntoProperty<R>,
    V::State: 'static,
    R: DomRenderer,
    R::Element: Clone + 'static,
{
    type State = RenderEffectState<V::State>;

    fn hydrate<const FROM_SERVER: bool>(
        mut self,
        el: &<R as Renderer>::Element,
        key: &str,
    ) -> Self::State {
        let key = key.to_owned();
        let el = el.to_owned();
        RenderEffect::new(move |prev| {
            let value = self();
            if let Some(mut state) = prev {
                value.rebuild(&mut state, &key);
                state
            } else {
                value.hydrate::<FROM_SERVER>(&el, &key)
            }
        })
        .into()
    }

    fn build(
        mut self,
        el: &<R as Renderer>::Element,
        key: &str,
    ) -> Self::State {
        let key = key.to_owned();
        let el = el.to_owned();
        RenderEffect::new(move |prev| {
            let value = self();
            if let Some(mut state) = prev {
                value.rebuild(&mut state, &key);
                state
            } else {
                value.build(&el, &key)
            }
        })
        .into()
    }

    fn rebuild(mut self, state: &mut Self::State, key: &str) {
        // TODO
        /* let prev_effect = mem::take(&mut state.0);
        let prev_value = prev_effect.as_ref().and_then(|e| e.take_value());
        drop(prev_effect);
        let key = key.to_owned();
        *state = RenderEffect::new_with_value(
            move |prev| {
                let value = self();
                if let Some(mut state) = prev {
                    value.rebuild(&mut state, &key);
                    state
                } else {
                    unreachable!()
                }
            },
            prev_value,
        )
        .into(); */
    }
}

/*
#[cfg(test)]
mod tests {
    use crate::{
        html::element::{button, main, HtmlElement},
        renderer::mock_dom::MockDom,
        view::Render,
    };
    use leptos_reactive::{create_runtime, RwSignal, SignalGet, SignalSet};

    #[test]
    fn create_dynamic_element() {
        let rt = create_runtime();
        let count = RwSignal::new(0);
        let app: HtmlElement<_, _, _, MockDom> =
            button((), move || count.get().to_string());
        let el = app.build();
        assert_eq!(el.el.to_debug_html(), "<button>0</button>");
        rt.dispose();
    }

    #[test]
    fn update_dynamic_element() {
        let rt = create_runtime();
        let count = RwSignal::new(0);
        let app: HtmlElement<_, _, _, MockDom> =
            button((), move || count.get().to_string());
        let el = app.build();
        assert_eq!(el.el.to_debug_html(), "<button>0</button>");
        count.set(1);
        assert_eq!(el.el.to_debug_html(), "<button>1</button>");
        rt.dispose();
    }

    #[test]
    fn update_dynamic_element_among_siblings() {
        let rt = create_runtime();
        let count = RwSignal::new(0);
        let app: HtmlElement<_, _, _, MockDom> = main(
            (),
            button(
                (),
                ("Hello, my ", move || count.get().to_string(), " friends."),
            ),
        );
        let el = app.build();
        assert_eq!(
            el.el.to_debug_html(),
            "<main><button>Hello, my 0 friends.</button></main>"
        );
        count.set(42);
        assert_eq!(
            el.el.to_debug_html(),
            "<main><button>Hello, my 42 friends.</button></main>"
        );
        rt.dispose();
    }
}
 */
