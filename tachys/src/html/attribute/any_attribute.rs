use super::{Attribute, NextAttribute};
use crate::renderer::Renderer;
use std::{
    any::{Any, TypeId},
    marker::PhantomData,
};

pub struct AnyAttribute<R: Renderer> {
    type_id: TypeId,
    value: Box<dyn Any + Send + Sync>,
    to_html:
        fn(Box<dyn Any>, &mut String, &mut String, &mut String, &mut String),
    build: fn(Box<dyn Any>, el: &R::Element) -> AnyAttributeState<R>,
    rebuild: fn(TypeId, Box<dyn Any>, &mut AnyAttributeState<R>),
    hydrate_from_server: fn(Box<dyn Any>, &R::Element) -> AnyAttributeState<R>,
    hydrate_from_template:
        fn(Box<dyn Any>, &R::Element) -> AnyAttributeState<R>,
}

pub struct AnyAttributeState<R>
where
    R: Renderer,
{
    type_id: TypeId,
    state: Box<dyn Any>,
    el: R::Element,
    rndr: PhantomData<R>,
}

pub trait IntoAnyAttribute<R>
where
    R: Renderer,
{
    fn into_any_attr(self) -> AnyAttribute<R>;
}

impl<T, R> IntoAnyAttribute<R> for T
where
    Self: Send + Sync,
    T: Attribute<R> + 'static,
    T::State: 'static,
    R: Renderer + 'static,
    R::Element: Clone,
{
    // inlining allows the compiler to remove the unused functions
    // i.e., doesn't ship HTML-generating code that isn't used
    #[inline(always)]
    fn into_any_attr(self) -> AnyAttribute<R> {
        let value = Box::new(self) as Box<dyn Any + Send + Sync>;

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
        AnyAttribute {
            type_id: TypeId::of::<T>(),
            value,
            to_html,
            build,
            rebuild,
            hydrate_from_server,
            hydrate_from_template,
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

    type State = AnyAttributeState<R>;

    fn to_html(
        self,
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
    ) {
        (self.to_html)(self.value, buf, class, style, inner_html);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &<R as Renderer>::Element,
    ) -> Self::State {
        if FROM_SERVER {
            (self.hydrate_from_server)(self.value, el)
        } else {
            (self.hydrate_from_template)(self.value, el)
        }
    }

    fn build(self, el: &<R as Renderer>::Element) -> Self::State {
        (self.build)(self.value, el)
    }

    fn rebuild(self, state: &mut Self::State) {
        (self.rebuild)(self.type_id, self.value, state)
    }
}
