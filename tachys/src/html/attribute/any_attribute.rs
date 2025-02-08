use super::{Attribute, NextAttribute};
use dyn_clone::DynClone;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
};
#[cfg(feature = "ssr")]
use std::{future::Future, pin::Pin};

trait DynAttr: DynClone + Any + Send + 'static {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    #[cfg(feature = "ssr")]
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

dyn_clone::clone_trait_object!(DynAttr);

impl<T: Clone> DynAttr for T
where
    T: Attribute + 'static,
{
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    #[cfg(feature = "ssr")]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A type-erased container for any [`Attribute`].
#[derive(Clone)]
pub struct AnyAttribute {
    type_id: TypeId,
    html_len: usize,
    value: Box<dyn DynAttr>,
    #[cfg(feature = "ssr")]
    to_html: fn(
        Box<dyn DynAttr>,
        &mut String,
        &mut String,
        &mut String,
        &mut String,
    ),
    build: fn(
        Box<dyn DynAttr>,
        el: &crate::renderer::types::Element,
    ) -> AnyAttributeState,
    rebuild: fn(TypeId, Box<dyn DynAttr>, &mut AnyAttributeState),
    #[cfg(feature = "hydrate")]
    hydrate_from_server: fn(
        Box<dyn DynAttr>,
        &crate::renderer::types::Element,
    ) -> AnyAttributeState,
    #[cfg(feature = "hydrate")]
    hydrate_from_template: fn(
        Box<dyn DynAttr>,
        &crate::renderer::types::Element,
    ) -> AnyAttributeState,
    #[cfg(feature = "ssr")]
    #[allow(clippy::type_complexity)]
    resolve: fn(
        Box<dyn DynAttr>,
    ) -> Pin<Box<dyn Future<Output = AnyAttribute> + Send>>,
    #[cfg(feature = "ssr")]
    dry_resolve: fn(&mut Box<dyn DynAttr>),
}

impl Debug for AnyAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyAttribute").finish_non_exhaustive()
    }
}

/// View state for [`AnyAttribute`].
pub struct AnyAttributeState {
    type_id: TypeId,
    state: Box<dyn Any>,
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
        let value =
            Box::new(self.into_cloneable_owned()) as Box<dyn Any + Send>;
        let value = match (value as Box<dyn Any>).downcast::<AnyAttribute>() {
            // if it's already an AnyAttribute, we don't need to double-wrap it
            Ok(any_attribute) => return *any_attribute,
            Err(value) => value.downcast::<T::CloneableOwned>().unwrap(),
        };

        #[cfg(feature = "ssr")]
        let to_html = |value: Box<dyn DynAttr>,
                       buf: &mut String,
                       class: &mut String,
                       style: &mut String,
                       inner_html: &mut String| {
            let value = value
                .into_any()
                .downcast::<T::CloneableOwned>()
                .expect("AnyAttribute::to_html could not be downcast");
            value.to_html(buf, class, style, inner_html);
        };
        let build = |value: Box<dyn DynAttr>,
                     el: &crate::renderer::types::Element| {
            let value = value
                .into_any()
                .downcast::<T::CloneableOwned>()
                .expect("AnyAttribute::build couldn't downcast");
            let state = Box::new(value.build(el));

            AnyAttributeState {
                type_id: TypeId::of::<T::CloneableOwned>(),
                state,
                el: el.clone(),
            }
        };
        #[cfg(feature = "hydrate")]
        let hydrate_from_server =
            |value: Box<dyn DynAttr>, el: &crate::renderer::types::Element| {
                let value =
                    value.into_any().downcast::<T::CloneableOwned>().expect(
                        "AnyAttribute::hydrate_from_server couldn't downcast",
                    );
                let state = Box::new(value.hydrate::<true>(el));

                AnyAttributeState {
                    type_id: TypeId::of::<T::CloneableOwned>(),
                    state,
                    el: el.clone(),
                }
            };
        #[cfg(feature = "hydrate")]
        let hydrate_from_template =
            |value: Box<dyn DynAttr>, el: &crate::renderer::types::Element| {
                let value =
                    value.into_any().downcast::<T::CloneableOwned>().expect(
                        "AnyAttribute::hydrate_from_server couldn't downcast",
                    );
                let state = Box::new(value.hydrate::<true>(el));

                AnyAttributeState {
                    type_id: TypeId::of::<T::CloneableOwned>(),
                    state,
                    el: el.clone(),
                }
            };
        let rebuild = |new_type_id: TypeId,
                       value: Box<dyn DynAttr>,
                       state: &mut AnyAttributeState| {
            let value = value
                .into_any()
                .downcast::<T::CloneableOwned>()
                .expect("AnyAttribute::rebuild couldn't downcast value");
            if new_type_id == state.type_id {
                let state = state
                    .state
                    .downcast_mut()
                    .expect("AnyAttribute::rebuild couldn't downcast state");
                value.rebuild(state);
            } else {
                let new = value.into_any_attr().build(&state.el);
                *state = new;
            }
        };
        #[cfg(feature = "ssr")]
        let dry_resolve = |value: &mut Box<dyn DynAttr>| {
            let value = value
                .as_any_mut()
                .downcast_mut::<T::CloneableOwned>()
                .expect("AnyView::resolve could not be downcast");
            value.dry_resolve();
        };

        #[cfg(feature = "ssr")]
        let resolve = |value: Box<dyn DynAttr>| {
            let value = value
                .into_any()
                .downcast::<T::CloneableOwned>()
                .expect("AnyView::resolve could not be downcast");
            Box::pin(async move { value.resolve().await.into_any_attr() })
                as Pin<Box<dyn Future<Output = AnyAttribute> + Send>>
        };
        AnyAttribute {
            type_id: TypeId::of::<T::CloneableOwned>(),
            html_len: value.html_len(),
            value,
            #[cfg(feature = "ssr")]
            to_html,
            build,
            rebuild,
            #[cfg(feature = "hydrate")]
            hydrate_from_server,
            #[cfg(feature = "hydrate")]
            hydrate_from_template,
            #[cfg(feature = "ssr")]
            resolve,
            #[cfg(feature = "ssr")]
            dry_resolve,
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
            (self.hydrate_from_server)(self.value, el)
        } else {
            (self.hydrate_from_template)(self.value, el)
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
        (self.build)(self.value, el)
    }

    fn rebuild(self, state: &mut Self::State) {
        (self.rebuild)(self.type_id, self.value, state)
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
