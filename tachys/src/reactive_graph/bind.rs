use crate::{
    dom::{event_target_checked, event_target_value},
    html::{
        attribute::{
            maybe_next_attr_erasure_macros::{
                next_attr_combine, next_attr_output_type,
            },
            Attribute, AttributeKey, AttributeValue, NextAttribute,
        },
        event::{change, input, on},
        property::{prop, IntoProperty},
    },
    prelude::AddAnyAttr,
    renderer::{types::Element, RemoveEventHandler},
    view::{Position, ToTemplate},
};
#[cfg(feature = "reactive_stores")]
use reactive_graph::owner::Storage;
use reactive_graph::{
    signal::{ReadSignal, RwSignal, WriteSignal},
    traits::{Get, Update},
    wrappers::read::Signal,
};
#[cfg(feature = "reactive_stores")]
use reactive_stores::{ArcField, Field, KeyedSubfield, Subfield};
use send_wrapper::SendWrapper;
use wasm_bindgen::JsValue;

/// `group` attribute used for radio inputs with `bind`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Group;

impl AttributeKey for Group {
    const KEY: &'static str = "group";
}

/// Adds a two-way binding to the element, which adds an attribute and an event listener to the
/// element when the element is created or hydrated.
pub trait BindAttribute<Key, Sig, T>
where
    Key: AttributeKey,
    Sig: IntoSplitSignal<Value = T>,
    T: FromEventTarget + AttributeValue + 'static,
{
    /// The type of the element with the two-way binding added.
    type Output;

    /// Adds a two-way binding to the element, which adds an attribute and an event listener to the
    /// element when the element is created or hydrated.
    ///
    /// Example:
    ///
    /// ```ignore
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

impl<V, Key, Sig, T> BindAttribute<Key, Sig, T> for V
where
    V: AddAnyAttr,
    Key: AttributeKey,
    Sig: IntoSplitSignal<Value = T>,
    T: FromEventTarget + AttributeValue + PartialEq + Sync + 'static,
    Signal<BoolOrT<T>>: IntoProperty,
    <Sig as IntoSplitSignal>::Read:
        Get<Value = T> + Send + Sync + Clone + 'static,
    <Sig as IntoSplitSignal>::Write: Send + Clone + 'static,
    Element: GetValue<T>,
{
    type Output = <Self as AddAnyAttr>::Output<
        Bind<
            Key,
            T,
            <Sig as IntoSplitSignal>::Read,
            <Sig as IntoSplitSignal>::Write,
        >,
    >;

    fn bind(self, key: Key, signal: Sig) -> Self::Output {
        self.add_any_attr(bind(key, signal))
    }
}

/// Adds a two-way binding to the element, which adds an attribute and an event listener to the
/// element when the element is created or hydrated.
#[inline(always)]
pub fn bind<Key, Sig, T>(
    key: Key,
    signal: Sig,
) -> Bind<Key, T, <Sig as IntoSplitSignal>::Read, <Sig as IntoSplitSignal>::Write>
where
    Key: AttributeKey,
    Sig: IntoSplitSignal<Value = T>,
    T: FromEventTarget + AttributeValue + 'static,
    <Sig as IntoSplitSignal>::Read: Get<Value = T> + Clone + 'static,
    <Sig as IntoSplitSignal>::Write: Send + Clone + 'static,
{
    let (read_signal, write_signal) = signal.into_split_signal();

    Bind {
        key,
        read_signal,
        write_signal,
    }
}

/// Two-way binding of an attribute and an event listener
#[derive(Debug)]
pub struct Bind<Key, T, R, W>
where
    Key: AttributeKey,
    T: FromEventTarget + AttributeValue + 'static,
    R: Get<Value = T> + Clone + 'static,
    W: Update<Value = T>,
{
    key: Key,
    read_signal: R,
    write_signal: W,
}

impl<Key, T, R, W> Clone for Bind<Key, T, R, W>
where
    Key: AttributeKey,
    T: FromEventTarget + AttributeValue + 'static,
    R: Get<Value = T> + Clone + 'static,
    W: Update<Value = T> + Clone,
{
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            read_signal: self.read_signal.clone(),
            write_signal: self.write_signal.clone(),
        }
    }
}

impl<Key, T, R, W> Bind<Key, T, R, W>
where
    Key: AttributeKey,
    T: FromEventTarget + AttributeValue + PartialEq + Sync + 'static,
    R: Get<Value = T> + Clone + Send + Sync + 'static,
    W: Update<Value = T> + Clone + 'static,
    Element: ChangeEvent + GetValue<T>,
{
    /// Attaches the event listener that updates the signal value to the element.
    pub fn attach(self, el: &Element) -> RemoveEventHandler<Element> {
        el.attach_change_event::<T, W>(Key::KEY, self.write_signal.clone())
    }

    /// Creates the signal to update the value of the attribute. This signal is different
    /// when using a `"group"` attribute
    pub fn read_signal(&self, el: &Element) -> Signal<BoolOrT<T>> {
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

impl<Key, T, R, W> Attribute for Bind<Key, T, R, W>
where
    Key: AttributeKey,
    T: FromEventTarget + AttributeValue + PartialEq + Sync + 'static,
    R: Get<Value = T> + Clone + Send + Sync + 'static,
    Signal<BoolOrT<T>>: IntoProperty,
    W: Update<Value = T> + Clone + Send + 'static,
    Element: ChangeEvent + GetValue<T>,
{
    const MIN_LENGTH: usize = 0;

    type State = (
        <Signal<BoolOrT<T>> as IntoProperty>::State,
        (Element, Option<RemoveEventHandler<Element>>),
    );
    type AsyncOutput = Self;
    type Cloneable = Bind<Key, T, R, W>;
    type CloneableOwned = Bind<Key, T, R, W>;

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
    fn hydrate<const FROM_SERVER: bool>(self, el: &Element) -> Self::State {
        let signal = self.read_signal(el);
        let attr_state = prop(self.key(), signal).hydrate::<FROM_SERVER>(el);

        let cleanup = self.attach(el);

        (attr_state, (el.clone(), Some(cleanup)))
    }

    #[inline(always)]
    fn build(self, el: &Element) -> Self::State {
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

impl<Key, T, R, W> NextAttribute for Bind<Key, T, R, W>
where
    Key: AttributeKey,
    T: FromEventTarget + AttributeValue + PartialEq + Sync + 'static,
    R: Get<Value = T> + Clone + Send + Sync + 'static,
    Signal<BoolOrT<T>>: IntoProperty,
    W: Update<Value = T> + Clone + Send + 'static,
    Element: ChangeEvent + GetValue<T>,
{
    next_attr_output_type!(Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        next_attr_combine!(self, new_attr)
    }
}

impl<Key, T, R, W> ToTemplate for Bind<Key, T, R, W>
where
    Key: AttributeKey,
    T: FromEventTarget + AttributeValue + 'static,
    R: Get<Value = T> + Clone + 'static,
    W: Update<Value = T> + Clone,
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
pub trait IntoSplitSignal {
    /// The actual contained value of the signal
    type Value;
    /// The read part of the signal
    type Read: Get<Value = Self::Value>;
    /// The write part of the signal
    type Write: Update<Value = Self::Value>;
    /// Splits a combined signal into its read and write parts.
    fn into_split_signal(self) -> (Self::Read, Self::Write);
}

impl<T> IntoSplitSignal for RwSignal<T>
where
    T: Send + Sync + 'static,
    ReadSignal<T>: Get<Value = T>,
{
    type Value = T;
    type Read = ReadSignal<T>;
    type Write = WriteSignal<T>;

    fn into_split_signal(self) -> (ReadSignal<T>, WriteSignal<T>) {
        self.split()
    }
}

impl<T, R, W> IntoSplitSignal for (R, W)
where
    R: Get<Value = T>,
    W: Update<Value = T>,
{
    type Value = T;
    type Read = R;
    type Write = W;

    fn into_split_signal(self) -> (Self::Read, Self::Write) {
        self
    }
}

#[cfg(feature = "reactive_stores")]
impl<Inner, Prev, T> IntoSplitSignal for Subfield<Inner, Prev, T>
where
    Self: Get<Value = T> + Update<Value = T> + Clone,
{
    type Value = T;
    type Read = Self;
    type Write = Self;

    fn into_split_signal(self) -> (Self::Read, Self::Write) {
        (self.clone(), self.clone())
    }
}

#[cfg(feature = "reactive_stores")]
impl<T, S> IntoSplitSignal for Field<T, S>
where
    Self: Get<Value = T> + Update<Value = T> + Clone,
    S: Storage<ArcField<T>>,
{
    type Value = T;
    type Read = Self;
    type Write = Self;

    fn into_split_signal(self) -> (Self::Read, Self::Write) {
        (self, self)
    }
}

#[cfg(feature = "reactive_stores")]
impl<Inner, Prev, K, T> IntoSplitSignal for KeyedSubfield<Inner, Prev, K, T>
where
    Self: Get<Value = T> + Update<Value = T> + Clone,
    for<'a> &'a T: IntoIterator,
{
    type Value = T;
    type Read = Self;
    type Write = Self;

    fn into_split_signal(self) -> (Self::Read, Self::Write) {
        (self.clone(), self.clone())
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
    fn attach_change_event<T, W>(
        &self,
        key: &str,
        write_signal: W,
    ) -> RemoveEventHandler<Self>
    where
        T: FromEventTarget + AttributeValue + 'static,
        W: Update<Value = T> + 'static,
        Self: Sized;
}

impl ChangeEvent for web_sys::Element {
    fn attach_change_event<T, W>(
        &self,
        key: &str,
        write_signal: W,
    ) -> RemoveEventHandler<Self>
    where
        T: FromEventTarget + AttributeValue + 'static,
        W: Update<Value = T> + 'static,
    {
        if key == "group" {
            let handler = move |evt| {
                let checked = event_target_checked(&evt);
                if checked {
                    write_signal
                        .try_update(|v| *v = T::from_event_target(&evt));
                }
            };

            on::<_, _>(change, handler).attach(self)
        } else {
            let handler = move |evt| {
                write_signal.try_update(|v| *v = T::from_event_target(&evt));
            };

            if key == "checked" || self.tag_name() == "SELECT" {
                on::<_, _>(change, handler).attach(self)
            } else {
                on::<_, _>(input, handler).attach(self)
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

impl<T> IntoProperty for BoolOrT<T>
where
    T: IntoProperty<State = (Element, JsValue)>
        + Into<JsValue>
        + Clone
        + 'static,
{
    type State = (Element, JsValue);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &Element,
        key: &str,
    ) -> Self::State {
        match self.clone() {
            Self::T(s) => {
                s.hydrate::<FROM_SERVER>(el, key);
            }
            Self::Bool(b) => {
                <bool as IntoProperty>::hydrate::<FROM_SERVER>(b, el, key);
            }
        };

        (el.clone(), self.into())
    }

    fn build(self, el: &Element, key: &str) -> Self::State {
        match self.clone() {
            Self::T(s) => {
                s.build(el, key);
            }
            Self::Bool(b) => {
                <bool as IntoProperty>::build(b, el, key);
            }
        }

        (el.clone(), self.into())
    }

    fn rebuild(self, state: &mut Self::State, key: &str) {
        let (el, prev) = state;

        match self {
            Self::T(s) => s.rebuild(&mut (el.clone(), prev.clone()), key),
            Self::Bool(b) => <bool as IntoProperty>::rebuild(
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
