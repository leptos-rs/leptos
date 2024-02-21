use super::{ElementWithChildren, HtmlElement};
use crate::{
    html::{attribute::Attribute, element::AddAttribute},
    prelude::Render,
    renderer::{DomRenderer, Renderer},
};
use std::marker::PhantomData;

#[inline(always)]
pub fn inner_html<T, R>(value: T) -> InnerHtml<T, R>
where
    T: AsRef<str>,
    R: DomRenderer,
{
    InnerHtml {
        value,
        rndr: PhantomData,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InnerHtml<T, R>
where
    T: AsRef<str>,
    R: Renderer,
{
    value: T,
    rndr: PhantomData<R>,
}

impl<T, R> Attribute<R> for InnerHtml<T, R>
where
    T: AsRef<str> + PartialEq,
    R: DomRenderer,
{
    const MIN_LENGTH: usize = 0;

    type State = (R::Element, T);

    fn to_html(
        self,
        _buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        inner_html: &mut String,
    ) {
        inner_html.push_str(self.value.as_ref());
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &<R as Renderer>::Element,
    ) -> Self::State {
        (el.clone(), self.value)
    }

    fn build(self, el: &<R as Renderer>::Element) -> Self::State {
        R::set_inner_html(el, self.value.as_ref());
        (el.clone(), self.value)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self.value != *prev {
            R::set_inner_html(el, self.value.as_ref());
            *prev = self.value;
        }
    }
}

pub trait InnerHtmlAttribute<T, Rndr>
where
    T: AsRef<str>,
    Rndr: DomRenderer,
    Self: Sized + AddAttribute<InnerHtml<T, Rndr>, Rndr>,
{
    fn inner_html(
        self,
        value: T,
    ) -> <Self as AddAttribute<InnerHtml<T, Rndr>, Rndr>>::Output {
        self.add_attr(inner_html(value))
    }
}

impl<T, E, At, Rndr> InnerHtmlAttribute<T, Rndr>
    for HtmlElement<E, At, (), Rndr>
where
    Self: AddAttribute<InnerHtml<T, Rndr>, Rndr>,
    E: ElementWithChildren,
    At: Attribute<Rndr>,
    T: AsRef<str>,
    Rndr: DomRenderer,
{
    fn inner_html(
        self,
        value: T,
    ) -> <Self as AddAttribute<InnerHtml<T, Rndr>, Rndr>>::Output {
        self.add_attr(inner_html(value))
    }
}
