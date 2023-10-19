use crate::{html::AnyElement, HtmlElement};
use std::rc::Rc;

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
/// fn my_directive(el: HtmlElement<AnyElement>) {
///     // do sth
/// }
///
/// // This requires an attribute value
/// fn another_directive(el: HtmlElement<AnyElement>, params: i32) {
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
pub trait Directive<T: ?Sized, P> {
    /// Calls the handler function
    fn run(&self, el: HtmlElement<AnyElement>, param: P);
}

impl<F> Directive<(HtmlElement<AnyElement>,), ()> for F
where
    F: Fn(HtmlElement<AnyElement>),
{
    fn run(&self, el: HtmlElement<AnyElement>, _: ()) {
        self(el)
    }
}

impl<F, P> Directive<(HtmlElement<AnyElement>, P), P> for F
where
    F: Fn(HtmlElement<AnyElement>, P),
{
    fn run(&self, el: HtmlElement<AnyElement>, param: P) {
        self(el, param);
    }
}

impl<T: ?Sized, P> Directive<T, P> for Rc<dyn Directive<T, P>> {
    fn run(&self, el: HtmlElement<AnyElement>, param: P) {
        (**self).run(el, param)
    }
}

impl<T: ?Sized, P> Directive<T, P> for Box<dyn Directive<T, P>> {
    fn run(&self, el: HtmlElement<AnyElement>, param: P) {
        (**self).run(el, param);
    }
}
