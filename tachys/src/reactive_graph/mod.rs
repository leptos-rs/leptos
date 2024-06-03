use crate::{
    async_views::Suspend,
    html::attribute::{Attribute, AttributeValue},
    hydration::Cursor,
    renderer::Renderer,
    ssr::StreamBuilder,
    view::{
        add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
        RenderHtml, ToTemplate,
    },
};
use any_error::Error as AnyError;
use reactive_graph::{
    computed::ScopedFuture,
    effect::RenderEffect,
    graph::{Observer, ReactiveNode},
};

mod class;
mod guards;
mod inner_html;
pub mod node_ref;
mod owned;
mod property;
mod style;
pub use owned::*;

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
    V::FallibleState: 'static,
    R: Renderer,
{
    type State = RenderEffectState<V::State>;
    type FallibleState =
        RenderEffectState<Result<V::FallibleState, Option<AnyError>>>;
    // TODO how this should be handled?
    type AsyncOutput = Self;

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

    fn try_build(mut self) -> any_error::Result<Self::FallibleState> {
        let parent = Observer::get();
        let effect = RenderEffect::new({
            move |prev| {
                let value = self();
                if let Some(mut state) = prev {
                    match state {
                        Ok(ref mut state) => {
                            if let Err(e) = value.try_rebuild(state) {
                                if let Some(parent) = &parent {
                                    parent.mark_check();
                                }
                                return Err(Some(e));
                            }
                        }
                        Err(_) => {
                            if let Some(parent) = &parent {
                                parent.mark_check();
                            }
                            return value.try_build().map_err(Some);
                        }
                    }
                    state
                } else {
                    value.try_build().map_err(Some)
                }
            }
        });
        effect
            .with_value_mut(|inner| match inner {
                Err(e) if e.is_some() => Err(e.take().unwrap()),
                _ => Ok(()),
            })
            .expect("RenderEffect should run once synchronously")
            .map(|_| effect.into())
    }

    #[track_caller]
    fn rebuild(self, _state: &mut Self::State) {
        // TODO rebuild
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> any_error::Result<()> {
        if let Some(inner) = &mut state.0 {
            inner
                .with_value_mut(|value| match value {
                    Err(e) if e.is_some() => Err(e.take().unwrap()),
                    _ => Ok(()),
                })
                .unwrap_or(Ok(()))
        } else {
            Ok(())
        }
    }

    async fn resolve(self) -> Self::AsyncOutput {
        self
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

pub struct RenderEffectFallibleState<T, E>
where
    T: 'static,
    E: 'static,
{
    effect: Option<RenderEffect<Result<T, E>>>,
}

impl<T, E, R> Mountable<R> for RenderEffectFallibleState<T, E>
where
    T: Mountable<R>,
    R: Renderer,
{
    fn unmount(&mut self) {
        if let Some(ref mut inner) = self.effect {
            inner.unmount();
        }
    }

    fn mount(&mut self, parent: &R::Element, marker: Option<&R::Node>) {
        if let Some(ref mut inner) = self.effect {
            inner.mount(parent, marker);
        }
    }

    fn insert_before_this(
        &self,
        parent: &R::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool {
        if let Some(inner) = &self.effect {
            inner.insert_before_this(parent, child)
        } else {
            false
        }
    }
}

impl<F, V, R> RenderHtml<R> for F
where
    F: FnMut() -> V + 'static,
    V: RenderHtml<R>,
    V::State: 'static,
    V::FallibleState: 'static,
    R: Renderer + 'static,
{
    const MIN_LENGTH: usize = 0;

    fn html_len(&self) -> usize {
        V::MIN_LENGTH
    }

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

impl<F, V, R> AddAnyAttr<R> for F
where
    F: FnMut() -> V + 'static,
    V: RenderHtml<R>,
    V::State: 'static,
    V::FallibleState: 'static,
    R: Renderer + 'static,
{
    type Output<SomeNewAttr: Attribute<R>> = Self;

    fn add_any_attr<NewAttr: Attribute<R>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<R>,
    {
        self
    }

    fn add_any_attr_by_ref<NewAttr: Attribute<R>>(
        self,
        attr: &NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<R>,
    {
        self
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

impl<M, E, R> Mountable<R> for Result<M, E>
where
    M: Mountable<R>,
    R: Renderer,
{
    fn unmount(&mut self) {
        if let Ok(ref mut inner) = self {
            inner.unmount();
        }
    }

    fn mount(
        &mut self,
        parent: &<R as Renderer>::Element,
        marker: Option<&<R as Renderer>::Node>,
    ) {
        if let Ok(ref mut inner) = self {
            inner.mount(parent, marker);
        }
    }

    fn insert_before_this(
        &self,
        parent: &<R as Renderer>::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool {
        if let Ok(inner) = &self {
            inner.insert_before_this(parent, child)
        } else {
            false
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
{
    type State = RenderEffectState<V::State>;

    fn html_len(&self) -> usize {
        0
    }

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
        let key = R::intern(key);
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
        let key = R::intern(key);
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

    fn rebuild(self, _key: &str, _state: &mut Self::State) {
        // TODO rebuild
    }
}

#[cfg(not(feature = "nightly"))]
mod stable {
    use super::RenderEffectState;
    use crate::{
        html::attribute::{Attribute, AttributeValue},
        hydration::Cursor,
        renderer::Renderer,
        ssr::StreamBuilder,
        view::{Position, PositionState, Render, RenderHtml},
    };
    use any_error::Error as AnyError;
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, Signal},
    };

    macro_rules! signal_impl {
        ($sig:ident) => {
            impl<V, R> Render<R> for $sig<V>
            where
                V: Render<R> + Clone + Send + Sync + 'static,
                V::State: 'static,
                V::FallibleState: 'static,
                R: Renderer,
            {
                type State = RenderEffectState<V::State>;
                type FallibleState = RenderEffectState<
                    Result<V::FallibleState, Option<AnyError>>,
                >;
                // TODO how this should be handled?
                type AsyncOutput = Self;

                #[track_caller]
                fn build(self) -> Self::State {
                    (move || self.get()).build()
                }

                fn try_build(self) -> any_error::Result<Self::FallibleState> {
                    (move || self.get()).try_build()
                }

                #[track_caller]
                fn rebuild(self, _state: &mut Self::State) {
                    // TODO rebuild
                }

                fn try_rebuild(
                    self,
                    state: &mut Self::FallibleState,
                ) -> any_error::Result<()> {
                    (move || self.get()).try_rebuild(state)
                }

                async fn resolve(self) -> Self::AsyncOutput {
                    self
                }
            }

            impl<V, R> RenderHtml<R> for $sig<V>
            where
                V: RenderHtml<R> + Clone + Send + Sync + 'static,
                V::State: 'static,
                V::FallibleState: 'static,
                R: Renderer + 'static,
            {
                const MIN_LENGTH: usize = 0;

                fn html_len(&self) -> usize {
                    V::MIN_LENGTH
                }

                fn to_html_with_buf(
                    self,
                    buf: &mut String,
                    position: &mut Position,
                ) {
                    let value = self.get();
                    value.to_html_with_buf(buf, position)
                }

                fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
                    self,
                    buf: &mut StreamBuilder,
                    position: &mut Position,
                ) where
                    Self: Sized,
                {
                    let value = self.get();
                    value.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
                }

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    cursor: &Cursor<R>,
                    position: &PositionState,
                ) -> Self::State {
                    (move || self.get())
                        .hydrate::<FROM_SERVER>(cursor, position)
                }
            }

            impl<V, R> AttributeValue<R> for $sig<V>
            where
                V: AttributeValue<R> + Clone + Send + Sync + 'static,
                V::State: 'static,
                R: Renderer,
            {
                type State = RenderEffectState<V::State>;

                fn html_len(&self) -> usize {
                    0
                }

                fn to_html(self, key: &str, buf: &mut String) {
                    let value = self.get();
                    value.to_html(key, buf);
                }

                fn to_template(_key: &str, _buf: &mut String) {}

                fn hydrate<const FROM_SERVER: bool>(
                    mut self,
                    key: &str,
                    el: &<R as Renderer>::Element,
                ) -> Self::State {
                    (move || self.get()).hydrate::<FROM_SERVER>(key, el)
                }

                fn build(
                    self,
                    el: &<R as Renderer>::Element,
                    key: &str,
                ) -> Self::State {
                    (move || self.get()).build(el, key)
                }

                fn rebuild(self, _key: &str, _state: &mut Self::State) {
                    // TODO rebuild
                }
            }
        };
    }

    macro_rules! signal_impl_unsend {
        ($sig:ident) => {
            impl<V, R> Render<R> for $sig<V>
            where
                V: Render<R> + Clone + 'static,
                V::State: 'static,
                V::FallibleState: 'static,
                R: Renderer,
            {
                type State = RenderEffectState<V::State>;
                type FallibleState = RenderEffectState<
                    Result<V::FallibleState, Option<AnyError>>,
                >;
                // TODO how this should be handled?
                type AsyncOutput = Self;

                #[track_caller]
                fn build(self) -> Self::State {
                    (move || self.get()).build()
                }

                fn try_build(self) -> any_error::Result<Self::FallibleState> {
                    (move || self.get()).try_build()
                }

                #[track_caller]
                fn rebuild(self, _state: &mut Self::State) {
                    // TODO rebuild
                }

                fn try_rebuild(
                    self,
                    state: &mut Self::FallibleState,
                ) -> any_error::Result<()> {
                    (move || self.get()).try_rebuild(state)
                }

                async fn resolve(self) -> Self::AsyncOutput {
                    self
                }
            }

            impl<V, R> RenderHtml<R> for $sig<V>
            where
                V: RenderHtml<R> + Clone + Send + Sync + 'static,
                V::State: 'static,
                V::FallibleState: 'static,
                R: Renderer + 'static,
            {
                const MIN_LENGTH: usize = 0;

                fn html_len(&self) -> usize {
                    V::MIN_LENGTH
                }

                fn to_html_with_buf(
                    self,
                    buf: &mut String,
                    position: &mut Position,
                ) {
                    let value = self.get();
                    value.to_html_with_buf(buf, position)
                }

                fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
                    self,
                    buf: &mut StreamBuilder,
                    position: &mut Position,
                ) where
                    Self: Sized,
                {
                    let value = self.get();
                    value.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
                }

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    cursor: &Cursor<R>,
                    position: &PositionState,
                ) -> Self::State {
                    (move || self.get())
                        .hydrate::<FROM_SERVER>(cursor, position)
                }
            }

            impl<V, R> AttributeValue<R> for $sig<V>
            where
                V: AttributeValue<R> + Clone + 'static,
                V::State: 'static,
                R: Renderer,
            {
                type State = RenderEffectState<V::State>;

                fn html_len(&self) -> usize {
                    0
                }

                fn to_html(self, key: &str, buf: &mut String) {
                    let value = self.get();
                    value.to_html(key, buf);
                }

                fn to_template(_key: &str, _buf: &mut String) {}

                fn hydrate<const FROM_SERVER: bool>(
                    mut self,
                    key: &str,
                    el: &<R as Renderer>::Element,
                ) -> Self::State {
                    (move || self.get()).hydrate::<FROM_SERVER>(key, el)
                }

                fn build(
                    self,
                    el: &<R as Renderer>::Element,
                    key: &str,
                ) -> Self::State {
                    (move || self.get()).build(el, key)
                }

                fn rebuild(self, _key: &str, _state: &mut Self::State) {
                    // TODO rebuild
                }
            }
        };
    }

    signal_impl!(RwSignal);
    signal_impl!(ReadSignal);
    signal_impl!(Memo);
    signal_impl!(Signal);
    signal_impl_unsend!(ArcRwSignal);
    signal_impl_unsend!(ArcReadSignal);
    signal_impl!(ArcMemo);
    signal_impl!(ArcSignal);
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
