use super::attribute::{Attribute, NextAttribute};
use crate::{
    prelude::AddAnyAttr,
    renderer::Renderer,
    view::{Position, ToTemplate},
};
use send_wrapper::SendWrapper;
use std::{marker::PhantomData, sync::Arc};

pub trait DirectiveAttribute<T, P, D, Rndr>
where
    D: IntoDirective<T, P, Rndr>,
    Rndr: Renderer,
{
    type Output;

    fn directive(self, handler: D, param: P) -> Self::Output;
}

impl<V, T, P, D, Rndr> DirectiveAttribute<T, P, D, Rndr> for V
where
    V: AddAnyAttr<Rndr>,
    D: IntoDirective<T, P, Rndr>,
    P: Clone + 'static,
    T: 'static,
    Rndr: Renderer,
{
    type Output = <Self as AddAnyAttr<Rndr>>::Output<Directive<T, D, P, Rndr>>;

    fn directive(self, handler: D, param: P) -> Self::Output {
        self.add_any_attr(directive(handler, param))
    }
}

#[inline(always)]
pub fn directive<T, P, D, R>(handler: D, param: P) -> Directive<T, D, P, R>
where
    D: IntoDirective<T, P, R>,
    R: Renderer,
{
    Directive(SendWrapper::new(DirectiveInner {
        handler,
        param,
        t: PhantomData,
        rndr: PhantomData,
    }))
}

#[derive(Debug)]
pub struct Directive<T, D, P, R>(SendWrapper<DirectiveInner<T, D, P, R>>);

impl<T, D, P, R> Clone for Directive<T, D, P, R>
where
    P: Clone + 'static,
    D: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Debug)]
pub struct DirectiveInner<T, D, P, R> {
    handler: D,
    param: P,
    t: PhantomData<T>,
    rndr: PhantomData<R>,
}

impl<T, D, P, R> Clone for DirectiveInner<T, D, P, R>
where
    P: Clone + 'static,
    D: Clone,
{
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            param: self.param.clone(),
            t: PhantomData,
            rndr: PhantomData,
        }
    }
}

impl<T, P, D, R> Attribute<R> for Directive<T, D, P, R>
where
    D: IntoDirective<T, P, R>,
    P: Clone + 'static, // TODO this is just here to make them cloneable
    T: 'static,
    R: Renderer,
{
    const MIN_LENGTH: usize = 0;

    type AsyncOutput = Self;
    type State = R::Element;
    type Cloneable = Directive<T, D::Cloneable, P, R>;
    type CloneableOwned = Directive<T, D::Cloneable, P, R>;

    fn html_len(&self) -> usize {
        0
    }

    fn to_html(
        self,
        _buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
    ) {
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        let inner = self.0.take();
        inner.handler.run(el.clone(), inner.param);
        el.clone()
    }

    fn build(self, el: &R::Element) -> Self::State {
        let inner = self.0.take();
        inner.handler.run(el.clone(), inner.param);
        el.clone()
    }

    fn rebuild(self, state: &mut Self::State) {
        let inner = self.0.take();
        inner.handler.run(state.clone(), inner.param);
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into_cloneable_owned()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        let DirectiveInner {
            handler,
            param,
            t,
            rndr,
        } = self.0.take();
        Directive(SendWrapper::new(DirectiveInner {
            handler: handler.into_cloneable(),
            param,
            t,
            rndr,
        }))
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl<T, D, P, R> NextAttribute<R> for Directive<T, D, P, R>
where
    D: IntoDirective<T, P, R>,
    P: Clone + 'static,
    T: 'static,
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

impl<T, D, P, R> ToTemplate for Directive<T, D, P, R> {
    const CLASS: &'static str = "";

    fn to_template(
        _buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
        _position: &mut Position,
    ) {
    }
}

/// Trait for a directive handler function.
/// This is used so it's possible to use functions with one or two
/// parameters as directive handlers.
///
/// You can use directives like the following.
///
/// ```
/// # use leptos::{*, html::AnyElement};
///
/// // This doesn't take an attribute value
/// fn my_directive(el: R::Element) {
///     // do sth
/// }
///
/// // This requires an attribute value
/// fn another_directive(el: R::Element, params: i32) {
///     // do sth
/// }
///
/// #[component]
/// pub fn MyComponent() -> impl IntoView {
///     view! {
///         // no attribute value
///         <div use:my_directive></div>
///
///         // with an attribute value
///         <div use:another_directive=8></div>
///     }
/// }
/// ```
///
/// A directive is just syntactic sugar for
///
/// ```ignore
/// let node_ref = create_node_ref();
///
/// create_effect(move |_| {
///     if let Some(el) = node_ref.get() {
///         directive_func(el, possibly_some_param);
///     }
/// });
/// ```
///
/// A directive can be a function with one or two parameters.
/// The first is the element the directive is added to and the optional
/// second is the parameter that is provided in the attribute.
pub trait IntoDirective<T: ?Sized, P, R: Renderer> {
    type Cloneable: IntoDirective<T, P, R> + Clone + 'static;

    /// Calls the handler function
    fn run(&self, el: R::Element, param: P);

    fn into_cloneable(self) -> Self::Cloneable;
}

impl<F, R> IntoDirective<(R::Element,), (), R> for F
where
    F: Fn(R::Element) + 'static,
    R: Renderer,
{
    type Cloneable = Arc<dyn Fn(R::Element)>;

    fn run(&self, el: R::Element, _: ()) {
        self(el)
    }

    fn into_cloneable(self) -> Self::Cloneable {
        Arc::new(self)
    }
}

impl<R> IntoDirective<(R::Element,), (), R> for Arc<dyn Fn(R::Element)>
where
    R: Renderer,
{
    type Cloneable = Arc<dyn Fn(R::Element)>;

    fn run(&self, el: R::Element, _: ()) {
        self(el)
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }
}

impl<F, P, R> IntoDirective<(R::Element, P), P, R> for F
where
    F: Fn(R::Element, P) + 'static,
    P: 'static,
    R: Renderer,
{
    type Cloneable = Arc<dyn Fn(R::Element, P)>;

    fn run(&self, el: R::Element, param: P) {
        self(el, param);
    }

    fn into_cloneable(self) -> Self::Cloneable {
        Arc::new(self)
    }
}

impl<P, R> IntoDirective<(R::Element, P), P, R> for Arc<dyn Fn(R::Element, P)>
where
    R: Renderer,
    P: 'static,
{
    type Cloneable = Arc<dyn Fn(R::Element, P)>;

    fn run(&self, el: R::Element, param: P) {
        self(el, param)
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }
}
