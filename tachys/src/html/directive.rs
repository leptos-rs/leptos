use super::attribute::{Attribute, NextAttribute};
use crate::{
    prelude::AddAnyAttr,
    view::{Position, ToTemplate},
};
use send_wrapper::SendWrapper;
use std::{marker::PhantomData, sync::Arc};

/// Adds a directive to the element, which runs some custom logic in the browser when the element
/// is created or hydrated.
pub trait DirectiveAttribute<T, P, D>
where
    D: IntoDirective<T, P>,
{
    /// The type of the element with the directive added.
    type Output;

    /// Adds a directive to the element, which runs some custom logic in the browser when the element
    /// is created or hydrated.
    fn directive(self, handler: D, param: P) -> Self::Output;
}

impl<V, T, P, D> DirectiveAttribute<T, P, D> for V
where
    V: AddAnyAttr,
    D: IntoDirective<T, P>,
    P: Clone + 'static,
    T: 'static,
{
    type Output = <Self as AddAnyAttr>::Output<Directive<T, D, P>>;

    fn directive(self, handler: D, param: P) -> Self::Output {
        self.add_any_attr(directive(handler, param))
    }
}

/// Adds a directive to the element, which runs some custom logic in the browser when the element
/// is created or hydrated.
#[inline(always)]
pub fn directive<T, P, D>(handler: D, param: P) -> Directive<T, D, P>
where
    D: IntoDirective<T, P>,
{
    Directive(Some(SendWrapper::new(DirectiveInner {
        handler,
        param,
        t: PhantomData,
    })))
}

/// Custom logic that runs in the browser when the element is created or hydrated.
#[derive(Debug)]
pub struct Directive<T, D, P>(Option<SendWrapper<DirectiveInner<T, D, P>>>);

impl<T, D, P> Clone for Directive<T, D, P>
where
    P: Clone + 'static,
    D: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Debug)]
struct DirectiveInner<T, D, P> {
    handler: D,
    param: P,
    t: PhantomData<T>,
}

impl<T, D, P> Clone for DirectiveInner<T, D, P>
where
    P: Clone + 'static,
    D: Clone,
{
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            param: self.param.clone(),
            t: PhantomData,
        }
    }
}

impl<T, P, D> Attribute for Directive<T, D, P>
where
    D: IntoDirective<T, P>,
    P: Clone + 'static, // TODO this is just here to make them cloneable
    T: 'static,
{
    const MIN_LENGTH: usize = 0;

    type AsyncOutput = Self;
    type State = crate::renderer::types::Element;
    type Cloneable = Directive<T, D::Cloneable, P>;
    type CloneableOwned = Directive<T, D::Cloneable, P>;

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

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let inner = self.0.expect("directive removed early").take();
        inner.handler.run(el.clone(), inner.param);
        el.clone()
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let inner = self.0.expect("directive removed early").take();
        inner.handler.run(el.clone(), inner.param);
        el.clone()
    }

    fn rebuild(self, state: &mut Self::State) {
        let inner = self.0.expect("directive removed early").take();
        inner.handler.run(state.clone(), inner.param);
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into_cloneable_owned()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        let inner = self.0.map(|inner| {
            let DirectiveInner { handler, param, t } = inner.take();
            SendWrapper::new(DirectiveInner {
                handler: handler.into_cloneable(),
                param,
                t,
            })
        });
        Directive(inner)
    }

    fn dry_resolve(&mut self) {
        // dry_resolve() only runs during SSR, and we should use it to
        // synchronously remove and drop the SendWrapper value
        // we don't need this value during SSR and leaving it here could drop it
        // from a different thread
        self.0.take();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl<T, D, P> NextAttribute for Directive<T, D, P>
where
    D: IntoDirective<T, P>,
    P: Clone + 'static,
    T: 'static,
{
    type Output<NewAttr: Attribute> = (Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        (self, new_attr)
    }
}

impl<T, D, P> ToTemplate for Directive<T, D, P> {
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
/// ```ignore
/// # use leptos::{*, html::AnyElement};
///
/// // This doesn't take an attribute value
/// fn my_directive(el: crate::renderer::types::Element) {
///     // do sth
/// }
///
/// // This requires an attribute value
/// fn another_directive(el: crate::renderer::types::Element, params: i32) {
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
pub trait IntoDirective<T: ?Sized, P> {
    /// An equivalent to this directive that is cloneable and owned.
    type Cloneable: IntoDirective<T, P> + Clone + 'static;

    /// Calls the handler function
    fn run(&self, el: crate::renderer::types::Element, param: P);

    /// Converts this into a cloneable type.
    fn into_cloneable(self) -> Self::Cloneable;
}

impl<F> IntoDirective<(crate::renderer::types::Element,), ()> for F
where
    F: Fn(crate::renderer::types::Element) + 'static,
{
    type Cloneable = Arc<dyn Fn(crate::renderer::types::Element)>;

    fn run(&self, el: crate::renderer::types::Element, _: ()) {
        self(el)
    }

    fn into_cloneable(self) -> Self::Cloneable {
        Arc::new(self)
    }
}

impl IntoDirective<(crate::renderer::types::Element,), ()>
    for Arc<dyn Fn(crate::renderer::types::Element)>
{
    type Cloneable = Arc<dyn Fn(crate::renderer::types::Element)>;

    fn run(&self, el: crate::renderer::types::Element, _: ()) {
        self(el)
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }
}

impl<F, P> IntoDirective<(crate::renderer::types::Element, P), P> for F
where
    F: Fn(crate::renderer::types::Element, P) + 'static,
    P: 'static,
{
    type Cloneable = Arc<dyn Fn(crate::renderer::types::Element, P)>;

    fn run(&self, el: crate::renderer::types::Element, param: P) {
        self(el, param);
    }

    fn into_cloneable(self) -> Self::Cloneable {
        Arc::new(self)
    }
}

impl<P> IntoDirective<(crate::renderer::types::Element, P), P>
    for Arc<dyn Fn(crate::renderer::types::Element, P)>
where
    P: 'static,
{
    type Cloneable = Arc<dyn Fn(crate::renderer::types::Element, P)>;

    fn run(&self, el: crate::renderer::types::Element, param: P) {
        self(el, param)
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }
}
