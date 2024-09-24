use super::Lang;
use crate::{
    html::{
        attribute::*,
        class::{class, Class, IntoClass},
        element::{ElementType, HasElementType, HtmlElement},
        event::{on, on_target, EventDescriptor, On, Targeted},
        property::{prop, IntoProperty, Property},
        style::{style, IntoStyle, Style},
    },
    prelude::RenderHtml,
    view::add_attr::AddAnyAttr,
};
use core::convert::From;

/// Adds an attribute that modifies the `class`.
pub trait ClassAttribute<C>
where
    C: IntoClass,
{
    /// The type of the element with the new attribute added.
    type Output;

    /// Adds a CSS class to an element.
    fn class(self, value: C) -> Self::Output;
}

impl<E, At, Ch, C> ClassAttribute<C> for HtmlElement<E, At, Ch>
where
    E: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
    C: IntoClass,
{
    type Output = <Self as AddAnyAttr>::Output<Class<C>>;

    fn class(self, value: C) -> Self::Output {
        self.add_any_attr(class(value))
    }
}

/// Adds an attribute that modifies the DOM properties.
pub trait PropAttribute<K, P>
where
    P: IntoProperty,
{
    /// The type of the element with the new attribute added.
    type Output;

    /// Adds a DOM property to an element.
    fn prop(self, key: K, value: P) -> Self::Output;
}

impl<E, At, Ch, K, P> PropAttribute<K, P> for HtmlElement<E, At, Ch>
where
    E: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
    K: AsRef<str> + Send,
    P: IntoProperty,
{
    type Output = <Self as AddAnyAttr>::Output<Property<K, P>>;

    fn prop(self, key: K, value: P) -> Self::Output {
        self.add_any_attr(prop(key, value))
    }
}

/// Adds an attribute that modifies the CSS styles.
pub trait StyleAttribute<S>
where
    S: IntoStyle,
{
    /// The type of the element with the new attribute added.
    type Output;

    /// Adds a CSS style to an element.
    fn style(self, value: S) -> Self::Output;
}

impl<E, At, Ch, S> StyleAttribute<S> for HtmlElement<E, At, Ch>
where
    E: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
    S: IntoStyle,
{
    type Output = <Self as AddAnyAttr>::Output<Style<S>>;

    fn style(self, value: S) -> Self::Output {
        self.add_any_attr(style(value))
    }
}

/// Adds an event listener to an element definition.
pub trait OnAttribute<E, F> {
    /// The type of the element with the event listener added.
    type Output;

    /// Adds an event listener to an element.
    fn on(self, event: E, cb: F) -> Self::Output;
}

impl<El, At, Ch, E, F> OnAttribute<E, F> for HtmlElement<El, At, Ch>
where
    El: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
    E: EventDescriptor + Send + 'static,
    E::EventType: 'static,
    E::EventType: From<crate::renderer::types::Event>,
    F: FnMut(E::EventType) + 'static,
{
    type Output = <Self as AddAnyAttr>::Output<On<E, F>>;

    fn on(self, event: E, cb: F) -> Self::Output {
        self.add_any_attr(on(event, cb))
    }
}

/// Adds an event listener with a typed target to an element definition.
pub trait OnTargetAttribute<E, F, T> {
    /// The type of the element with the new attribute added.
    type Output;

    /// Adds an event listener with a typed target to an element definition.
    fn on_target(self, event: E, cb: F) -> Self::Output;
}

impl<El, At, Ch, E, F> OnTargetAttribute<E, F, Self> for HtmlElement<El, At, Ch>
where
    El: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
    E: EventDescriptor + Send + 'static,
    E::EventType: 'static,
    E::EventType: From<crate::renderer::types::Event>,
    F: FnMut(Targeted<E::EventType, <Self as HasElementType>::ElementType>)
        + 'static,
{
    type Output =
        <Self as AddAnyAttr>::Output<On<E, Box<dyn FnMut(E::EventType)>>>;

    fn on_target(self, event: E, cb: F) -> Self::Output {
        self.add_any_attr(on_target::<E, HtmlElement<El, At, Ch>, F>(event, cb))
    }
}

/// Global attributes can be added to any HTML element.
pub trait GlobalAttributes<V>
where
    Self: Sized + AddAnyAttr,
    V: AttributeValue,
{
    /// The `accesskey` global attribute provides a hint for generating a keyboard shortcut for the current element.
    fn accesskey(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<Accesskey, V>> {
        self.add_any_attr(accesskey(value))
    }

    /// The `autocapitalize` global attribute controls whether and how text input is automatically capitalized as it is entered/edited by the user.
    fn autocapitalize(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<Autocapitalize, V>> {
        self.add_any_attr(autocapitalize(value))
    }

    /// The `autofocus` global attribute is a Boolean attribute indicating that an element should receive focus as soon as the page is loaded.
    fn autofocus(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<Autofocus, V>> {
        self.add_any_attr(autofocus(value))
    }

    /// The `contenteditable` global attribute is an enumerated attribute indicating if the element should be editable by the user.
    fn contenteditable(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<Contenteditable, V>> {
        self.add_any_attr(contenteditable(value))
    }

    /// The `dir` global attribute is an enumerated attribute indicating the directionality of the element's text.
    fn dir(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Dir, V>> {
        self.add_any_attr(dir(value))
    }

    /// The `draggable` global attribute is an enumerated attribute indicating whether the element can be dragged.
    fn draggable(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<Draggable, V>> {
        self.add_any_attr(draggable(value))
    }

    /// The `enterkeyhint` global attribute is used to customize the enter key on virtual keyboards.
    fn enterkeyhint(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<Enterkeyhint, V>> {
        self.add_any_attr(enterkeyhint(value))
    }

    /// The `hidden` global attribute is a Boolean attribute indicating that the element is not yet, or is no longer, relevant.
    fn hidden(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Hidden, V>> {
        self.add_any_attr(hidden(value))
    }

    /// The `id` global attribute defines a unique identifier (ID) which must be unique in the whole document.
    fn id(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Id, V>> {
        self.add_any_attr(id(value))
    }

    /// The `inert` global attribute is a Boolean attribute that makes an element behave inertly.
    fn inert(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Inert, V>> {
        self.add_any_attr(inert(value))
    }

    /// The `inputmode` global attribute provides a hint to browsers for which virtual keyboard to display.
    fn inputmode(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<Inputmode, V>> {
        self.add_any_attr(inputmode(value))
    }

    /// The `is` global attribute allows you to specify that a standard HTML element should behave like a custom built-in element.
    fn is(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Is, V>> {
        self.add_any_attr(is(value))
    }

    /// The `itemid` global attribute is used to specify the unique, global identifier of an item.
    fn itemid(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Itemid, V>> {
        self.add_any_attr(itemid(value))
    }

    /// The `itemprop` global attribute is used to add properties to an item.
    fn itemprop(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<Itemprop, V>> {
        self.add_any_attr(itemprop(value))
    }

    /// The `itemref` global attribute is used to refer to other elements.
    fn itemref(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<Itemref, V>> {
        self.add_any_attr(itemref(value))
    }

    /// The `itemscope` global attribute is used to create a new item.
    fn itemscope(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<Itemscope, V>> {
        self.add_any_attr(itemscope(value))
    }

    /// The `itemtype` global attribute is used to specify the types of items.
    fn itemtype(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<Itemtype, V>> {
        self.add_any_attr(itemtype(value))
    }

    /// The `lang` global attribute helps define the language of an element.
    fn lang(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Lang, V>> {
        self.add_any_attr(lang(value))
    }

    /// The `nonce` global attribute is used to specify a cryptographic nonce.
    fn nonce(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Nonce, V>> {
        self.add_any_attr(nonce(value))
    }

    /// The `part` global attribute identifies the element as a part of a component.
    fn part(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Part, V>> {
        self.add_any_attr(part(value))
    }

    /// The `popover` global attribute defines the popover's behavior.
    fn popover(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<Popover, V>> {
        self.add_any_attr(popover(value))
    }

    /// The `role` global attribute defines the role of an element in ARIA.
    fn role(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Role, V>> {
        self.add_any_attr(role(value))
    }

    /// The `slot` global attribute assigns a slot in a shadow DOM.
    fn slot(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Slot, V>> {
        self.add_any_attr(slot(value))
    }

    /// The `spellcheck` global attribute is an enumerated attribute that defines whether the element may be checked for spelling errors.
    fn spellcheck(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<Spellcheck, V>> {
        self.add_any_attr(spellcheck(value))
    }

    /// The `tabindex` global attribute indicates if the element can take input focus.
    fn tabindex(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<Tabindex, V>> {
        self.add_any_attr(tabindex(value))
    }

    /// The `title` global attribute contains text representing advisory information.
    fn title(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Title, V>> {
        self.add_any_attr(title(value))
    }

    /// The `translate` global attribute is an enumerated attribute that specifies whether an element's attribute values and text content should be translated when the page is localized.
    fn translate(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<Translate, V>> {
        self.add_any_attr(translate(value))
    }

    /// The `virtualkeyboardpolicy` global attribute specifies the behavior of the virtual keyboard.
    fn virtualkeyboardpolicy(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<Virtualkeyboardpolicy, V>> {
        self.add_any_attr(virtualkeyboardpolicy(value))
    }
}

impl<El, At, Ch, V> GlobalAttributes<V> for HtmlElement<El, At, Ch>
where
    El: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
    V: AttributeValue,
{
}

macro_rules! on_definitions {
	($(#[$meta:meta] $key:ident $html:literal),* $(,)?) => {
        paste::paste! {
            $(
                #[doc = concat!("Adds the HTML `", $html, "` attribute to the element.\n\n**Note**: This is the HTML attribute, which takes a JavaScript string, not an `on:` listener that takes application logic written in Rust.")]
                #[track_caller]
                fn $key(
                    self,
                    value: V,
                ) -> <Self as AddAnyAttr>::Output<Attr<[<$key:camel>], V>>
                {
                    self.add_any_attr($key(value))
                }
            )*
		}
    }
}

/// Provides methods for HTML event listener attributes.
pub trait GlobalOnAttributes<V>
where
    Self: Sized + AddAnyAttr,
    V: AttributeValue,
{
    on_definitions! {
        /// The `onabort` attribute specifies the event handler for the abort event.
        onabort "onabort",
        /// The `onautocomplete` attribute specifies the event handler for the autocomplete event.
        onautocomplete "onautocomplete",
        /// The `onautocompleteerror` attribute specifies the event handler for the autocompleteerror event.
        onautocompleteerror "onautocompleteerror",
        /// The `onblur` attribute specifies the event handler for the blur event.
        onblur "onblur",
        /// The `oncancel` attribute specifies the event handler for the cancel event.
        oncancel "oncancel",
        /// The `oncanplay` attribute specifies the event handler for the canplay event.
        oncanplay "oncanplay",
        /// The `oncanplaythrough` attribute specifies the event handler for the canplaythrough event.
        oncanplaythrough "oncanplaythrough",
        /// The `onchange` attribute specifies the event handler for the change event.
        onchange "onchange",
        /// The `onclick` attribute specifies the event handler for the click event.
        onclick "onclick",
        /// The `onclose` attribute specifies the event handler for the close event.
        onclose "onclose",
        /// The `oncontextmenu` attribute specifies the event handler for the contextmenu event.
        oncontextmenu "oncontextmenu",
        /// The `oncuechange` attribute specifies the event handler for the cuechange event.
        oncuechange "oncuechange",
        /// The `ondblclick` attribute specifies the event handler for the double click event.
        ondblclick "ondblclick",
        /// The `ondrag` attribute specifies the event handler for the drag event.
        ondrag "ondrag",
        /// The `ondragend` attribute specifies the event handler for the dragend event.
        ondragend "ondragend",
        /// The `ondragenter` attribute specifies the event handler for the dragenter event.
        ondragenter "ondragenter",
        /// The `ondragleave` attribute specifies the event handler for the dragleave event.
        ondragleave "ondragleave",
        /// The `ondragover` attribute specifies the event handler for the dragover event.
        ondragover "ondragover",
        /// The `ondragstart` attribute specifies the event handler for the dragstart event.
        ondragstart "ondragstart",
        /// The `ondrop` attribute specifies the event handler for the drop event.
        ondrop "ondrop",
        /// The `ondurationchange` attribute specifies the event handler for the durationchange event.
        ondurationchange "ondurationchange",
        /// The `onemptied` attribute specifies the event handler for the emptied event.
        onemptied "onemptied",
        /// The `onended` attribute specifies the event handler for the ended event.
        onended "onended",
        /// The `onerror` attribute specifies the event handler for the error event.
        onerror "onerror",
        /// The `onfocus` attribute specifies the event handler for the focus event.
        onfocus "onfocus",
        /// The `onformdata` attribute specifies the event handler for the formdata event.
        onformdata "onformdata",
        /// The `oninput` attribute specifies the event handler for the input event.
        oninput "oninput",
        /// The `oninvalid` attribute specifies the event handler for the invalid event.
        oninvalid "oninvalid",
        /// The `onkeydown` attribute specifies the event handler for the keydown event.
        onkeydown "onkeydown",
        /// The `onkeypress` attribute specifies the event handler for the keypress event.
        onkeypress "onkeypress",
        /// The `onkeyup` attribute specifies the event handler for the keyup event.
        onkeyup "onkeyup",
        /// The `onlanguagechange` attribute specifies the event handler for the languagechange event.
        onlanguagechange "onlanguagechange",
        /// The `onload` attribute specifies the event handler for the load event.
        onload "onload",
        /// The `onloadeddata` attribute specifies the event handler for the loadeddata event.
        onloadeddata "onloadeddata",
        /// The `onloadedmetadata` attribute specifies the event handler for the loadedmetadata event.
        onloadedmetadata "onloadedmetadata",
        /// The `onloadstart` attribute specifies the event handler for the loadstart event.
        onloadstart "onloadstart",
        /// The `onmousedown` attribute specifies the event handler for the mousedown event.
        onmousedown "onmousedown",
        /// The `onmouseenter` attribute specifies the event handler for the mouseenter event.
        onmouseenter "onmouseenter",
        /// The `onmouseleave` attribute specifies the event handler for the mouseleave event.
        onmouseleave "onmouseleave",
        /// The `onmousemove` attribute specifies the event handler for the mousemove event.
        onmousemove "onmousemove",
        /// The `onmouseout` attribute specifies the event handler for the mouseout event.
        onmouseout "onmouseout",
        /// The `onmouseover` attribute specifies the event handler for the mouseover event.
        onmouseover "onmouseover",
        /// The `onmouseup` attribute specifies the event handler for the mouseup event.
        onmouseup "onmouseup",
        /// The `onpause` attribute specifies the event handler for the pause event.
        onpause "onpause",
        /// The `onplay` attribute specifies the event handler for the play event.
        onplay "onplay",
        /// The `onplaying` attribute specifies the event handler for the playing event.
        onplaying "onplaying",
        /// The `onprogress` attribute specifies the event handler for the progress event.
        onprogress "onprogress",
        /// The `onratechange` attribute specifies the event handler for the ratechange event.
        onratechange "onratechange",
        /// The `onreset` attribute specifies the event handler for the reset event.
        onreset "onreset",
        /// The `onresize` attribute specifies the event handler for the resize event.
        onresize "onresize",
        /// The `onscroll` attribute specifies the event handler for the scroll event.
        onscroll "onscroll",
        /// The `onsecuritypolicyviolation` attribute specifies the event handler for the securitypolicyviolation event.
        onsecuritypolicyviolation "onsecuritypolicyviolation",
        /// The `onseeked` attribute specifies the event handler for the seeked event.
        onseeked "onseeked",
        /// The `onseeking` attribute specifies the event handler for the seeking event.
        onseeking "onseeking",
        /// The `onselect` attribute specifies the event handler for the select event.
        onselect "onselect",
        /// The `onslotchange` attribute specifies the event handler for the slotchange event.
        onslotchange "onslotchange",
        /// The `onstalled` attribute specifies the event handler for the stalled event.
        onstalled "onstalled",
        /// The `onsubmit` attribute specifies the event handler for the submit event.
        onsubmit "onsubmit",
        /// The `onsuspend` attribute specifies the event handler for the suspend event.
        onsuspend "onsuspend",
        /// The `ontimeupdate` attribute specifies the event handler for the timeupdate event.
        ontimeupdate "ontimeupdate",
        /// The `ontoggle` attribute specifies the event handler for the toggle event.
        ontoggle "ontoggle",
        /// The `onvolumechange` attribute specifies the event handler for the volumechange event.
        onvolumechange "onvolumechange",
        /// The `onwaiting` attribute specifies the event handler for the waiting event.
        onwaiting "onwaiting",
        /// The `onwebkitanimationend` attribute specifies the event handler for the webkitanimationend event.
        onwebkitanimationend "onwebkitanimationend",
        /// The `onwebkitanimationiteration` attribute specifies the event handler for the webkitanimationiteration event.
        onwebkitanimationiteration "onwebkitanimationiteration",
        /// The `onwebkitanimationstart` attribute specifies the event handler for the webkitanimationstart event.
        onwebkitanimationstart "onwebkitanimationstart",
        /// The `onwebkittransitionend` attribute specifies the event handler for the webkittransitionend event.
        onwebkittransitionend "onwebkittransitionend",
        /// The `onwheel` attribute specifies the event handler for the wheel event.
        onwheel "onwheel",

    }
}

impl<El, At, Ch, V> GlobalOnAttributes<V> for HtmlElement<El, At, Ch>
where
    El: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
    V: AttributeValue,
{
}
