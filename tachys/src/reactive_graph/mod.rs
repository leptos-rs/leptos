use crate::{
    html::attribute::{Attribute, AttributeValue},
    hydration::Cursor,
    renderer::Renderer,
    ssr::StreamBuilder,
    view::{
        add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
        RenderHtml, ToTemplate,
    },
};
use reactive_graph::effect::RenderEffect;
use std::sync::{Arc, Mutex};

mod class;
mod guards;
mod inner_html;
pub mod node_ref;
mod owned;
mod property;
mod style;
mod suspense;
pub use owned::*;
pub use suspense::*;

impl<F, V> ToTemplate for F
where
    F: ReactiveFunction<Output = V>,
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
    F: ReactiveFunction<Output = V>,
    V: Render<R>,
    V::State: 'static,
    R: Renderer,
{
    type State = RenderEffectState<V::State>;

    #[track_caller]
    fn build(self) -> Self::State {
        RenderEffect::new(move |prev| {
            let value = self.invoke();
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
    fn rebuild(self, _state: &mut Self::State) {
        // TODO rebuild
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
    F: ReactiveFunction<Output = V>,
    V: RenderHtml<R> + 'static,
    V::State: 'static,

    R: Renderer + 'static,
{
    type AsyncOutput = V::AsyncOutput;

    const MIN_LENGTH: usize = 0;

    fn dry_resolve(&self) {
        self.invoke().dry_resolve();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        self.invoke().resolve().await
    }

    fn html_len(&self) -> usize {
        V::MIN_LENGTH
    }

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        let value = self.invoke();
        value.to_html_with_buf(buf, position)
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        let value = self.invoke();
        value.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        let cursor = cursor.clone();
        let position = position.clone();
        RenderEffect::new(move |prev| {
            let value = self.invoke();
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
    F: ReactiveFunction<Output = V>,
    V: RenderHtml<R> + 'static,
    R: Renderer + 'static,
{
    type Output<SomeNewAttr: Attribute<R>> =
        Box<dyn Fn() -> V::Output<SomeNewAttr::CloneableOwned> + Send>;

    fn add_any_attr<NewAttr: Attribute<R>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<R>,
    {
        let attr = attr.into_cloneable_owned();
        Box::new(move || self.invoke().add_any_attr(attr.clone()))
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
    F: ReactiveFunction<Output = V>,
    V: AttributeValue<R> + 'static,
    V::State: 'static,
    R: Renderer,
{
    type State = RenderEffectState<V::State>;
    type Cloneable = SharedReactiveFunction<V>;
    type CloneableOwned = SharedReactiveFunction<V>;

    fn html_len(&self) -> usize {
        0
    }

    fn to_html(self, key: &str, buf: &mut String) {
        let value = self.invoke();
        value.to_html(key, buf);
    }

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        key: &str,
        el: &<R as Renderer>::Element,
    ) -> Self::State {
        let key = R::intern(key);
        let key = key.to_owned();
        let el = el.to_owned();

        RenderEffect::new(move |prev| {
            let value = self.invoke();
            if let Some(mut state) = prev {
                value.rebuild(&key, &mut state);
                state
            } else {
                value.hydrate::<FROM_SERVER>(&key, &el)
            }
        })
        .into()
    }

    fn build(self, el: &<R as Renderer>::Element, key: &str) -> Self::State {
        let key = R::intern(key);
        let key = key.to_owned();
        let el = el.to_owned();

        RenderEffect::new(move |prev| {
            let value = self.invoke();
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

    fn into_cloneable(self) -> Self::Cloneable {
        self.into_shared()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into_shared()
    }
}

pub type SharedReactiveFunction<T> = Arc<Mutex<dyn Fn() -> T + Send>>;

pub trait ReactiveFunction: Send + 'static {
    type Output;

    fn invoke(&self) -> Self::Output;

    fn into_shared(self) -> Arc<Mutex<dyn Fn() -> Self::Output + Send>>;
}

impl<T: 'static> ReactiveFunction for Arc<Mutex<dyn Fn() -> T + Send>> {
    type Output = T;

    fn invoke(&self) -> Self::Output {
        let fun = self.lock().expect("lock poisoned");
        fun()
    }

    fn into_shared(self) -> Arc<Mutex<dyn Fn() -> Self::Output + Send>> {
        self
    }
}

impl<F, T> ReactiveFunction for F
where
    F: Fn() -> T + Send + 'static,
{
    type Output = T;

    fn invoke(&self) -> Self::Output {
        self()
    }

    fn into_shared(self) -> Arc<Mutex<dyn Fn() -> Self::Output + Send>> {
        Arc::new(Mutex::new(self))
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
        view::{
            add_attr::AddAnyAttr, Position, PositionState, Render, RenderHtml,
        },
    };
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

                R: Renderer,
            {
                type State = RenderEffectState<V::State>;

                #[track_caller]
                fn build(self) -> Self::State {
                    (move || self.get()).build()
                }

                #[track_caller]
                fn rebuild(self, _state: &mut Self::State) {
                    // TODO rebuild
                }
            }

            impl<V, R> AddAnyAttr<R> for $sig<V>
            where
                V: RenderHtml<R> + Clone + Send + Sync + 'static,
                V::State: 'static,
                R: Renderer + 'static,
            {
                type Output<SomeNewAttr: Attribute<R>> = $sig<V>;

                fn add_any_attr<NewAttr: Attribute<R>>(
                    self,
                    _attr: NewAttr,
                ) -> Self::Output<NewAttr>
                where
                    Self::Output<NewAttr>: RenderHtml<R>,
                {
                    todo!()
                }
            }

            impl<V, R> RenderHtml<R> for $sig<V>
            where
                V: RenderHtml<R> + Clone + Send + Sync + 'static,
                V::State: 'static,

                R: Renderer + 'static,
            {
                type AsyncOutput = Self;

                const MIN_LENGTH: usize = 0;

                fn dry_resolve(&self) {}

                async fn resolve(self) -> Self::AsyncOutput {
                    self
                }

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
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn html_len(&self) -> usize {
                    0
                }

                fn to_html(self, key: &str, buf: &mut String) {
                    let value = self.get();
                    value.to_html(key, buf);
                }

                fn to_template(_key: &str, _buf: &mut String) {}

                fn hydrate<const FROM_SERVER: bool>(
                    self,
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

                fn into_cloneable(self) -> Self::Cloneable {
                    self
                }

                fn into_cloneable_owned(self) -> Self::CloneableOwned {
                    self
                }
            }
        };
    }

    macro_rules! signal_impl_unsend {
        ($sig:ident) => {
            impl<V, R> Render<R> for $sig<V>
            where
                V: Render<R> + Send + Sync + Clone + 'static,
                V::State: 'static,

                R: Renderer,
            {
                type State = RenderEffectState<V::State>;

                #[track_caller]
                fn build(self) -> Self::State {
                    (move || self.get()).build()
                }

                #[track_caller]
                fn rebuild(self, _state: &mut Self::State) {
                    // TODO rebuild
                }
            }

            impl<V, R> AddAnyAttr<R> for $sig<V>
            where
                V: RenderHtml<R> + Clone + Send + Sync + 'static,
                V::State: 'static,
                R: Renderer + 'static,
            {
                type Output<SomeNewAttr: Attribute<R>> = $sig<V>;

                fn add_any_attr<NewAttr: Attribute<R>>(
                    self,
                    _attr: NewAttr,
                ) -> Self::Output<NewAttr>
                where
                    Self::Output<NewAttr>: RenderHtml<R>,
                {
                    todo!()
                }
            }

            impl<V, R> RenderHtml<R> for $sig<V>
            where
                V: RenderHtml<R> + Clone + Send + Sync + 'static,
                V::State: 'static,

                R: Renderer + 'static,
            {
                type AsyncOutput = Self;

                const MIN_LENGTH: usize = 0;

                fn dry_resolve(&self) {}

                async fn resolve(self) -> Self::AsyncOutput {
                    self
                }

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
                V: AttributeValue<R> + Send + Sync + Clone + 'static,
                V::State: 'static,
                R: Renderer,
            {
                type State = RenderEffectState<V::State>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn html_len(&self) -> usize {
                    0
                }

                fn to_html(self, key: &str, buf: &mut String) {
                    let value = self.get();
                    value.to_html(key, buf);
                }

                fn to_template(_key: &str, _buf: &mut String) {}

                fn hydrate<const FROM_SERVER: bool>(
                    self,
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

                fn into_cloneable(self) -> Self::Cloneable {
                    self
                }

                fn into_cloneable_owned(self) -> Self::CloneableOwned {
                    self
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
