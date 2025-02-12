use super::{ReactiveFunction, SharedReactiveFunction};
use crate::html::element::InnerHtmlValue;
use reactive_graph::effect::RenderEffect;

impl<F, V> InnerHtmlValue for F
where
    F: ReactiveFunction<Output = V>,
    V: InnerHtmlValue + 'static,
    V::State: 'static,
{
    type AsyncOutput = V::AsyncOutput;
    type State = RenderEffect<V::State>;
    type Cloneable = SharedReactiveFunction<V>;
    type CloneableOwned = SharedReactiveFunction<V>;

    fn html_len(&self) -> usize {
        0
    }

    fn to_html(mut self, buf: &mut String) {
        let value = self.invoke();
        value.to_html(buf);
    }

    fn to_template(_buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        mut self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let el = el.to_owned();
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
        let el = el.to_owned();
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
    use crate::html::element::InnerHtmlValue;
    #[allow(deprecated)]
    use reactive_graph::wrappers::read::MaybeSignal;
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        effect::RenderEffect,
        owner::Storage,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, Signal},
    };
    #[cfg(feature = "reactive_stores")]
    use {
        reactive_stores::{
            ArcField, ArcStore, AtIndex, AtKeyed, DerefedField, Field,
            KeyedSubfield, Store, StoreField, Subfield,
        },
        std::ops::{Deref, DerefMut, Index, IndexMut},
    };

    macro_rules! inner_html_signal {
        ($sig:ident) => {
            impl<V> InnerHtmlValue for $sig<V>
            where
                $sig<V>: Get<Value = V>,
                V: InnerHtmlValue + Send + Sync + Clone + 'static,
                V::State: 'static,
            {
                type AsyncOutput = Self;
                type State = RenderEffect<Option<V::State>>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn html_len(&self) -> usize {
                    0
                }

                fn to_html(self, buf: &mut String) {
                    let value = self.try_get();
                    value.to_html(buf);
                }

                fn to_template(_buf: &mut String) {}

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &crate::renderer::types::Element,
                ) -> Self::State {
                    let el = el.to_owned();
                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        // Outer Some means there was a previous state
                        // Inner Some means the previous state was valid
                        // (i.e., the signal was successfully accessed)
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state);
                                Some(state)
                            }
                            (None, Some(value)) => {
                                Some(value.hydrate::<FROM_SERVER>(&el))
                            }
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => {
                                Some(value.hydrate::<FROM_SERVER>(&el))
                            }
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn build(
                    self,
                    el: &crate::renderer::types::Element,
                ) -> Self::State {
                    let el = el.to_owned();
                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        // Outer Some means there was a previous state
                        // Inner Some means the previous state was valid
                        // (i.e., the signal was successfully accessed)
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state);
                                Some(state)
                            }
                            (None, Some(value)) => Some(value.build(&el)),
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => Some(value.build(&el)),
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn rebuild(self, state: &mut Self::State) {
                    let prev_value = state.take_value();
                    *state = RenderEffect::new_with_value(
                        move |prev| {
                            let value = self.try_get();
                            match (prev, value) {
                                (Some(Some(mut state)), Some(value)) => {
                                    value.rebuild(&mut state);
                                    Some(state)
                                }
                                (Some(Some(state)), None) => Some(state),
                                (Some(None), Some(_)) => None,
                                (Some(None), None) => None,
                                (None, Some(_)) => None, // unreachable!()
                                (None, None) => None,    // unreachable!()
                            }
                        },
                        prev_value,
                    );
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

    macro_rules! inner_html_signal_arena {
        ($sig:ident) => {
            #[allow(deprecated)]
            impl<V, S> InnerHtmlValue for $sig<V, S>
            where
                $sig<V, S>: Get<Value = V>,
                S: Send + Sync + 'static,
                S: Storage<V>,
                V: InnerHtmlValue + Send + Sync + Clone + 'static,
                V::State: 'static,
            {
                type AsyncOutput = Self;
                type State = RenderEffect<Option<V::State>>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn html_len(&self) -> usize {
                    0
                }

                fn to_html(self, buf: &mut String) {
                    let value = self.try_get();
                    value.to_html(buf);
                }

                fn to_template(_buf: &mut String) {}

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &crate::renderer::types::Element,
                ) -> Self::State {
                    let el = el.to_owned();
                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        // Outer Some means there was a previous state
                        // Inner Some means the previous state was valid
                        // (i.e., the signal was successfully accessed)
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state);
                                Some(state)
                            }
                            (None, Some(value)) => {
                                Some(value.hydrate::<FROM_SERVER>(&el))
                            }
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => {
                                Some(value.hydrate::<FROM_SERVER>(&el))
                            }
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn build(
                    self,
                    el: &crate::renderer::types::Element,
                ) -> Self::State {
                    let el = el.to_owned();
                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        // Outer Some means there was a previous state
                        // Inner Some means the previous state was valid
                        // (i.e., the signal was successfully accessed)
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state);
                                Some(state)
                            }
                            (None, Some(value)) => Some(value.build(&el)),
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => Some(value.build(&el)),
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn rebuild(self, state: &mut Self::State) {
                    let prev_value = state.take_value();
                    *state = RenderEffect::new_with_value(
                        move |prev| {
                            let value = self.try_get();
                            match (prev, value) {
                                (Some(Some(mut state)), Some(value)) => {
                                    value.rebuild(&mut state);
                                    Some(state)
                                }
                                (Some(Some(state)), None) => Some(state),
                                (Some(None), Some(_)) => None,
                                (Some(None), None) => None,
                                (None, Some(_)) => None, // unreachable!()
                                (None, None) => None,    // unreachable!()
                            }
                        },
                        prev_value,
                    );
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

    #[cfg(feature = "reactive_stores")]
    macro_rules! inner_html_store_field {
        ($name:ident, <$($gen:ident),*>, $v:ty, $( $where_clause:tt )*) =>
        {
            impl<$($gen),*> InnerHtmlValue for $name<$($gen),*>
            where
                $v: InnerHtmlValue + Clone + Send + Sync + 'static,
                <$v as InnerHtmlValue>::State: 'static,
                $($where_clause)*
            {
                type AsyncOutput = Self;
                type State = RenderEffect<Option<<$v as InnerHtmlValue>::State>>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn html_len(&self) -> usize {
                    0
                }

                fn to_html(self, buf: &mut String) {
                    let value = self.try_get();
                    value.to_html(buf);
                }

                fn to_template(_buf: &mut String) {}

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &crate::renderer::types::Element,
                ) -> Self::State {
                    let el = el.to_owned();
                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        // Outer Some means there was a previous state
                        // Inner Some means the previous state was valid
                        // (i.e., the signal was successfully accessed)
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state);
                                Some(state)
                            }
                            (None, Some(value)) => {
                                Some(value.hydrate::<FROM_SERVER>(&el))
                            }
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => {
                                Some(value.hydrate::<FROM_SERVER>(&el))
                            }
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn build(
                    self,
                    el: &crate::renderer::types::Element,
                ) -> Self::State {
                    let el = el.to_owned();
                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        // Outer Some means there was a previous state
                        // Inner Some means the previous state was valid
                        // (i.e., the signal was successfully accessed)
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state);
                                Some(state)
                            }
                            (None, Some(value)) => Some(value.build(&el)),
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => Some(value.build(&el)),
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn rebuild(self, state: &mut Self::State) {
                    let prev_value = state.take_value();
                    *state = RenderEffect::new_with_value(
                        move |prev| {
                            let value = self.try_get();
                            match (prev, value) {
                                (Some(Some(mut state)), Some(value)) => {
                                    value.rebuild(&mut state);
                                    Some(state)
                                }
                                (Some(Some(state)), None) => Some(state),
                                (Some(None), Some(_)) => None,
                                (Some(None), None) => None,
                                (None, Some(_)) => None, // unreachable!()
                                (None, None) => None,    // unreachable!()
                            }
                        },
                        prev_value,
                    );
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

    #[cfg(feature = "reactive_stores")]
    inner_html_store_field!(
        Subfield,
        <Inner, Prev, V>,
        V,
        Subfield<Inner, Prev, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
    );

    #[cfg(feature = "reactive_stores")]
    inner_html_store_field!(
        AtKeyed,
        <Inner, Prev, K, V>,
        V,
        AtKeyed<Inner, Prev, K, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
        K: Send + Sync + std::fmt::Debug + Clone + 'static,
        for<'a> &'a V: IntoIterator,
    );

    #[cfg(feature = "reactive_stores")]
    inner_html_store_field!(
        KeyedSubfield,
        <Inner, Prev, K, V>,
        V,
        KeyedSubfield<Inner, Prev, K, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
        K: Send + Sync + std::fmt::Debug + Clone + 'static,
        for<'a> &'a V: IntoIterator,
    );

    #[cfg(feature = "reactive_stores")]
    inner_html_store_field!(
        DerefedField,
        <S>,
        <S::Value as Deref>::Target,
        S: Clone + StoreField + Send + Sync + 'static,
        <S as StoreField>::Value: Deref + DerefMut
    );

    #[cfg(feature = "reactive_stores")]
    inner_html_store_field!(
        AtIndex,
        <Inner, Prev>,
        <Prev as Index<usize>>::Output,
        AtIndex<Inner, Prev>: Get<Value = Prev::Output>,
        Prev: Send + Sync + IndexMut<usize> + 'static,
        Inner: Send + Sync + Clone + 'static,
    );

    #[cfg(feature = "reactive_stores")]
    inner_html_signal_arena!(Store);
    #[cfg(feature = "reactive_stores")]
    inner_html_signal_arena!(Field);
    inner_html_signal_arena!(RwSignal);
    inner_html_signal_arena!(ReadSignal);
    inner_html_signal_arena!(Memo);
    inner_html_signal_arena!(Signal);
    inner_html_signal_arena!(MaybeSignal);
    #[cfg(feature = "reactive_stores")]
    inner_html_signal!(ArcStore);
    #[cfg(feature = "reactive_stores")]
    inner_html_signal!(ArcField);
    inner_html_signal!(ArcRwSignal);
    inner_html_signal!(ArcReadSignal);
    inner_html_signal!(ArcMemo);
    inner_html_signal!(ArcSignal);
}
