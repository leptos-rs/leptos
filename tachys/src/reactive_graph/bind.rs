use crate::{
    dom::{event_target_checked, event_target_value},
    html::{
        attribute::{Attribute, AttributeKey, AttributeValue, NextAttribute},
        event::{change, input, on},
        property::{prop, IntoProperty},
    },
    prelude::AddAnyAttr,
    renderer::{DomRenderer, RemoveEventHandler, Renderer},
    view::{Position, ToTemplate},
};
use reactive_graph::{
    signal::{ReadSignal, RwSignal, WriteSignal},
    traits::{Get, Update},
    wrappers::read::Signal,
};
use send_wrapper::SendWrapper;
use std::marker::PhantomData;
use wasm_bindgen::JsValue;

/// `group` attribute used for radio inputs with `bind`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Group;

impl AttributeKey for Group {
    const KEY: &'static str = "group";
}

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
    /// // Use `Checked` and a `bool` signal for a checkbox
    /// checkbox_element.bind(Checked, is_awesome);
    ///
    /// // Use `Group` and `String` for radio inputs
    /// radio_element.bind(Group, (text, set_text));
    ///
    /// // Use `Value` and `String` for everything else
    /// input_element.bind(Value, (text, set_text));
    /// ```
    ///
    /// Depending on the input different events are listened to.
    /// - `<input type="checkbox">`, `<input type="radio">` and `<select>` use the `change` event;
    /// - `<input>` with the rest of the types and `<textarea>` elements use the `input` event;
    fn bind(self, key: Key, signal: Sig) -> Self::Output;
}

impl<V, Key, Sig, T, Rndr> BindAttribute<Key, Sig, T, Rndr> for V
where
    V: AddAnyAttr<Rndr>,
    Key: AttributeKey,
    Sig: IntoSplitSignal<Rndr, Value = T>,
    T: FromEventTarget + AttributeValue<Rndr> + PartialEq + Sync + 'static,
    Signal<BoolOrT<T>>: IntoProperty<Rndr>,
    <Sig as IntoSplitSignal<Rndr>>::Read:
        Get<Value = T> + Send + Sync + Clone + 'static,
    <Sig as IntoSplitSignal<Rndr>>::Write: Send + Clone + 'static,
    Rndr: Renderer + DomRenderer,
    Rndr::Element: ChangeEvent + GetValue<T>,
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
    <Sig as IntoSplitSignal<Rndr>>::Read: Get<Value = T> + Clone + 'static,
    <Sig as IntoSplitSignal<Rndr>>::Write: Send + Clone + 'static,
    Rndr: Renderer,
{
    let (read_signal, write_signal) = signal.into_split_signal();

    Bind {
        key,
        read_signal,
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
    R: Get<Value = T> + Clone + 'static,
    W: Update<Value = T>,
    Rndr: Renderer,
{
    key: Key,
    read_signal: R,
    write_signal: W,
    _marker: PhantomData<Rndr>,
}

impl<Key, T, R, W, Rndr> Clone for Bind<Key, T, R, W, Rndr>
where
    Key: AttributeKey,
    T: FromEventTarget + AttributeValue<Rndr> + 'static,
    R: Get<Value = T> + Clone + 'static,
    W: Update<Value = T> + Clone,
    Rndr: Renderer,
{
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            read_signal: self.read_signal.clone(),
            write_signal: self.write_signal.clone(),
            _marker: PhantomData,
        }
    }
}

impl<Key, T, R, W, Rndr> Bind<Key, T, R, W, Rndr>
where
    Key: AttributeKey,
    T: FromEventTarget + AttributeValue<Rndr> + PartialEq + Sync + 'static,
    R: Get<Value = T> + Clone + Send + Sync + 'static,
    W: Update<Value = T> + Clone + 'static,
    Rndr: Renderer + DomRenderer,
    Rndr::Element: ChangeEvent + GetValue<T>,
    web_sys::Event: From<<Rndr as DomRenderer>::Event>,
{
    /// Attaches the event listener that updates the signal value to the element.
    pub fn attach(
        self,
        el: &Rndr::Element,
    ) -> RemoveEventHandler<Rndr::Element> {
        el.attach_change_event::<T, W, Rndr>(
            Key::KEY,
            self.write_signal.clone(),
        )
    }

    /// Creates the signal to update the value of the attribute. This signal is different
    /// when using a `"group"` attribute
    pub fn read_signal(&self, el: &Rndr::Element) -> Signal<BoolOrT<T>> {
        let read_signal = self.read_signal.clone();

        if Key::KEY == "group" {
            let el = SendWrapper::new(el.clone());

            Signal::derive(move || {
                BoolOrT::Bool(el.get_value() == read_signal.get())
            })
        } else {
            Signal::derive(move || BoolOrT::T(read_signal.get()))
        }
    }

    /// Returns the key of the attribute. If the key is `"group"` it returns `"checked"`, otherwise
    /// the one which was provided originally.
    pub fn key(&self) -> &'static str {
        if Key::KEY == "group" {
            "checked"
        } else {
            Key::KEY
        }
    }
}

impl<Key, T, R, W, Rndr> Attribute<Rndr> for Bind<Key, T, R, W, Rndr>
where
    Key: AttributeKey,
    T: FromEventTarget + AttributeValue<Rndr> + PartialEq + Sync + 'static,
    R: Get<Value = T> + Clone + Send + Sync + 'static,
    Signal<BoolOrT<T>>: IntoProperty<Rndr>,
    W: Update<Value = T> + Clone + Send + 'static,
    Rndr: Renderer + DomRenderer,
    Rndr::Element: ChangeEvent + GetValue<T>,
    web_sys::Event: From<<Rndr as DomRenderer>::Event>,
{
    const MIN_LENGTH: usize = 0;

    type State = (
        <Signal<BoolOrT<T>> as IntoProperty<Rndr>>::State,
        (Rndr::Element, Option<RemoveEventHandler<Rndr::Element>>),
    );
    type AsyncOutput = Self;
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
        let signal = self.read_signal(el);
        let attr_state = prop(self.key(), signal).hydrate::<FROM_SERVER>(el);

        let cleanup = self.attach(el);

        (attr_state, (el.clone(), Some(cleanup)))
    }

    #[inline(always)]
    fn build(self, el: &Rndr::Element) -> Self::State {
        let signal = self.read_signal(el);
        let attr_state = prop(self.key(), signal).build(el);

        let cleanup = self.attach(el);

        (attr_state, (el.clone(), Some(cleanup)))
    }

    #[inline(always)]
    fn rebuild(self, state: &mut Self::State) {
        let (attr_state, (el, prev_cleanup)) = state;

        let signal = self.read_signal(el);
        prop(self.key(), signal).rebuild(attr_state);

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

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl<Key, T, R, W, Rndr> NextAttribute<Rndr> for Bind<Key, T, R, W, Rndr>
where
    Key: AttributeKey,
    T: FromEventTarget + AttributeValue<Rndr> + PartialEq + Sync + 'static,
    R: Get<Value = T> + Clone + Send + Sync + 'static,
    Signal<BoolOrT<T>>: IntoProperty<Rndr>,
    W: Update<Value = T> + Clone + Send + 'static,
    Rndr: Renderer + DomRenderer,
    Rndr::Element: ChangeEvent + GetValue<T>,
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
    R: Get<Value = T> + Clone + 'static,
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
    type Read: Get<Value = Self::Value>;
    /// The write part of the signal
    type Write: Update<Value = Self::Value>;
    /// Splits a combined signal into its read and write parts.
    fn into_split_signal(self) -> (Self::Read, Self::Write);
}

impl<T, Rndr> IntoSplitSignal<Rndr> for RwSignal<T>
where
    T: Send + Sync + 'static,
    ReadSignal<T>: Get<Value = T>,
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
    R: Get<Value = T>,
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
    fn attach_change_event<T, W, Rndr>(
        &self,
        key: &str,
        write_signal: W,
    ) -> RemoveEventHandler<Self>
    where
        T: FromEventTarget + AttributeValue<Rndr> + 'static,
        W: Update<Value = T> + 'static,
        Rndr: Renderer<Element = Self> + DomRenderer,
        web_sys::Event: From<<Rndr as DomRenderer>::Event>,
        Self: Sized;
}

impl ChangeEvent for web_sys::Element {
    fn attach_change_event<T, W, Rndr>(
        &self,
        key: &str,
        write_signal: W,
    ) -> RemoveEventHandler<Self>
    where
        T: FromEventTarget + AttributeValue<Rndr> + 'static,
        W: Update<Value = T> + 'static,
        Rndr: Renderer<Element = Self> + DomRenderer,
        web_sys::Event: From<<Rndr as DomRenderer>::Event>,
    {
        if key == "group" {
            let handler = move |evt| {
                let checked = event_target_checked(&evt);
                if checked {
                    write_signal
                        .try_update(|v| *v = T::from_event_target(&evt));
                }
            };

            on::<_, _, Rndr>(change, handler).attach(self)
        } else {
            let handler = move |evt| {
                write_signal.try_update(|v| *v = T::from_event_target(&evt));
            };

            if key == "checked" || self.tag_name() == "SELECT" {
                on::<_, _, Rndr>(change, handler).attach(self)
            } else {
                on::<_, _, Rndr>(input, handler).attach(self)
            }
        }
    }
}

/// Get the value attribute of an element (input).
/// Reads `value` if `T` is `String` and `checked` if `T` is `bool`.
pub trait GetValue<T> {
    /// Get the value attribute of an element (input).
    fn get_value(&self) -> T;
}

impl GetValue<String> for web_sys::Element {
    fn get_value(&self) -> String {
        self.get_attribute("value").unwrap_or_default()
    }
}

impl GetValue<bool> for web_sys::Element {
    fn get_value(&self) -> bool {
        self.get_attribute("checked").unwrap_or_default() == "true"
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// Bool or a type. Needed to make the `group` attribute work. It is decided at runtime
/// if the derived signal value is a bool or a type `T`.
pub enum BoolOrT<T> {
    /// We have definitely a boolean value for the `group` attribute
    Bool(bool),
    /// Standard case with some type `T`
    T(T),
}

impl<T, Rndr> IntoProperty<Rndr> for BoolOrT<T>
where
    T: IntoProperty<Rndr, State = (<Rndr as Renderer>::Element, JsValue)>
        + Into<JsValue>
        + Clone
        + 'static,
    Rndr: DomRenderer,
{
    type State = (Rndr::Element, JsValue);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &Rndr::Element,
        key: &str,
    ) -> Self::State {
        match self.clone() {
            Self::T(s) => {
                s.hydrate::<FROM_SERVER>(el, key);
            }
            Self::Bool(b) => {
                <bool as IntoProperty<Rndr>>::hydrate::<FROM_SERVER>(
                    b, el, key,
                );
            }
        };

        (el.clone(), self.into())
    }

    fn build(self, el: &Rndr::Element, key: &str) -> Self::State {
        match self.clone() {
            Self::T(s) => {
                s.build(el, key);
            }
            Self::Bool(b) => {
                <bool as IntoProperty<Rndr>>::build(b, el, key);
            }
        }

        (el.clone(), self.into())
    }

    fn rebuild(self, state: &mut Self::State, key: &str) {
        let (el, prev) = state;

        match self {
            Self::T(s) => s.rebuild(&mut (el.clone(), prev.clone()), key),
            Self::Bool(b) => <bool as IntoProperty<Rndr>>::rebuild(
                b,
                &mut (el.clone(), prev.clone()),
                key,
            ),
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }
}

impl<T> From<BoolOrT<T>> for JsValue
where
    T: Into<JsValue>,
{
    fn from(value: BoolOrT<T>) -> Self {
        match value {
            BoolOrT::Bool(b) => b.into(),
            BoolOrT::T(t) => t.into(),
        }
    }
}
