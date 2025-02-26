use super::{Attribute, NextAttribute};
use crate::erased::{Erased, ErasedLocal};
use std::{any::TypeId, fmt::Debug};
#[cfg(feature = "ssr")]
use std::{future::Future, pin::Pin};

/// A type-erased container for any [`Attribute`].
pub struct AnyAttribute {
    type_id: TypeId,
    html_len: usize,
    value: Erased,
    clone: fn(&Erased) -> AnyAttribute,
    #[cfg(feature = "ssr")]
    to_html: fn(Erased, &mut String, &mut String, &mut String, &mut String),
    build: fn(Erased, el: crate::renderer::types::Element) -> AnyAttributeState,
    rebuild: fn(Erased, &mut AnyAttributeState),
    #[cfg(feature = "hydrate")]
    hydrate_from_server:
        fn(Erased, crate::renderer::types::Element) -> AnyAttributeState,
    #[cfg(feature = "hydrate")]
    hydrate_from_template:
        fn(Erased, crate::renderer::types::Element) -> AnyAttributeState,
    #[cfg(feature = "ssr")]
    #[allow(clippy::type_complexity)]
    resolve: fn(Erased) -> Pin<Box<dyn Future<Output = AnyAttribute> + Send>>,
    #[cfg(feature = "ssr")]
    dry_resolve: fn(&mut Erased),
}

impl Clone for AnyAttribute {
    fn clone(&self) -> Self {
        (self.clone)(&self.value)
    }
}

impl Debug for AnyAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyAttribute").finish_non_exhaustive()
    }
}

/// View state for [`AnyAttribute`].
pub struct AnyAttributeState {
    type_id: TypeId,
    state: ErasedLocal,
    el: crate::renderer::types::Element,
}

/// Converts an [`Attribute`] into [`AnyAttribute`].
pub trait IntoAnyAttribute {
    /// Wraps the given attribute.
    fn into_any_attr(self) -> AnyAttribute;
}

impl<T> IntoAnyAttribute for T
where
    Self: Send,
    T: Attribute,
    crate::renderer::types::Element: Clone,
{
    fn into_any_attr(self) -> AnyAttribute {
        fn clone<T: Attribute + Clone + 'static>(
            value: &Erased,
        ) -> AnyAttribute {
            value.get_ref::<T>().clone().into_any_attr()
        }

        #[cfg(feature = "ssr")]
        fn to_html<T: Attribute + 'static>(
            value: Erased,
            buf: &mut String,
            class: &mut String,
            style: &mut String,
            inner_html: &mut String,
        ) {
            value
                .into_inner::<T>()
                .to_html(buf, class, style, inner_html);
        }

        fn build<T: Attribute + 'static>(
            value: Erased,
            el: crate::renderer::types::Element,
        ) -> AnyAttributeState {
            AnyAttributeState {
                type_id: TypeId::of::<T>(),
                state: ErasedLocal::new(value.into_inner::<T>().build(&el)),
                el,
            }
        }

        #[cfg(feature = "hydrate")]
        fn hydrate_from_server<T: Attribute + 'static>(
            value: Erased,
            el: crate::renderer::types::Element,
        ) -> AnyAttributeState {
            AnyAttributeState {
                type_id: TypeId::of::<T>(),
                state: ErasedLocal::new(
                    value.into_inner::<T>().hydrate::<true>(&el),
                ),
                el,
            }
        }

        #[cfg(feature = "hydrate")]
        fn hydrate_from_template<T: Attribute + 'static>(
            value: Erased,
            el: crate::renderer::types::Element,
        ) -> AnyAttributeState {
            AnyAttributeState {
                type_id: TypeId::of::<T>(),
                state: ErasedLocal::new(
                    value.into_inner::<T>().hydrate::<true>(&el),
                ),
                el,
            }
        }

        fn rebuild<T: Attribute + 'static>(
            value: Erased,
            state: &mut AnyAttributeState,
        ) {
            let value = value.into_inner::<T>();
            let state = state.state.get_mut::<T::State>();
            value.rebuild(state);
        }

        #[cfg(feature = "ssr")]
        fn dry_resolve<T: Attribute + 'static>(value: &mut Erased) {
            value.get_mut::<T>().dry_resolve();
        }

        #[cfg(feature = "ssr")]
        fn resolve<T: Attribute + 'static>(
            value: Erased,
        ) -> Pin<Box<dyn Future<Output = AnyAttribute> + Send>> {
            use futures::FutureExt;

            async move {value.into_inner::<T>().resolve().await.into_any_attr()}.boxed()
        }

        let value = self.into_cloneable_owned();
        AnyAttribute {
            type_id: TypeId::of::<T::CloneableOwned>(),
            html_len: value.html_len(),
            value: Erased::new(value),
            clone: clone::<T::CloneableOwned>,
            #[cfg(feature = "ssr")]
            to_html: to_html::<T::CloneableOwned>,
            build: build::<T::CloneableOwned>,
            rebuild: rebuild::<T::CloneableOwned>,
            #[cfg(feature = "hydrate")]
            hydrate_from_server: hydrate_from_server::<T::CloneableOwned>,
            #[cfg(feature = "hydrate")]
            hydrate_from_template: hydrate_from_template::<T::CloneableOwned>,
            #[cfg(feature = "ssr")]
            resolve: resolve::<T::CloneableOwned>,
            #[cfg(feature = "ssr")]
            dry_resolve: dry_resolve::<T::CloneableOwned>,
        }
    }
}

impl NextAttribute for AnyAttribute {
    type Output<NewAttr: Attribute> = Vec<AnyAttribute>;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        vec![self, new_attr.into_any_attr()]
    }
}

impl Attribute for AnyAttribute {
    const MIN_LENGTH: usize = 0;

    type AsyncOutput = AnyAttribute;
    type State = AnyAttributeState;
    type Cloneable = AnyAttribute;
    type CloneableOwned = AnyAttribute;

    fn html_len(&self) -> usize {
        self.html_len
    }

    #[allow(unused)] // they are used in SSR
    fn to_html(
        self,
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
    ) {
        #[cfg(feature = "ssr")]
        {
            (self.to_html)(self.value, buf, class, style, inner_html);
        }
        #[cfg(not(feature = "ssr"))]
        panic!(
            "You are rendering AnyAttribute to HTML without the `ssr` feature \
             enabled."
        );
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        #[cfg(feature = "hydrate")]
        if FROM_SERVER {
            (self.hydrate_from_server)(self.value, el.clone())
        } else {
            (self.hydrate_from_template)(self.value, el.clone())
        }
        #[cfg(not(feature = "hydrate"))]
        {
            _ = el;
            panic!(
                "You are trying to hydrate AnyAttribute without the `hydrate` \
                 feature enabled."
            );
        }
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        (self.build)(self.value, el.clone())
    }

    fn rebuild(self, state: &mut Self::State) {
        if self.type_id == state.type_id {
            (self.rebuild)(self.value, state)
        } else {
            let new = self.build(&state.el);
            *state = new;
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }

    fn dry_resolve(&mut self) {
        #[cfg(feature = "ssr")]
        {
            (self.dry_resolve)(&mut self.value)
        }
        #[cfg(not(feature = "ssr"))]
        panic!(
            "You are rendering AnyAttribute to HTML without the `ssr` feature \
             enabled."
        );
    }

    async fn resolve(self) -> Self::AsyncOutput {
        #[cfg(feature = "ssr")]
        {
            (self.resolve)(self.value).await
        }
        #[cfg(not(feature = "ssr"))]
        panic!(
            "You are rendering AnyAttribute to HTML without the `ssr` feature \
             enabled."
        );
    }
}

impl NextAttribute for Vec<AnyAttribute> {
    type Output<NewAttr: Attribute> = Self;

    fn add_any_attr<NewAttr: Attribute>(
        mut self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        self.push(new_attr.into_any_attr());
        self
    }
}

impl Attribute for Vec<AnyAttribute> {
    const MIN_LENGTH: usize = 0;

    type AsyncOutput = Vec<AnyAttribute>;
    type State = Vec<AnyAttributeState>;
    type Cloneable = Vec<AnyAttribute>;
    type CloneableOwned = Vec<AnyAttribute>;

    fn html_len(&self) -> usize {
        self.iter().map(|attr| attr.html_len()).sum()
    }

    #[allow(unused)] // they are used in SSR
    fn to_html(
        self,
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
    ) {
        #[cfg(feature = "ssr")]
        {
            for mut attr in self {
                attr.to_html(buf, class, style, inner_html)
            }
        }
        #[cfg(not(feature = "ssr"))]
        panic!(
            "You are rendering AnyAttribute to HTML without the `ssr` feature \
             enabled."
        );
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        #[cfg(feature = "hydrate")]
        if FROM_SERVER {
            self.into_iter()
                .map(|attr| attr.hydrate::<true>(el))
                .collect()
        } else {
            self.into_iter()
                .map(|attr| attr.hydrate::<false>(el))
                .collect()
        }
        #[cfg(not(feature = "hydrate"))]
        {
            _ = el;
            panic!(
                "You are trying to hydrate AnyAttribute without the `hydrate` \
                 feature enabled."
            );
        }
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        self.into_iter().map(|attr| attr.build(el)).collect()
    }

    fn rebuild(self, state: &mut Self::State) {
        for (attr, state) in self.into_iter().zip(state.iter_mut()) {
            attr.rebuild(state)
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }

    fn dry_resolve(&mut self) {
        #[cfg(feature = "ssr")]
        {
            for attr in self.iter_mut() {
                attr.dry_resolve()
            }
        }
        #[cfg(not(feature = "ssr"))]
        panic!(
            "You are rendering AnyAttribute to HTML without the `ssr` feature \
             enabled."
        );
    }

    async fn resolve(self) -> Self::AsyncOutput {
        #[cfg(feature = "ssr")]
        {
            futures::future::join_all(
                self.into_iter().map(|attr| attr.resolve()),
            )
            .await
        }
        #[cfg(not(feature = "ssr"))]
        panic!(
            "You are rendering AnyAttribute to HTML without the `ssr` feature \
             enabled."
        );
    }
}
