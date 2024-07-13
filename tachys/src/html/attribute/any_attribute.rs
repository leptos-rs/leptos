use super::{Attribute, NextAttribute};
use crate::renderer::Renderer;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    marker::PhantomData,
};
#[cfg(feature = "ssr")]
use std::{future::Future, pin::Pin};

/// A type-erased container for any [`Attribute`].
pub struct AnyAttribute<R: Renderer> {
    type_id: TypeId,
    html_len: usize,
    value: Box<dyn Any + Send>,
    #[cfg(feature = "ssr")]
    to_html:
        fn(Box<dyn Any>, &mut String, &mut String, &mut String, &mut String),
    build: fn(Box<dyn Any>, el: &R::Element) -> AnyAttributeState<R>,
    rebuild: fn(TypeId, Box<dyn Any>, &mut AnyAttributeState<R>),
    #[cfg(feature = "hydrate")]
    hydrate_from_server: fn(Box<dyn Any>, &R::Element) -> AnyAttributeState<R>,
    #[cfg(feature = "hydrate")]
    hydrate_from_template:
        fn(Box<dyn Any>, &R::Element) -> AnyAttributeState<R>,
    #[cfg(feature = "ssr")]
    #[allow(clippy::type_complexity)]
    resolve: fn(
        Box<dyn Any>,
    ) -> Pin<Box<dyn Future<Output = AnyAttribute<R>> + Send>>,
    #[cfg(feature = "ssr")]
    dry_resolve: fn(&mut Box<dyn Any + Send>),
}

impl<R> Debug for AnyAttribute<R>
where
    R: Renderer,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyAttribute").finish_non_exhaustive()
    }
}

/// View state for [`AnyAttribute`].
pub struct AnyAttributeState<R>
where
    R: Renderer,
{
    type_id: TypeId,
    state: Box<dyn Any>,
    el: R::Element,
    rndr: PhantomData<R>,
}

/// Converts an [`Attribute`] into [`AnyAttribute`].
pub trait IntoAnyAttribute<R>
where
    R: Renderer,
{
    /// Wraps the given attribute.
    fn into_any_attr(self) -> AnyAttribute<R>;
}

impl<T, R> IntoAnyAttribute<R> for T
where
    Self: Send,
    T: Attribute<R> + 'static,
    T::State: 'static,
    R: Renderer + 'static,
    R::Element: Clone,
{
    // inlining allows the compiler to remove the unused functions
    // i.e., doesn't ship HTML-generating code that isn't used
    #[inline(always)]
    fn into_any_attr(self) -> AnyAttribute<R> {
        let html_len = self.html_len();

        let value = Box::new(self) as Box<dyn Any + Send>;

        #[cfg(feature = "ssr")]
        let to_html = |value: Box<dyn Any>,
                       buf: &mut String,
                       class: &mut String,
                       style: &mut String,
                       inner_html: &mut String| {
            let value = value
                .downcast::<T>()
                .expect("AnyAttribute::to_html could not be downcast");
            value.to_html(buf, class, style, inner_html);
        };
        let build = |value: Box<dyn Any>, el: &R::Element| {
            let value = value
                .downcast::<T>()
                .expect("AnyAttribute::build couldn't downcast");
            let state = Box::new(value.build(el));

            AnyAttributeState {
                type_id: TypeId::of::<T>(),
                state,
                el: el.clone(),
                rndr: PhantomData,
            }
        };
        #[cfg(feature = "hydrate")]
        let hydrate_from_server = |value: Box<dyn Any>, el: &R::Element| {
            let value = value
                .downcast::<T>()
                .expect("AnyAttribute::hydrate_from_server couldn't downcast");
            let state = Box::new(value.hydrate::<true>(el));

            AnyAttributeState {
                type_id: TypeId::of::<T>(),
                state,
                el: el.clone(),
                rndr: PhantomData,
            }
        };
        #[cfg(feature = "hydrate")]
        let hydrate_from_template = |value: Box<dyn Any>, el: &R::Element| {
            let value = value
                .downcast::<T>()
                .expect("AnyAttribute::hydrate_from_server couldn't downcast");
            let state = Box::new(value.hydrate::<true>(el));

            AnyAttributeState {
                type_id: TypeId::of::<T>(),
                state,
                el: el.clone(),
                rndr: PhantomData,
            }
        };
        let rebuild = |new_type_id: TypeId,
                       value: Box<dyn Any>,
                       state: &mut AnyAttributeState<R>| {
            let value = value
                .downcast::<T>()
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
        let dry_resolve = |value: &mut Box<dyn Any + Send>| {
            let value = value
                .downcast_mut::<T>()
                .expect("AnyView::resolve could not be downcast");
            value.dry_resolve();
        };

        #[cfg(feature = "ssr")]
        let resolve = |value: Box<dyn Any>| {
            let value = value
                .downcast::<T>()
                .expect("AnyView::resolve could not be downcast");
            Box::pin(async move { value.resolve().await.into_any_attr() })
                as Pin<Box<dyn Future<Output = AnyAttribute<R>> + Send>>
        };
        AnyAttribute {
            type_id: TypeId::of::<T>(),
            html_len,
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

impl<R> NextAttribute<R> for AnyAttribute<R>
where
    R: Renderer,
{
    type Output<NewAttr: Attribute<R>> = (Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute<R>>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        (self, new_attr)
    }
}

impl<R> Attribute<R> for AnyAttribute<R>
where
    R: Renderer,
{
    const MIN_LENGTH: usize = 0;

    type AsyncOutput = AnyAttribute<R>;
    type State = AnyAttributeState<R>;
    type Cloneable = ();
    type CloneableOwned = ();

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
        el: &<R as Renderer>::Element,
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

    fn build(self, el: &<R as Renderer>::Element) -> Self::State {
        (self.build)(self.value, el)
    }

    fn rebuild(self, state: &mut Self::State) {
        (self.rebuild)(self.type_id, self.value, state)
    }

    fn into_cloneable(self) -> Self::Cloneable {
        todo!()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        todo!()
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
