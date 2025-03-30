use super::{ReactiveFunction, SharedReactiveFunction};
use crate::{html::class::IntoClass, renderer::Rndr};
use reactive_graph::effect::RenderEffect;
use std::borrow::Borrow;

pub struct RenderEffectWithClassName<T>
where
    T: 'static,
{
    name: &'static str,
    effect: RenderEffect<T>,
}

impl<T> RenderEffectWithClassName<T>
where
    T: 'static,
{
    fn new(name: &'static str, effect: RenderEffect<T>) -> Self {
        Self { effect, name }
    }
}

impl<F, C> IntoClass for F
where
    F: ReactiveFunction<Output = C>,
    C: IntoClass + 'static,
    C::State: 'static,
{
    type AsyncOutput = C::AsyncOutput;
    type State = RenderEffect<C::State>;
    type Cloneable = SharedReactiveFunction<C>;
    type CloneableOwned = SharedReactiveFunction<C>;

    fn html_len(&self) -> usize {
        0
    }

    fn to_html(mut self, class: &mut String) {
        let value = self.invoke();
        value.to_html(class);
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
        self.invoke().dry_resolve();
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

impl<F, T> IntoClass for (&'static str, F)
where
    F: ReactiveFunction<Output = T>,
    T: Borrow<bool> + Send + 'static,
{
    type AsyncOutput = (&'static str, bool);
    type State =
        RenderEffectWithClassName<(crate::renderer::types::ClassList, bool)>;
    type Cloneable = (&'static str, SharedReactiveFunction<T>);
    type CloneableOwned = (&'static str, SharedReactiveFunction<T>);

    fn html_len(&self) -> usize {
        self.0.len()
    }

    fn to_html(self, class: &mut String) {
        let (name, mut f) = self;
        let include = *f.invoke().borrow();
        if include {
            <&str as IntoClass>::to_html(name, class);
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        // TODO FROM_SERVER vs template
        let (name, mut f) = self;
        let class_list = Rndr::class_list(el);
        let name = Rndr::intern(name);

        RenderEffectWithClassName::new(
            name,
            RenderEffect::new(
                move |prev: Option<(
                    crate::renderer::types::ClassList,
                    bool,
                )>| {
                    let include = *f.invoke().borrow();
                    if let Some((class_list, prev)) = prev {
                        if include {
                            if !prev {
                                Rndr::add_class(&class_list, name);
                            }
                        } else if prev {
                            Rndr::remove_class(&class_list, name);
                        }
                    }
                    (class_list.clone(), include)
                },
            ),
        )
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let (name, mut f) = self;
        let class_list = Rndr::class_list(el);
        let name = Rndr::intern(name);

        RenderEffectWithClassName::new(
            name,
            RenderEffect::new(
                move |prev: Option<(
                    crate::renderer::types::ClassList,
                    bool,
                )>| {
                    let include = *f.invoke().borrow();
                    match prev {
                        Some((class_list, prev)) => {
                            if include {
                                if !prev {
                                    Rndr::add_class(&class_list, name);
                                }
                            } else if prev {
                                Rndr::remove_class(&class_list, name);
                            }
                        }
                        None => {
                            if include {
                                Rndr::add_class(&class_list, name);
                            }
                        }
                    }
                    (class_list.clone(), include)
                },
            ),
        )
    }

    fn rebuild(self, state: &mut Self::State) {
        let (name, mut f) = self;
        // Name might've updated:
        state.name = name;
        state.effect = RenderEffect::new_with_value(
            move |prev| {
                let include = *f.invoke().borrow();
                match prev {
                    Some((class_list, prev)) => {
                        if include {
                            if !prev {
                                Rndr::add_class(&class_list, name);
                            }
                        } else if prev {
                            Rndr::remove_class(&class_list, name);
                        }
                        (class_list.clone(), include)
                    }
                    None => {
                        unreachable!()
                    }
                }
            },
            state.effect.take_value(),
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

    async fn resolve(mut self) -> Self::AsyncOutput {
        (self.0, *self.1.invoke().borrow())
    }

    fn reset(state: &mut Self::State) {
        let name = state.name;
        state.effect = RenderEffect::new_with_value(
            move |prev| {
                if let Some(mut state) = prev {
                    let (class_list, prev) = &mut state;
                    Rndr::remove_class(class_list, name);
                    *prev = false;
                    state
                } else {
                    unreachable!()
                }
            },
            state.effect.take_value(),
        );
    }
}

// TODO this needs a non-reactive form too to be restored
/*
impl<F, T> IntoClass for (Vec<Cow<'static, str>>, F)
where
    F: ReactiveFunction<Output = T>,
    T: Borrow<bool> + Send + 'static,

{
    type AsyncOutput = (Vec<Cow<'static, str>>, bool);
    type State = RenderEffect<(crate::renderer::types::ClassList, bool)>;
    type Cloneable = (Vec<Cow<'static, str>>, SharedReactiveFunction<T>);
    type CloneableOwned = (Vec<Cow<'static, str>>, SharedReactiveFunction<T>);

    fn html_len(&self) -> usize {
        self.0.iter().map(|n| n.len()).sum()
    }

    fn to_html(self, class: &mut String) {
        let (names, mut f) = self;
        let include = *f.invoke().borrow();
        if include {
            for name in names {
                <&str as IntoClass>::to_html(&name, class);
            }
        }
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &crate::renderer::types::Element) -> Self::State {
        // TODO FROM_SERVER vs template
        let (names, mut f) = self;
        let class_list = Rndr::class_list(el);

        RenderEffect::new(move |prev: Option<(crate::renderer::types::ClassList, bool)>| {
            let include = *f.invoke().borrow();
            if let Some((class_list, prev)) = prev {
                if include {
                    if !prev {
                        for name in &names {
                            // TODO multi-class optimizations here
                            Rndr::add_class(&class_list, name);
                        }
                    }
                } else if prev {
                    for name in &names {
                        Rndr::remove_class(&class_list, name);
                    }
                }
            }
            (class_list.clone(), include)
        })
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let (names, mut f) = self;
        let class_list = Rndr::class_list(el);

        RenderEffect::new(move |prev: Option<(crate::renderer::types::ClassList, bool)>| {
            let include = *f.invoke().borrow();
            match prev {
                Some((class_list, prev)) => {
                    if include {
                        for name in &names {
                            if !prev {
                                Rndr::add_class(&class_list, name);
                            }
                        }
                    } else if prev {
                        for name in &names {
                            Rndr::remove_class(&class_list, name);
                        }
                    }
                }
                None => {
                    if include {
                        for name in &names {
                            Rndr::add_class(&class_list, name);
                        }
                    }
                }
            }
            (class_list.clone(), include)
        })
    }

    fn rebuild(self, state: &mut Self::State) {
        let (names, mut f) = self;
        let prev_value = state.take_value();

        *state = RenderEffect::new_with_value(
            move |prev: Option<(crate::renderer::types::ClassList, bool)>| {
                let include = *f.invoke().borrow();
                match prev {
                    Some((class_list, prev)) => {
                        if include {
                            for name in &names {
                                if !prev {
                                    Rndr::add_class(&class_list, name);
                                }
                            }
                        } else if prev {
                            for name in &names {
                                Rndr::remove_class(&class_list, name);
                            }
                        }
                        (class_list.clone(), include)
                    }
                    None => {
                        unreachable!()
                    }
                }
            },
            prev_value,
        );
    }

    fn into_cloneable(self) -> Self::Cloneable {
        (self.0.clone(), self.1.into_shared())
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        (self.0.clone(), self.1.into_shared())
    }

    fn dry_resolve(&mut self) {
        self.1.invoke();
    }

    async fn resolve(mut self) -> Self::AsyncOutput {
        (self.0, *self.1.invoke().borrow())
    }
}
*/

/*
impl<G> IntoClass for ReadGuard<String, G>
where
    G: Deref<Target = String> + Send,
{
    type AsyncOutput = Self;
    type State = <String as IntoClass>::State;
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, class: &mut String) {
        <&str as IntoClass>::to_html(self.deref().as_str(), class);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        <String as IntoClass>::hydrate::<FROM_SERVER>(
            self.deref().to_owned(),
            el,
        )
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        <String as IntoClass>::build(self.deref().to_owned(), el)
    }

    fn rebuild(self, state: &mut Self::State) {
        <String as IntoClass>::rebuild(self.deref().to_owned(), state)
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.as_str().into()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.as_str().into()
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl<G> IntoClass for (&'static str, ReadGuard<bool, G>)
where
    G: Deref<Target = bool> + Send,
{
    type AsyncOutput = Self;
    type State = <(&'static str, bool) as IntoClass>::State;
    type Cloneable = (&'static str, bool);
    type CloneableOwned = (&'static str, bool);

    fn html_len(&self) -> usize {
        self.0.len()
    }

    fn to_html(self, class: &mut String) {
        <(&'static str, bool) as IntoClass>::to_html(
            (self.0, *self.1.deref()),
            class,
        );
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        <(&'static str, bool) as IntoClass>::hydrate::<FROM_SERVER>(
            (self.0, *self.1.deref()),
            el,
        )
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        <(&'static str, bool) as IntoClass>::build(
            (self.0, *self.1.deref()),
            el,
        )
    }

    fn rebuild(self, state: &mut Self::State) {
        <(&'static str, bool) as IntoClass>::rebuild(
            (self.0, *self.1.deref()),
            state,
        )
    }

    fn into_cloneable(self) -> Self::Cloneable {
        (self.0, *self.1)
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        (self.0, *self.1)
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}
*/

macro_rules!  tuple_class_reactive {
    ($name:ident, <$($impl_gen:ident),*>, <$($gen:ident),*> , $( $where_clause:tt )*) =>
    {
        #[allow(deprecated)]
        impl<$($impl_gen),*>  IntoClass for (&'static str, $name<$($gen),*>)
        where
            $($where_clause)*
        {
            type AsyncOutput = Self;
            type State = RenderEffectWithClassName<(
                crate::renderer::types::ClassList,
                bool,
            )>;
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn html_len(&self) -> usize {
                self.0.len()
            }

            fn to_html(self, class: &mut String) {
                let (name, f) = self;
                let include = f.get();
                if include {
                    <&str as IntoClass>::to_html(name, class);
                }
            }

            fn hydrate<const FROM_SERVER: bool>(
                self,
                el: &crate::renderer::types::Element,
            ) -> Self::State {
                IntoClass::hydrate::<FROM_SERVER>(
                    (self.0, move || self.1.get()),
                    el,
                )
            }

            fn build(
                self,
                el: &crate::renderer::types::Element,
            ) -> Self::State {
                IntoClass::build((self.0, move || self.1.get()), el)
            }

            fn rebuild(self, state: &mut Self::State) {
                IntoClass::rebuild((self.0, move || self.1.get()), state)
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
                let name = state.name;
                *state = RenderEffectWithClassName::new(
                    state.name,
                    RenderEffect::new_with_value(
                        move |prev| {
                            if let Some(mut state) = prev {
                                let (class_list, prev) = &mut state;
                                Rndr::remove_class(class_list, name);
                                *prev = false;
                                state
                            } else {
                                unreachable!()
                            }
                        },
                        state.effect.take_value(),
                    ),
                );
            }
        }
    };
}

macro_rules!  class_reactive {
    ($name:ident, <$($gen:ident),*>, $v:ty, $( $where_clause:tt )*) =>
    {
        #[allow(deprecated)]
        impl<$($gen),*> IntoClass for $name<$($gen),*>
        where
            $v: IntoClass + Clone + Send + Sync + 'static,
            <$v as IntoClass>::State: 'static,
            $($where_clause)*
        {
            type AsyncOutput = Self;
            type State = RenderEffect<<$v as IntoClass>::State>;
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn html_len(&self) -> usize {
                0
            }

            fn to_html(self, class: &mut String) {
                let value = self.get();
                value.to_html(class);
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
    };
}

#[cfg(not(feature = "nightly"))]
mod stable {
    use super::{RenderEffect, RenderEffectWithClassName};
    use crate::{html::class::IntoClass, renderer::Rndr};
    #[allow(deprecated)]
    use reactive_graph::wrappers::read::MaybeSignal;
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        owner::Storage,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, Signal},
    };
    class_reactive!(
        RwSignal,
        <V, S>,
        V,
        RwSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    class_reactive!(
        ReadSignal,
        <V, S>,
        V,
        ReadSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    class_reactive!(
        Memo,
        <V, S>,
        V,
        Memo<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    class_reactive!(
        Signal,
        <V, S>,
        V,
        Signal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    class_reactive!(
        MaybeSignal,
        <V, S>,
        V,
        MaybeSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    class_reactive!(ArcRwSignal, <V>, V, ArcRwSignal<V>: Get<Value = V>);
    class_reactive!(ArcReadSignal, <V>, V, ArcReadSignal<V>: Get<Value = V>);
    class_reactive!(ArcMemo, <V>, V, ArcMemo<V>: Get<Value = V>);
    class_reactive!(ArcSignal, <V>, V, ArcSignal<V>: Get<Value = V>);

    tuple_class_reactive!(
        RwSignal,
        <S>,
        <bool, S>,
        RwSignal<bool, S>: Get<Value = bool>,
        S: Storage<bool>,
        S: Send  + 'static,
    );
    tuple_class_reactive!(
        ReadSignal,
        <S>,
        <bool, S>,
        ReadSignal<bool, S>: Get<Value = bool>,
        S: Storage<bool>,
        S: Send + 'static,
    );
    tuple_class_reactive!(
        Memo,
        <S>,
        <bool, S>,
        Memo<bool, S>: Get<Value = bool>,
        S: Storage<bool>,
        S: Send + 'static,
    );
    tuple_class_reactive!(
        Signal,
        <S>,
        <bool, S>,
        Signal<bool, S>: Get<Value = bool>,
        S: Storage<bool>,
        S: Send + 'static,
    );
    tuple_class_reactive!(
        MaybeSignal,
        <S>,
        <bool, S>,
        MaybeSignal<bool, S>: Get<Value = bool>,
        S: Storage<bool>,
        S: Send + 'static,
    );
    tuple_class_reactive!(ArcRwSignal,<>, <bool>, ArcRwSignal<bool>: Get<Value = bool>);
    tuple_class_reactive!(ArcReadSignal,<>, <bool>, ArcReadSignal<bool>: Get<Value = bool>);
    tuple_class_reactive!(ArcMemo,<>, <bool>, ArcMemo<bool>: Get<Value = bool>);
    tuple_class_reactive!(ArcSignal,<>, <bool>, ArcSignal<bool>: Get<Value = bool>);
}

#[cfg(feature = "reactive_stores")]
mod reactive_stores {
    use super::{RenderEffect, RenderEffectWithClassName};
    use crate::{html::class::IntoClass, renderer::Rndr};
    #[allow(deprecated)]
    use reactive_graph::{owner::Storage, traits::Get};
    use reactive_stores::{
        ArcField, ArcStore, AtIndex, AtKeyed, DerefedField, Field,
        KeyedSubfield, Store, StoreField, Subfield,
    };
    use std::ops::{Deref, DerefMut, Index, IndexMut};

    class_reactive!(
        Subfield,
        <Inner, Prev, V>,
        V,
        Subfield<Inner, Prev, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
    );

    class_reactive!(
        AtKeyed,
        <Inner, Prev, K, V>,
        V,
        AtKeyed<Inner, Prev, K, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
        K: Send + Sync + std::fmt::Debug + Clone + 'static,
        for<'a> &'a V: IntoIterator,
    );

    class_reactive!(
        KeyedSubfield,
        <Inner, Prev, K, V>,
        V,
        KeyedSubfield<Inner, Prev, K, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
        K: Send + Sync + std::fmt::Debug + Clone + 'static,
        for<'a> &'a V: IntoIterator,
    );

    class_reactive!(
        DerefedField,
        <S>,
        <S::Value as Deref>::Target,
        S: Clone + StoreField + Send + Sync + 'static,
        <S as StoreField>::Value: Deref + DerefMut
    );

    class_reactive!(
        AtIndex,
        <Inner, Prev>,
        <Prev as Index<usize>>::Output,
        AtIndex<Inner, Prev>: Get<Value = Prev::Output>,
        Prev: Send + Sync + IndexMut<usize> + 'static,
        Inner: Send + Sync + Clone + 'static,
    );
    class_reactive!(
        Store,
        <V, S>,
        V,
        Store<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    class_reactive!(
        Field,
        <V, S>,
        V,
        Field<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    class_reactive!(ArcStore, <V>, V, ArcStore<V>: Get<Value = V>);
    class_reactive!(ArcField, <V>, V, ArcField<V>: Get<Value = V>);

    tuple_class_reactive!(
        Subfield,
        <Inner, Prev>,
        <Inner, Prev, bool>,
        Subfield<Inner, Prev, bool>: Get<Value = bool>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
    );

    tuple_class_reactive!(
        AtKeyed,
        <Inner, Prev, K>,
        <Inner, Prev, K, bool>,
        AtKeyed<Inner, Prev, K, bool>: Get<Value = bool>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
        K: Send + Sync + std::fmt::Debug + Clone + 'static,
        for<'a> &'a bool: IntoIterator,
    );

    tuple_class_reactive!(
        KeyedSubfield,
        <Inner, Prev, K>,
        <Inner, Prev, K, bool>,
        KeyedSubfield<Inner, Prev, K, bool>: Get<Value = bool>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
        K: Send + Sync + std::fmt::Debug + Clone + 'static,
        for<'a> &'a bool: IntoIterator,
    );

    tuple_class_reactive!(
        DerefedField,
        <S>,
        <S>,
        S: Clone + StoreField + Send + Sync + 'static,
        <S as StoreField>::Value: Deref<Target = bool> + DerefMut
    );

    tuple_class_reactive!(
        AtIndex,
        <Inner, Prev>,
        <Inner, Prev>,
        AtIndex<Inner, Prev>: Get<Value = Prev::Output>,
        Prev: Send + Sync + IndexMut<usize,Output = bool> + 'static,
        Inner: Send + Sync + Clone + 'static,
    );
    tuple_class_reactive!(
        Store,
        <S>,
        <bool, S>,
        Store<bool, S>: Get<Value = bool>,
        S: Storage<bool>,
        S: Send  + 'static,
    );
    tuple_class_reactive!(
        Field,
        <S>,
        <bool, S>,
        Field<bool, S>: Get<Value = bool>,
        S: Storage<bool>,
        S: Send  + 'static,
    );
    tuple_class_reactive!(ArcStore,<>, <bool>, ArcStore<bool>: Get<Value = bool>);
    tuple_class_reactive!(ArcField,<>, <bool>, ArcField<bool>: Get<Value = bool>);
}

/*
impl<Fut> IntoClass for Suspend<Fut>
where
    Fut: Clone + Future + Send + 'static,
    Fut::Output: IntoClass,
{
    type AsyncOutput = Fut::Output;
    type State = Rc<RefCell<Option<<Fut::Output as IntoClass>::State>>>;
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn html_len(&self) -> usize {
        0
    }

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
