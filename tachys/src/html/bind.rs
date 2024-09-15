use crate::{
    dom::{event_target_checked, event_target_value},
    html::{
        attribute::{
            Attr, Attribute, AttributeKey, AttributeValue, NextAttribute,
        },
        event::{change, input, on},
    },
    prelude::AddAnyAttr,
    renderer::{DomRenderer, RemoveEventHandler, Renderer},
    view::{Position, ToTemplate},
};
use reactive_graph::{
    signal::{ReadSignal, RwSignal, WriteSignal},
    traits::Update,
};
use std::marker::PhantomData;

/// Adds a two-way binding to the element, which adds an attribute and an event listener to the
/// element when the element is created or hydrated.
pub trait BindAttribute<Key, Sig, T, Rndr>
where
    Key: AttributeKey,
    Sig: IntoSplitSignal<Rndr, Value = T>,
    T: FromEventTarget + AttributeValue<Rndr> + 'static,
    Rndr: Renderer,
{
    /// The type of the element with the two-way binding added.
    type Output;

    /// Adds a two-way binding to the element, which adds an attribute and an event listener to the
    /// element when the element is created or hydrated.
    ///
    /// Example:
    ///
    /// ```
    /// // You can use `RwSignal`s
    /// let is_awesome = RwSignal::new(true);
    ///
    /// // And you can use split signals
    /// let (text, set_text) = signal("Hello world".to_string());
    ///
    /// // Use `checked` and a `bool` signal for a checkbox
    /// checkbox_element.bind(checked, is_awesome);
    ///
    /// // Use `value` and `String` for everything else
    /// input_element.bind(value, (text, set_text));
    /// ```
    fn bind(self, key: Key, signal: Sig) -> Self::Output;
}

impl<V, Key, Sig, T, Rndr> BindAttribute<Key, Sig, T, Rndr> for V
where
    V: AddAnyAttr<Rndr>,
    Key: AttributeKey,
    Sig: IntoSplitSignal<Rndr, Value = T>,
    T: FromEventTarget + AttributeValue<Rndr> + 'static,
    <Sig as IntoSplitSignal<Rndr>>::Read:
        AttributeValue<Rndr> + Clone + 'static,
    <Sig as IntoSplitSignal<Rndr>>::Write: Send + Clone + 'static,
    Rndr: Renderer + DomRenderer,
    Rndr::Element: ChangeEvent,
    web_sys::Event: From<<Rndr as DomRenderer>::Event>,
{
    type Output = <Self as AddAnyAttr<Rndr>>::Output<
        Bind<
            Key,
            T,
            <Sig as IntoSplitSignal<Rndr>>::Read,
            <Sig as IntoSplitSignal<Rndr>>::Write,
            Rndr,
        >,
    >;

    fn bind(self, key: Key, signal: Sig) -> Self::Output {
        self.add_any_attr(bind(key, signal))
    }
}

/// Adds a two-way binding to the element, which adds an attribute and an event listener to the
/// element when the element is created or hydrated.
#[inline(always)]
pub fn bind<Key, Sig, T, Rndr>(
    key: Key,
    signal: Sig,
) -> Bind<
    Key,
    T,
    <Sig as IntoSplitSignal<Rndr>>::Read,
    <Sig as IntoSplitSignal<Rndr>>::Write,
    Rndr,
>
where
    Key: AttributeKey,
    Sig: IntoSplitSignal<Rndr, Value = T>,
    T: FromEventTarget + AttributeValue<Rndr> + 'static,
    <Sig as IntoSplitSignal<Rndr>>::Read:
        AttributeValue<Rndr> + Clone + 'static,
    <Sig as IntoSplitSignal<Rndr>>::Write: Send + Clone + 'static,
    Rndr: Renderer,
{
    let (read_signal, write_signal) = signal.into_split_signal();

    let attr = Attr(key, read_signal, PhantomData);

    Bind {
        attr,
        write_signal,
        _marker: PhantomData,
    }
}

/// Two-way binding of an attribute and an event listener
#[derive(Debug)]
pub struct Bind<Key, T, R, W, Rndr>
where
    Key: AttributeKey,
    T: FromEventTarget + AttributeValue<Rndr> + 'static,
    R: AttributeValue<Rndr> + Clone + 'static,
    W: Update<Value = T>,
    Rndr: Renderer,
{
    attr: Attr<Key, R, Rndr>,
    write_signal: W,
    _marker: PhantomData<Rndr>,
}

impl<Key, T, R, W, Rndr> Clone for Bind<Key, T, R, W, Rndr>
where
    Key: AttributeKey,
    T: FromEventTarget + AttributeValue<Rndr> + 'static,
    R: AttributeValue<Rndr> + Clone + 'static,
    W: Update<Value = T> + Clone,
    Rndr: Renderer,
{
    fn clone(&self) -> Self {
        Self {
            attr: self.attr.clone(),
            write_signal: self.write_signal.clone(),
            _marker: PhantomData,
        }
    }
}

impl<Key, T, R, W, Rndr> Bind<Key, T, R, W, Rndr>
where
    Key: AttributeKey,
    T: FromEventTarget + AttributeValue<Rndr> + 'static,
    R: AttributeValue<Rndr> + Clone + 'static,
    W: Update<Value = T> + 'static,
    Rndr: Renderer + DomRenderer,
    Rndr::Element: ChangeEvent,
    web_sys::Event: From<<Rndr as DomRenderer>::Event>,
{
    /// Attaches the event listener that updates the signal value to the element.
    pub fn attach(
        self,
        el: &Rndr::Element,
    ) -> RemoveEventHandler<Rndr::Element> {
        let handler = move |evt| {
            self.write_signal
                .try_update(|v| *v = T::from_event_target(&evt));
        };

        el.attach_change_event::<_, Rndr>(Key::KEY, handler)
    }
}

impl<Key, T, R, W, Rndr> Attribute<Rndr> for Bind<Key, T, R, W, Rndr>
where
    Key: AttributeKey,
    T: FromEventTarget + AttributeValue<Rndr> + 'static,
    R: AttributeValue<Rndr> + Clone + 'static,
    W: Update<Value = T> + Clone + Send + 'static,
    Rndr: Renderer + DomRenderer,
    Rndr::Element: ChangeEvent,
    web_sys::Event: From<<Rndr as DomRenderer>::Event>,
{
    const MIN_LENGTH: usize = 0;

    type AsyncOutput = Self;
    type State = (
        R::State,
        (Rndr::Element, Option<RemoveEventHandler<Rndr::Element>>),
    );
    type Cloneable = Bind<Key, T, R, W, Rndr>;
    type CloneableOwned = Bind<Key, T, R, W, Rndr>;

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

    #[inline(always)]
    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &Rndr::Element,
    ) -> Self::State {
        let attr_state = self.attr.clone().hydrate::<FROM_SERVER>(el);
        let cleanup = self.attach(el);
        (attr_state, (el.clone(), Some(cleanup)))
    }

    #[inline(always)]
    fn build(self, el: &Rndr::Element) -> Self::State {
        let attr_state = self.attr.clone().build(el);
        let cleanup = self.attach(el);
        (attr_state, (el.clone(), Some(cleanup)))
    }

    #[inline(always)]
    fn rebuild(self, state: &mut Self::State) {
        let (attr_state, (el, prev_cleanup)) = state;
        let _ = self.attr.clone().rebuild(attr_state);
        if let Some(prev) = prev_cleanup.take() {
            (prev.into_inner())(el);
        }
        *prev_cleanup = Some(self.attach(el));
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into_cloneable_owned()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }

    fn dry_resolve(&mut self) {
        self.attr.dry_resolve()
    }

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl<Key, T, R, W, Rndr> NextAttribute<Rndr> for Bind<Key, T, R, W, Rndr>
where
    Key: AttributeKey,
    T: FromEventTarget + AttributeValue<Rndr> + 'static,
    R: AttributeValue<Rndr> + Clone + 'static,
    W: Update<Value = T> + Clone + Send + 'static,
    Rndr: Renderer + DomRenderer,
    Rndr::Element: ChangeEvent,
    web_sys::Event: From<<Rndr as DomRenderer>::Event>,
{
    type Output<NewAttr: Attribute<Rndr>> = (Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        (self, new_attr)
    }
}

impl<Key, T, R, W, Rndr> ToTemplate for Bind<Key, T, R, W, Rndr>
where
    Key: AttributeKey,
    T: FromEventTarget + AttributeValue<Rndr> + 'static,
    R: AttributeValue<Rndr> + Clone + 'static,
    W: Update<Value = T> + Clone,
    Rndr: Renderer,
{
    #[inline(always)]
    fn to_template(
        _buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
        _position: &mut Position,
    ) {
    }
}

/// Splits a combined signal into its read and write parts.
///
/// This allows you to either provide a `RwSignal` or a tuple `(ReadSignal, WriteSignal)`.
pub trait IntoSplitSignal<Rndr: Renderer> {
    /// The actual contained value of the signal
    type Value;
    /// The read part of the signal
    type Read: AttributeValue<Rndr>;
    /// The write part of the signal
    type Write: Update<Value = Self::Value>;
    /// Splits a combined signal into its read and write parts.
    fn into_split_signal(self) -> (Self::Read, Self::Write);
}

impl<T, Rndr> IntoSplitSignal<Rndr> for RwSignal<T>
where
    T: Send + Sync + 'static,
    ReadSignal<T>: AttributeValue<Rndr>,
    Rndr: Renderer,
{
    type Value = T;
    type Read = ReadSignal<T>;
    type Write = WriteSignal<T>;

    fn into_split_signal(self) -> (ReadSignal<T>, WriteSignal<T>) {
        self.split()
    }
}

impl<T, R, W, Rndr> IntoSplitSignal<Rndr> for (R, W)
where
    R: AttributeValue<Rndr>,
    W: Update<Value = T>,
    Rndr: Renderer,
{
    type Value = T;
    type Read = R;
    type Write = W;

    fn into_split_signal(self) -> (Self::Read, Self::Write) {
        self
    }
}

/// Returns self from an event target.
pub trait FromEventTarget {
    /// Returns self from an event target.
    fn from_event_target(evt: &web_sys::Event) -> Self;
}

impl FromEventTarget for bool {
    fn from_event_target(evt: &web_sys::Event) -> Self {
        event_target_checked(evt)
    }
}

impl FromEventTarget for String {
    fn from_event_target(evt: &web_sys::Event) -> Self {
        event_target_value(evt)
    }
}

/// Attaches the appropriate change event listener to the element.
/// - `<input>` with text types and `<textarea>` elements use the `input` event;
/// - `<input type="checkbox">`, `<input type="radio">` and `<select>` use the `change` event;
pub trait ChangeEvent {
    /// Attaches the appropriate change event listener to the element.
    fn attach_change_event<F, Rndr>(
        &self,
        key: &str,
        handler: F,
    ) -> RemoveEventHandler<Self>
    where
        F: FnMut(web_sys::Event) + 'static,
        Rndr: Renderer<Element = Self> + DomRenderer,
        web_sys::Event: From<<Rndr as DomRenderer>::Event>,
        Self: Sized;
}

impl ChangeEvent for web_sys::Element {
    fn attach_change_event<F, Rndr>(
        &self,
        key: &str,
        handler: F,
    ) -> RemoveEventHandler<Self>
    where
        F: FnMut(web_sys::Event) + 'static,
        Rndr: Renderer<Element = Self> + DomRenderer,
        web_sys::Event: From<<Rndr as DomRenderer>::Event>,
    {
        if key == "checked" || self.tag_name() == "SELECT" {
            on::<_, _, Rndr>(change, handler).attach(self)
        } else {
            on::<_, _, Rndr>(input, handler).attach(self)
        }
    }
}
