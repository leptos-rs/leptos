use crate::attr::{
    any_attribute::{AnyAttribute, IntoAnyAttribute},
    Attribute, NextAttribute,
};
use leptos::prelude::*;

/// Function stored to build/rebuild the wrapped children when attributes are added.
pub type AiChildBuilder<T> = dyn Fn(AnyAttribute) -> T + Send + Sync + 'static;

/// Wrapper to intercept attributes passed to a component so you can apply them to a different
/// element.
pub struct AttrInterceptor<T: IntoView, A> {
    children_builder: Box<AiChildBuilder<T>>,
    children: T,
    attributes: A,
}

impl<T: IntoView> AttrInterceptor<T, ()> {
    /// Use this as the returned view from your component to collect the attributes that are passed
    /// to your component so you can manually handle them.
    pub fn new<F>(children: F) -> Self
    where
        F: Fn(AnyAttribute) -> T + Send + Sync + 'static,
    {
        let children_builder = Box::new(children);
        let children = children_builder(().into_any_attr());

        Self {
            children_builder,
            children,
            attributes: (),
        }
    }
}

impl<T: IntoView, A: Attribute> Render for AttrInterceptor<T, A> {
    type State = <T as Render>::State;

    fn build(self) -> Self::State {
        self.children.build()
    }

    fn rebuild(self, state: &mut Self::State) {
        self.children.rebuild(state);
    }
}

impl<T: IntoView, A> AddAnyAttr for AttrInterceptor<T, A>
where
    A: Attribute,
{
    type Output<SomeNewAttr: leptos::attr::Attribute> =
        AttrInterceptor<T, <A as NextAttribute>::Output<SomeNewAttr>>;

    fn add_any_attr<NewAttr: leptos::attr::Attribute>(self, attr: NewAttr) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        let attributes = self.attributes.add_any_attr(attr);

        // I think I want to do something like this but we can't move attributes into the
        // children_builder and still store it here to keep track of new attributes.
        // let children = (self.children_builder)(attributes.into_any_attr());
        
        AttrInterceptor {
            children_builder: self.children_builder,
            children: self.children,
            attributes,
        }
    }
}

impl<T: IntoView, A: Attribute> RenderHtml for AttrInterceptor<T, A> {
    type AsyncOutput = T::AsyncOutput;

    const MIN_LENGTH: usize = T::MIN_LENGTH;

    fn dry_resolve(&mut self) {
        self.children.dry_resolve()
    }

    fn resolve(self) -> impl std::future::Future<Output = Self::AsyncOutput> + Send {
        self.children.resolve()
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut leptos::tachys::view::Position,
        escape: bool,
        mark_branches: bool,
    ) {
        self.children
            .to_html_with_buf(buf, position, escape, mark_branches)
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &leptos::tachys::hydration::Cursor,
        position: &leptos::tachys::view::PositionState,
    ) -> Self::State {
        self.children.hydrate::<FROM_SERVER>(cursor, position)
    }
}

