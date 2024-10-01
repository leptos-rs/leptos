use super::{ReactiveFunction, SharedReactiveFunction, Suspend};
use crate::{html::class::IntoClass, renderer::Rndr};
use futures::FutureExt;
use reactive_graph::{effect::RenderEffect, signal::guards::ReadGuard};
use std::{
    borrow::Borrow, cell::RefCell, future::Future, ops::Deref, rc::Rc,
    sync::Arc,
};

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
}

impl<F, T> IntoClass for (&'static str, F)
where
    F: ReactiveFunction<Output = T>,
    T: Borrow<bool> + Send + 'static,
{
    type AsyncOutput = (&'static str, bool);
    type State = RenderEffect<(crate::renderer::types::ClassList, bool)>;
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

        RenderEffect::new(
            move |prev: Option<(crate::renderer::types::ClassList, bool)>| {
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
        )
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let (name, mut f) = self;
        let class_list = Rndr::class_list(el);
        let name = Rndr::intern(name);

        RenderEffect::new(
            move |prev: Option<(crate::renderer::types::ClassList, bool)>| {
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
        )
    }

    fn rebuild(self, state: &mut Self::State) {
        let (name, mut f) = self;
        let prev_value = state.take_value();
        *state = RenderEffect::new_with_value(
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

    async fn resolve(mut self) -> Self::AsyncOutput {
        (self.0, *self.1.invoke().borrow())
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

#[cfg(not(feature = "nightly"))]
mod stable {
    macro_rules! class_signal_arena {
        ($sig:ident) => {
            impl<C, S> IntoClass for $sig<C, S>
            where
                $sig<C, S>: Get<Value = C>,
                S: Send + Sync + 'static,
                S: Storage<C> + Storage<Option<C>>,
                C: IntoClass + Send + Sync + Clone + 'static,
                C::State: 'static,
            {
                type AsyncOutput = Self;
                type State = RenderEffect<C::State>;
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
            }

            impl<S> IntoClass for (&'static str, $sig<bool, S>)
            where
                $sig<bool, S>: Get<Value = bool>,
                S: Send + 'static,
                S: Storage<bool>,
            {
                type AsyncOutput = Self;
                type State =
                    RenderEffect<(crate::renderer::types::ClassList, bool)>;
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
            }
        };
    }

    macro_rules! class_signal {
        ($sig:ident) => {
            impl<C> IntoClass for $sig<C>
            where
                $sig<C>: Get<Value = C>,
                C: IntoClass + Send + Sync + Clone + 'static,
                C::State: 'static,
            {
                type AsyncOutput = Self;
                type State = RenderEffect<C::State>;
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
            }

            impl IntoClass for (&'static str, $sig<bool>)
            where
                $sig<bool>: Get<Value = bool>,
            {
                type AsyncOutput = Self;
                type State =
                    RenderEffect<(crate::renderer::types::ClassList, bool)>;
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
            }
        };
    }

    use super::RenderEffect;
    use crate::html::class::IntoClass;
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        owner::Storage,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, MaybeSignal, Signal},
    };

    class_signal_arena!(RwSignal);
    class_signal_arena!(ReadSignal);
    class_signal_arena!(Memo);
    class_signal_arena!(Signal);
    class_signal_arena!(MaybeSignal);
    class_signal!(ArcRwSignal);
    class_signal!(ArcReadSignal);
    class_signal!(ArcMemo);
    class_signal!(ArcSignal);
}

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
