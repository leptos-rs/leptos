use super::{ReactiveFunction, SharedReactiveFunction};
use crate::{
    html::style::{IntoStyle, IntoStyleValue},
    renderer::Rndr,
};
use reactive_graph::effect::RenderEffect;
use std::sync::Arc;

impl<F, S> IntoStyleValue for F
where
    F: ReactiveFunction<Output = S>,
    S: IntoStyleValue + 'static,
{
    type AsyncOutput = Self;
    type State = (Arc<str>, RenderEffect<S::State>);
    type Cloneable = SharedReactiveFunction<S>;
    type CloneableOwned = SharedReactiveFunction<S>;

    fn to_html(self, name: &str, style: &mut String) {
        let mut f = self;
        let value = f.invoke();
        value.to_html(name, style);
    }

    fn build(
        mut self,
        style: &crate::renderer::dom::CssStyleDeclaration,
        name: &str,
    ) -> Self::State {
        let name: Arc<str> = Rndr::intern(name).into();
        let style = style.to_owned();
        (
            Arc::clone(&name),
            RenderEffect::new(move |prev| {
                let value = self.invoke();
                if let Some(mut state) = prev {
                    value.rebuild(&style, &name, &mut state);
                    state
                } else {
                    value.build(&style, &name)
                }
            }),
        )
    }

    fn rebuild(
        mut self,
        style: &crate::renderer::dom::CssStyleDeclaration,
        name: &str,
        state: &mut Self::State,
    ) {
        let (prev_name, prev_effect) = state;
        let mut prev_value = prev_effect.take_value();
        if name != prev_name.as_ref() {
            Rndr::remove_css_property(style, prev_name.as_ref());
            prev_value = None;
        }
        let name: Arc<str> = name.into();
        let style = style.to_owned();

        *state = (
            Arc::clone(&name),
            RenderEffect::new_with_value(
                move |prev| {
                    let value = self.invoke();
                    if let Some(mut state) = prev {
                        value.rebuild(&style, &name, &mut state);
                        state
                    } else {
                        value.build(&style, &name)
                    }
                },
                prev_value,
            ),
        );
    }

    fn hydrate(
        mut self,
        style: &crate::renderer::dom::CssStyleDeclaration,
        name: &str,
    ) -> Self::State {
        let name: Arc<str> = Rndr::intern(name).into();
        let style = style.to_owned();
        (
            Arc::clone(&name),
            RenderEffect::new(move |prev| {
                let value = self.invoke();
                if let Some(mut state) = prev {
                    value.rebuild(&style, &name, &mut state);
                    state
                } else {
                    value.hydrate(&style, &name)
                }
            }),
        )
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

    fn reset(state: &mut Self::State) {
        *state = RenderEffect::new_with_value(
            move |prev| {
                if let Some(mut state) = prev {
                    C::reset(&mut state);
                    state
                } else {
                    unreachable!()
                }
            },
            state.take_value(),
        );
    }
}

macro_rules! style_reactive {
    ($name:ident, <$($gen:ident),*>, $v:ty, $( $where_clause:tt )*) =>
    {
        #[allow(deprecated)]
        impl<$($gen),*> IntoStyle for $name<$($gen),*>
        where
            $v: IntoStyle + Clone + Send + Sync + 'static,
            <$v as IntoStyle>::State: 'static,
            $($where_clause)*
        {
            type AsyncOutput = Self;
            type State = RenderEffect<<$v as IntoStyle>::State>;
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

            fn reset(state: &mut Self::State) {
                *state = RenderEffect::new_with_value(
                    move |prev| {
                        if let Some(mut state) = prev {
                            <$v>::reset(&mut state);
                            state
                        } else {
                            unreachable!()
                        }
                    },
                    state.take_value(),
                );
            }
        }

        #[allow(deprecated)]
        impl<$($gen),*> IntoStyleValue for $name<$($gen),*>
        where
            $v: IntoStyleValue + Send + Sync + Clone + 'static,
            $($where_clause)*
        {
            type AsyncOutput = Self;
            type State = (Arc<str>, RenderEffect<<$v as IntoStyleValue>::State>);
            type Cloneable = $name<$($gen),*>;
            type CloneableOwned = $name<$($gen),*>;

            fn to_html(self, name: &str, style: &mut String) {
                IntoStyleValue::to_html(move || self.get(), name, style)
            }

            fn build(
                self,
                style: &crate::renderer::dom::CssStyleDeclaration,
                name: &str,
            ) -> Self::State {
                IntoStyleValue::build(move || self.get(), style, name)
            }

            fn rebuild(
                self,
                style: &crate::renderer::dom::CssStyleDeclaration,
                name: &str,
                state: &mut Self::State,
            ) {
                IntoStyleValue::rebuild(
                    move || self.get(),
                    style,
                    name,
                    state,
                )
            }

            fn hydrate(
                self,
                style: &crate::renderer::dom::CssStyleDeclaration,
                name: &str,
            ) -> Self::State {
                IntoStyleValue::hydrate(move || self.get(), style, name)
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

#[cfg(not(feature = "nightly"))]
mod stable {
    use super::RenderEffect;
    use crate::html::style::{IntoStyle, IntoStyleValue};
    #[allow(deprecated)]
    use reactive_graph::wrappers::read::MaybeSignal;
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        owner::Storage,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, Signal},
    };
    use std::sync::Arc;

    style_reactive!(
        RwSignal,
        <V, S>,
        V,
        RwSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    style_reactive!(
        ReadSignal,
        <V, S>,
        V,
        ReadSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    style_reactive!(
        Memo,
        <V, S>,
        V,
        Memo<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    style_reactive!(
        Signal,
        <V, S>,
        V,
        Signal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    style_reactive!(
        MaybeSignal,
        <V, S>,
        V,
        MaybeSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    style_reactive!(ArcRwSignal, <V>, V, ArcRwSignal<V>: Get<Value = V>);
    style_reactive!(ArcReadSignal, <V>, V, ArcReadSignal<V>: Get<Value = V>);
    style_reactive!(ArcMemo, <V>, V, ArcMemo<V>: Get<Value = V>);
    style_reactive!(ArcSignal, <V>, V, ArcSignal<V>: Get<Value = V>);
}

#[cfg(feature = "reactive_stores")]
mod reactive_stores {
    use super::RenderEffect;
    use crate::html::style::{IntoStyle, IntoStyleValue};
    #[allow(deprecated)]
    use reactive_graph::{owner::Storage, traits::Get};
    use reactive_stores::{
        ArcField, ArcStore, AtIndex, AtKeyed, DerefedField, Field,
        KeyedSubfield, Store, StoreField, Subfield,
    };
    use std::{
        ops::{Deref, DerefMut, Index, IndexMut},
        sync::Arc,
    };

    style_reactive!(
        Subfield,
        <Inner, Prev, V>,
        V,
        Subfield<Inner, Prev, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
    );

    style_reactive!(
        AtKeyed,
        <Inner, Prev, K, V>,
        V,
        AtKeyed<Inner, Prev, K, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
        K: Send + Sync + std::fmt::Debug + Clone + 'static,
        for<'a> &'a V: IntoIterator,
    );

    style_reactive!(
        KeyedSubfield,
        <Inner, Prev, K, V>,
        V,
        KeyedSubfield<Inner, Prev, K, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
        K: Send + Sync + std::fmt::Debug + Clone + 'static,
        for<'a> &'a V: IntoIterator,
    );

    style_reactive!(
        DerefedField,
        <S>,
        <S::Value as Deref>::Target,
        S: Clone + StoreField + Send + Sync + 'static,
        <S as StoreField>::Value: Deref + DerefMut
    );

    style_reactive!(
        AtIndex,
        <Inner, Prev>,
        <Prev as Index<usize>>::Output,
        AtIndex<Inner, Prev>: Get<Value = Prev::Output>,
        Prev: Send + Sync + IndexMut<usize> + 'static,
        Inner: Send + Sync + Clone + 'static,
    );
    style_reactive!(
        Store,
        <V, S>,
        V,
        Store<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    style_reactive!(
        Field,
        <V, S>,
        V,
        Field<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    style_reactive!(ArcStore, <V>, V, ArcStore<V>: Get<Value = V>);
    style_reactive!(ArcField, <V>, V, ArcField<V>: Get<Value = V>);
}
/*
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
*/
