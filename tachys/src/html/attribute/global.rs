use super::Lang;
use crate::{
    html::{
        attribute::*,
        class::{class, Class, IntoClass},
        element::ElementType,
        event::{on, on_target, EventDescriptor, On, Targeted},
        property::{property, IntoProperty, Property},
        style::{style, IntoStyle, Style},
    },
    renderer::DomRenderer,
    view::add_attr::AddAnyAttr,
};
use core::convert::From;

pub trait ClassAttribute<C, Rndr>
where
    C: IntoClass<Rndr>,
    Rndr: DomRenderer,
{
    type Output;

    fn class(self, value: C) -> Self::Output;
}

impl<T, C, Rndr> ClassAttribute<C, Rndr> for T
where
    T: AddAnyAttr<Rndr>,
    C: IntoClass<Rndr>,
    Rndr: DomRenderer,
{
    type Output = <Self as AddAnyAttr<Rndr>>::Output<Class<C, Rndr>>;

    fn class(self, value: C) -> Self::Output {
        self.add_any_attr(class(value))
    }
}

pub trait PropAttribute<K, P, Rndr>
where
    P: IntoProperty<Rndr>,
    Rndr: DomRenderer,
{
    type Output;

    fn prop(self, key: K, value: P) -> Self::Output;
}

impl<T, K, P, Rndr> PropAttribute<K, P, Rndr> for T
where
    T: AddAnyAttr<Rndr>,
    K: AsRef<str>,
    P: IntoProperty<Rndr>,
    Rndr: DomRenderer,
{
    type Output = <Self as AddAnyAttr<Rndr>>::Output<Property<K, P, Rndr>>;
    fn prop(self, key: K, value: P) -> Self::Output {
        self.add_any_attr(property(key, value))
    }
}

pub trait StyleAttribute<S, Rndr>
where
    S: IntoStyle<Rndr>,
    Rndr: DomRenderer,
{
    type Output;

    fn style(self, value: S) -> Self::Output;
}

impl<T, S, Rndr> StyleAttribute<S, Rndr> for T
where
    T: AddAnyAttr<Rndr>,
    S: IntoStyle<Rndr>,
    Rndr: DomRenderer,
{
    type Output = <Self as AddAnyAttr<Rndr>>::Output<Style<S, Rndr>>;

    fn style(self, value: S) -> Self::Output {
        self.add_any_attr(style(value))
    }
}

pub trait OnAttribute<E, F, Rndr> {
    type Output;

    fn on(self, event: E, cb: F) -> Self::Output;
}

impl<T, E, F, Rndr> OnAttribute<E, F, Rndr> for T
where
    T: AddAnyAttr<Rndr>,
    E: EventDescriptor + 'static,
    E::EventType: 'static,
    E::EventType: From<Rndr::Event>,
    F: FnMut(E::EventType) + 'static,
    Rndr: DomRenderer,
{
    type Output = <Self as AddAnyAttr<Rndr>>::Output<On<Rndr>>;

    fn on(self, event: E, cb: F) -> Self::Output {
        self.add_any_attr(on(event, cb))
    }
}

pub trait OnTargetAttribute<E, F, T, Rndr> {
    type Output;

    fn on_target(self, event: E, cb: F) -> Self::Output;
}

impl<T, E, F, Rndr> OnTargetAttribute<E, F, Self, Rndr> for T
where
    Self: ElementType,
    T: AddAnyAttr<Rndr>,
    E: EventDescriptor + 'static,
    E::EventType: 'static,
    E::EventType: From<Rndr::Event>,
    F: FnMut(Targeted<E::EventType, <Self as ElementType>::Output, Rndr>)
        + 'static,
    Rndr: DomRenderer,
{
    type Output = <Self as AddAnyAttr<Rndr>>::Output<On<Rndr>>;

    fn on_target(self, event: E, cb: F) -> Self::Output {
        self.add_any_attr(on_target(event, cb))
    }
}

pub trait GlobalAttributes<Rndr, V>
where
    Self: Sized + AddAnyAttr<Rndr>,
    V: AttributeValue<Rndr>,
    Rndr: Renderer,
{
    fn accesskey(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Accesskey, V, Rndr>> {
        self.add_any_attr(accesskey(value))
    }

    fn autocapitalize(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Autocapitalize, V, Rndr>> {
        self.add_any_attr(autocapitalize(value))
    }

    fn autofocus(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Autofocus, V, Rndr>> {
        self.add_any_attr(autofocus(value))
    }

    fn contenteditable(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Contenteditable, V, Rndr>>
    {
        self.add_any_attr(contenteditable(value))
    }

    fn dir(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Dir, V, Rndr>> {
        self.add_any_attr(dir(value))
    }

    fn draggable(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Draggable, V, Rndr>> {
        self.add_any_attr(draggable(value))
    }

    fn enterkeyhint(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Enterkeyhint, V, Rndr>> {
        self.add_any_attr(enterkeyhint(value))
    }

    fn hidden(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Hidden, V, Rndr>> {
        self.add_any_attr(hidden(value))
    }

    fn id(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Id, V, Rndr>> {
        self.add_any_attr(id(value))
    }

    fn inert(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Inert, V, Rndr>> {
        self.add_any_attr(inert(value))
    }

    fn inputmode(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Inputmode, V, Rndr>> {
        self.add_any_attr(inputmode(value))
    }

    fn is(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Is, V, Rndr>> {
        self.add_any_attr(is(value))
    }

    fn itemid(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Itemid, V, Rndr>> {
        self.add_any_attr(itemid(value))
    }

    fn itemprop(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Itemprop, V, Rndr>> {
        self.add_any_attr(itemprop(value))
    }

    fn itemref(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Itemref, V, Rndr>> {
        self.add_any_attr(itemref(value))
    }

    fn itemscope(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Itemscope, V, Rndr>> {
        self.add_any_attr(itemscope(value))
    }

    fn itemtype(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Itemtype, V, Rndr>> {
        self.add_any_attr(itemtype(value))
    }

    fn lang(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Lang, V, Rndr>> {
        self.add_any_attr(lang(value))
    }

    fn nonce(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Nonce, V, Rndr>> {
        self.add_any_attr(nonce(value))
    }

    fn part(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Part, V, Rndr>> {
        self.add_any_attr(part(value))
    }

    fn popover(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Popover, V, Rndr>> {
        self.add_any_attr(popover(value))
    }

    fn role(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Role, V, Rndr>> {
        self.add_any_attr(role(value))
    }

    fn slot(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Slot, V, Rndr>> {
        self.add_any_attr(slot(value))
    }

    fn spellcheck(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Spellcheck, V, Rndr>> {
        self.add_any_attr(spellcheck(value))
    }

    fn tabindex(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Tabindex, V, Rndr>> {
        self.add_any_attr(tabindex(value))
    }

    fn title(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Title, V, Rndr>> {
        self.add_any_attr(title(value))
    }

    fn translate(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Translate, V, Rndr>> {
        self.add_any_attr(translate(value))
    }

    fn virtualkeyboardpolicy(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Virtualkeyboardpolicy, V, Rndr>>
    {
        self.add_any_attr(virtualkeyboardpolicy(value))
    }
}

impl<T, Rndr, V> GlobalAttributes<Rndr, V> for T
where
    T: AddAnyAttr<Rndr>,
    V: AttributeValue<Rndr>,
    Rndr: Renderer,
{
}
