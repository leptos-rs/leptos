use super::Lang;
use crate::{
    html::{
        attribute::*,
        class::{class, Class, IntoClass},
        element::HasElementType,
        event::{on, on_target, EventDescriptor, On, Targeted},
        property::{prop, IntoProperty, Property},
        style::{style, IntoStyle, Style},
    },
    renderer::DomRenderer,
    view::add_attr::AddAnyAttr,
};
use core::convert::From;

/// Adds an attribute that modifies the `class`.
pub trait ClassAttribute<C, Rndr>
where
    C: IntoClass<Rndr>,
    Rndr: DomRenderer,
{
    /// The type of the element with the new attribute added.
    type Output;

    /// Adds a CSS class to an element.
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

/// Adds an attribute that modifies the DOM properties.
pub trait PropAttribute<K, P, Rndr>
where
    P: IntoProperty<Rndr>,
    Rndr: DomRenderer,
{
    /// The type of the element with the new attribute added.
    type Output;

    /// Adds a DOM property to an element.
    fn prop(self, key: K, value: P) -> Self::Output;
}

impl<T, K, P, Rndr> PropAttribute<K, P, Rndr> for T
where
    T: AddAnyAttr<Rndr>,
    K: AsRef<str> + Send,
    P: IntoProperty<Rndr>,
    Rndr: DomRenderer,
{
    type Output = <Self as AddAnyAttr<Rndr>>::Output<Property<K, P, Rndr>>;

    fn prop(self, key: K, value: P) -> Self::Output {
        self.add_any_attr(prop(key, value))
    }
}

/// Adds an attribute that modifies the CSS styles.
pub trait StyleAttribute<S, Rndr>
where
    S: IntoStyle<Rndr>,
    Rndr: DomRenderer,
{
    /// The type of the element with the new attribute added.
    type Output;

    /// Adds a CSS style to an element.
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

/// Adds an event listener to an element definition.
pub trait OnAttribute<E, F, Rndr> {
    /// The type of the element with the event listener added.
    type Output;

    /// Adds an event listener to an element.
    fn on(self, event: E, cb: F) -> Self::Output;
}

impl<T, E, F, Rndr> OnAttribute<E, F, Rndr> for T
where
    T: AddAnyAttr<Rndr>,
    E: EventDescriptor + Send + 'static,
    E::EventType: 'static,
    E::EventType: From<Rndr::Event>,
    F: FnMut(E::EventType) + 'static,
    Rndr: DomRenderer,
{
    type Output = <Self as AddAnyAttr<Rndr>>::Output<On<E, F, Rndr>>;

    fn on(self, event: E, cb: F) -> Self::Output {
        self.add_any_attr(on(event, cb))
    }
}

/// Adds an event listener with a typed target to an element definition.
pub trait OnTargetAttribute<E, F, T, Rndr> {
    /// The type of the element with the new attribute added.
    type Output;

    /// Adds an event listener with a typed target to an element definition.
    fn on_target(self, event: E, cb: F) -> Self::Output;
}

impl<T, E, F, Rndr> OnTargetAttribute<E, F, Self, Rndr> for T
where
    T: AddAnyAttr<Rndr> + HasElementType,
    E: EventDescriptor + Send + 'static,
    E::EventType: 'static,
    E::EventType: From<Rndr::Event>,
    F: FnMut(
            Targeted<E::EventType, <Self as HasElementType>::ElementType, Rndr>,
        ) + 'static,
    Rndr: DomRenderer,
{
    type Output = <Self as AddAnyAttr<Rndr>>::Output<
        On<E, Box<dyn FnMut(E::EventType)>, Rndr>,
    >;

    fn on_target(self, event: E, cb: F) -> Self::Output {
        self.add_any_attr(on_target::<E, T, Rndr, F>(event, cb))
    }
}

/// Global attributes can be added to any HTML element.
pub trait GlobalAttributes<Rndr, V>
where
    Self: Sized + AddAnyAttr<Rndr>,
    V: AttributeValue<Rndr>,
    Rndr: Renderer,
{
    /// The `accesskey` global attribute provides a hint for generating a keyboard shortcut for the current element.
    fn accesskey(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Accesskey, V, Rndr>> {
        self.add_any_attr(accesskey(value))
    }

    /// The `autocapitalize` global attribute controls whether and how text input is automatically capitalized as it is entered/edited by the user.
    fn autocapitalize(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Autocapitalize, V, Rndr>> {
        self.add_any_attr(autocapitalize(value))
    }

    /// The `autofocus` global attribute is a Boolean attribute indicating that an element should receive focus as soon as the page is loaded.
    fn autofocus(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Autofocus, V, Rndr>> {
        self.add_any_attr(autofocus(value))
    }

    /// The `contenteditable` global attribute is an enumerated attribute indicating if the element should be editable by the user.
    fn contenteditable(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Contenteditable, V, Rndr>>
    {
        self.add_any_attr(contenteditable(value))
    }

    /// The `dir` global attribute is an enumerated attribute indicating the directionality of the element's text.
    fn dir(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Dir, V, Rndr>> {
        self.add_any_attr(dir(value))
    }

    /// The `draggable` global attribute is an enumerated attribute indicating whether the element can be dragged.
    fn draggable(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Draggable, V, Rndr>> {
        self.add_any_attr(draggable(value))
    }

    /// The `enterkeyhint` global attribute is used to customize the enter key on virtual keyboards.
    fn enterkeyhint(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Enterkeyhint, V, Rndr>> {
        self.add_any_attr(enterkeyhint(value))
    }

    /// The `hidden` global attribute is a Boolean attribute indicating that the element is not yet, or is no longer, relevant.
    fn hidden(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Hidden, V, Rndr>> {
        self.add_any_attr(hidden(value))
    }

    /// The `id` global attribute defines a unique identifier (ID) which must be unique in the whole document.
    fn id(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Id, V, Rndr>> {
        self.add_any_attr(id(value))
    }

    /// The `inert` global attribute is a Boolean attribute that makes an element behave inertly.
    fn inert(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Inert, V, Rndr>> {
        self.add_any_attr(inert(value))
    }

    /// The `inputmode` global attribute provides a hint to browsers for which virtual keyboard to display.
    fn inputmode(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Inputmode, V, Rndr>> {
        self.add_any_attr(inputmode(value))
    }

    /// The `is` global attribute allows you to specify that a standard HTML element should behave like a custom built-in element.
    fn is(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Is, V, Rndr>> {
        self.add_any_attr(is(value))
    }

    /// The `itemid` global attribute is used to specify the unique, global identifier of an item.
    fn itemid(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Itemid, V, Rndr>> {
        self.add_any_attr(itemid(value))
    }

    /// The `itemprop` global attribute is used to add properties to an item.
    fn itemprop(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Itemprop, V, Rndr>> {
        self.add_any_attr(itemprop(value))
    }

    /// The `itemref` global attribute is used to refer to other elements.
    fn itemref(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Itemref, V, Rndr>> {
        self.add_any_attr(itemref(value))
    }

    /// The `itemscope` global attribute is used to create a new item.
    fn itemscope(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Itemscope, V, Rndr>> {
        self.add_any_attr(itemscope(value))
    }

    /// The `itemtype` global attribute is used to specify the types of items.
    fn itemtype(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Itemtype, V, Rndr>> {
        self.add_any_attr(itemtype(value))
    }

    /// The `lang` global attribute helps define the language of an element.
    fn lang(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Lang, V, Rndr>> {
        self.add_any_attr(lang(value))
    }

    /// The `nonce` global attribute is used to specify a cryptographic nonce.
    fn nonce(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Nonce, V, Rndr>> {
        self.add_any_attr(nonce(value))
    }

    /// The `part` global attribute identifies the element as a part of a component.
    fn part(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Part, V, Rndr>> {
        self.add_any_attr(part(value))
    }

    /// The `popover` global attribute defines the popover's behavior.
    fn popover(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Popover, V, Rndr>> {
        self.add_any_attr(popover(value))
    }

    /// The `role` global attribute defines the role of an element in ARIA.
    fn role(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Role, V, Rndr>> {
        self.add_any_attr(role(value))
    }

    /// The `slot` global attribute assigns a slot in a shadow DOM.
    fn slot(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Slot, V, Rndr>> {
        self.add_any_attr(slot(value))
    }

    /// The `spellcheck` global attribute is an enumerated attribute that defines whether the element may be checked for spelling errors.
    fn spellcheck(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Spellcheck, V, Rndr>> {
        self.add_any_attr(spellcheck(value))
    }

    /// The `tabindex` global attribute indicates if the element can take input focus.
    fn tabindex(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Tabindex, V, Rndr>> {
        self.add_any_attr(tabindex(value))
    }

    /// The `title` global attribute contains text representing advisory information.
    fn title(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Title, V, Rndr>> {
        self.add_any_attr(title(value))
    }

    /// The `translate` global attribute is an enumerated attribute that specifies whether an element's attribute values and text content should be translated when the page is localized.
    fn translate(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<Translate, V, Rndr>> {
        self.add_any_attr(translate(value))
    }

    /// The `virtualkeyboardpolicy` global attribute specifies the behavior of the virtual keyboard.
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
