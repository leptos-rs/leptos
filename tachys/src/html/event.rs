use crate::{
    html::attribute::{
        maybe_next_attr_erasure_macros::next_attr_combine, Attribute,
    },
    renderer::{CastFrom, RemoveEventHandler, Rndr},
    view::{Position, ToTemplate},
};
use send_wrapper::SendWrapper;
use std::{
    borrow::Cow,
    cell::RefCell,
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
};
use wasm_bindgen::convert::FromWasmAbi;

/// A cloneable event callback.
pub type SharedEventCallback<E> = Rc<RefCell<dyn FnMut(E)>>;

/// A function that can be called in response to an event.
pub trait EventCallback<E>: 'static {
    /// Runs the event handler.
    fn invoke(&mut self, event: E);

    /// Converts this into a cloneable/shared event handler.
    fn into_shared(self) -> SharedEventCallback<E>;
}

impl<E: 'static> EventCallback<E> for SharedEventCallback<E> {
    fn invoke(&mut self, event: E) {
        let mut fun = self.borrow_mut();
        fun(event)
    }

    fn into_shared(self) -> SharedEventCallback<E> {
        self
    }
}

impl<F, E> EventCallback<E> for F
where
    F: FnMut(E) + 'static,
{
    fn invoke(&mut self, event: E) {
        self(event)
    }

    fn into_shared(self) -> SharedEventCallback<E> {
        Rc::new(RefCell::new(self))
    }
}

/// An event listener with a typed event target.
pub struct Targeted<E, T> {
    event: E,
    el_ty: PhantomData<T>,
}

impl<E, T> Targeted<E, T> {
    /// Returns the inner event.
    pub fn into_inner(self) -> E {
        self.event
    }

    /// Returns the event's target, as an HTML element of the correct type.
    pub fn target(&self) -> T
    where
        T: CastFrom<crate::renderer::types::Element>,

        crate::renderer::types::Event: From<E>,
        E: Clone,
    {
        let ev = crate::renderer::types::Event::from(self.event.clone());
        Rndr::event_target(&ev)
    }
}

impl<E, T> Deref for Targeted<E, T> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}

impl<E, T> DerefMut for Targeted<E, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.event
    }
}

impl<E, T> From<E> for Targeted<E, T> {
    fn from(event: E) -> Self {
        Targeted {
            event,
            el_ty: PhantomData,
        }
    }
}

/// Creates an [`Attribute`] that will add an event listener to an element.
pub fn on<E, F>(event: E, cb: F) -> On<E, F>
where
    F: FnMut(E::EventType) + 'static,
    E: EventDescriptor + Send + 'static,
    E::EventType: 'static,
    E::EventType: From<crate::renderer::types::Event>,
{
    On {
        event,
        cb: Some(SendWrapper::new(cb)),
    }
}

/// Creates an [`Attribute`] that will add an event listener with a typed target to an element.
#[allow(clippy::type_complexity)]
pub fn on_target<E, T, F>(
    event: E,
    mut cb: F,
) -> On<E, Box<dyn FnMut(E::EventType)>>
where
    T: HasElementType,
    F: FnMut(Targeted<E::EventType, <T as HasElementType>::ElementType>)
        + 'static,
    E: EventDescriptor + Send + 'static,
    E::EventType: 'static,

    E::EventType: From<crate::renderer::types::Event>,
{
    on(event, Box::new(move |ev: E::EventType| cb(ev.into())))
}

/// An [`Attribute`] that adds an event listener to an element.
pub struct On<E, F> {
    event: E,
    cb: Option<SendWrapper<F>>,
}

impl<E, F> Clone for On<E, F>
where
    E: Clone,
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            event: self.event.clone(),
            cb: self.cb.clone(),
        }
    }
}

impl<E, F> On<E, F>
where
    F: EventCallback<E::EventType>,
    E: EventDescriptor + Send + 'static,
    E::EventType: 'static,
    E::EventType: From<crate::renderer::types::Event>,
{
    /// Attaches the event listener to the element.
    pub fn attach(
        self,
        el: &crate::renderer::types::Element,
    ) -> RemoveEventHandler<crate::renderer::types::Element> {
        fn attach_inner(
            el: &crate::renderer::types::Element,
            cb: Box<dyn FnMut(crate::renderer::types::Event)>,
            name: Cow<'static, str>,
            // TODO investigate: does passing this as an option
            // (rather than, say, having a const DELEGATED: bool)
            // add to binary size?
            delegation_key: Option<Cow<'static, str>>,
        ) -> RemoveEventHandler<crate::renderer::types::Element> {
            match delegation_key {
                None => Rndr::add_event_listener(el, &name, cb),
                Some(key) => {
                    Rndr::add_event_listener_delegated(el, name, key, cb)
                }
            }
        }

        let mut cb = self.cb.expect("callback removed before attaching").take();

        #[cfg(feature = "tracing")]
        let span = tracing::Span::current();

        let cb = Box::new(move |ev: crate::renderer::types::Event| {
            #[cfg(all(debug_assertions, feature = "reactive_graph"))]
            let _rx_guard =
                reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
            #[cfg(feature = "tracing")]
            let _tracing_guard = span.enter();

            let ev = E::EventType::from(ev);
            cb.invoke(ev);
        }) as Box<dyn FnMut(crate::renderer::types::Event)>;

        attach_inner(
            el,
            cb,
            self.event.name(),
            (E::BUBBLES && cfg!(feature = "delegation"))
                .then(|| self.event.event_delegation_key()),
        )
    }

    /// Attaches the event listener to the element as a listener that is triggered during the capture phase,
    /// meaning it will fire before any event listeners further down in the DOM.
    pub fn attach_capture(
        self,
        el: &crate::renderer::types::Element,
    ) -> RemoveEventHandler<crate::renderer::types::Element> {
        fn attach_inner(
            el: &crate::renderer::types::Element,
            cb: Box<dyn FnMut(crate::renderer::types::Event)>,
            name: Cow<'static, str>,
        ) -> RemoveEventHandler<crate::renderer::types::Element> {
            Rndr::add_event_listener_use_capture(el, &name, cb)
        }

        let mut cb = self.cb.expect("callback removed before attaching").take();

        #[cfg(feature = "tracing")]
        let span = tracing::Span::current();

        let cb = Box::new(move |ev: crate::renderer::types::Event| {
            #[cfg(all(debug_assertions, feature = "reactive_graph"))]
            let _rx_guard =
                reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
            #[cfg(feature = "tracing")]
            let _tracing_guard = span.enter();

            let ev = E::EventType::from(ev);
            cb.invoke(ev);
        }) as Box<dyn FnMut(crate::renderer::types::Event)>;

        attach_inner(el, cb, self.event.name())
    }
}

impl<E, F> Debug for On<E, F>
where
    E: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("On").field(&self.event).finish()
    }
}

impl<E, F> Attribute for On<E, F>
where
    F: EventCallback<E::EventType>,
    E: EventDescriptor + Send + 'static,
    E::EventType: 'static,

    E::EventType: From<crate::renderer::types::Event>,
{
    const MIN_LENGTH: usize = 0;
    type AsyncOutput = Self;
    // a function that can be called once to remove the event listener
    type State = (
        crate::renderer::types::Element,
        Option<RemoveEventHandler<crate::renderer::types::Element>>,
    );
    type Cloneable = On<E, SharedEventCallback<E::EventType>>;
    type CloneableOwned = On<E, SharedEventCallback<E::EventType>>;

    #[inline(always)]
    fn html_len(&self) -> usize {
        0
    }

    #[inline(always)]
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
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let cleanup = if E::CAPTURE {
            self.attach_capture(el)
        } else {
            self.attach(el)
        };
        (el.clone(), Some(cleanup))
    }

    #[inline(always)]
    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let cleanup = if E::CAPTURE {
            self.attach_capture(el)
        } else {
            self.attach(el)
        };
        (el.clone(), Some(cleanup))
    }

    #[inline(always)]
    fn rebuild(self, state: &mut Self::State) {
        let (el, prev_cleanup) = state;
        if let Some(prev) = prev_cleanup.take() {
            (prev.into_inner())(el);
        }
        *prev_cleanup = Some(if E::CAPTURE {
            self.attach_capture(el)
        } else {
            self.attach(el)
        });
    }

    fn into_cloneable(self) -> Self::Cloneable {
        On {
            cb: self.cb.map(|cb| SendWrapper::new(cb.take().into_shared())),
            event: self.event,
        }
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        On {
            cb: self.cb.map(|cb| SendWrapper::new(cb.take().into_shared())),
            event: self.event,
        }
    }

    fn dry_resolve(&mut self) {
        // dry_resolve() only runs during SSR, and we should use it to
        // synchronously remove and drop the SendWrapper value
        // we don't need this value during SSR and leaving it here could drop it
        // from a different thread
        self.cb.take();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl<E, F> NextAttribute for On<E, F>
where
    F: EventCallback<E::EventType>,
    E: EventDescriptor + Send + 'static,
    E::EventType: 'static,

    E::EventType: From<crate::renderer::types::Event>,
{
    next_attr_output_type!(Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        next_attr_combine!(self, new_attr)
    }
}

impl<E, F> ToTemplate for On<E, F> {
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

/// A trait for converting types into [web_sys events](web_sys).
pub trait EventDescriptor: Clone {
    /// The [`web_sys`] event type, such as [`web_sys::MouseEvent`].
    type EventType: FromWasmAbi;

    /// Indicates if this event bubbles. For example, `click` bubbles,
    /// but `focus` does not.
    ///
    /// If this is true, then the event will be delegated globally if the `delegation`
    /// feature is enabled. Otherwise, event listeners will be directly attached to the element.
    const BUBBLES: bool;

    /// Indicates if this event should be handled during the capture phase.
    const CAPTURE: bool = false;

    /// The name of the event, such as `click` or `mouseover`.
    fn name(&self) -> Cow<'static, str>;

    /// The key used for event delegation.
    fn event_delegation_key(&self) -> Cow<'static, str>;

    /// Return the options for this type. This is only used when you create a [`Custom`] event
    /// handler.
    #[inline(always)]
    fn options(&self) -> Option<&web_sys::AddEventListenerOptions> {
        None
    }
}

/// A wrapper that tells the framework to handle an event during the capture phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture<E> {
    inner: E,
}

/// Wraps an event to indicate that it should be handled during the capture phase.
pub fn capture<E>(event: E) -> Capture<E> {
    Capture { inner: event }
}

impl<E: EventDescriptor> EventDescriptor for Capture<E> {
    type EventType = E::EventType;

    const CAPTURE: bool = true;
    const BUBBLES: bool = E::BUBBLES;

    fn name(&self) -> Cow<'static, str> {
        self.inner.name()
    }

    fn event_delegation_key(&self) -> Cow<'static, str> {
        self.inner.event_delegation_key()
    }
}

/// A custom event.
#[derive(Debug)]
pub struct Custom<E: FromWasmAbi = web_sys::Event> {
    name: Cow<'static, str>,
    options: Option<SendWrapper<web_sys::AddEventListenerOptions>>,
    _event_type: PhantomData<fn() -> E>,
}

impl<E: FromWasmAbi> Clone for Custom<E> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            options: self.options.clone(),
            _event_type: PhantomData,
        }
    }
}

impl<E: FromWasmAbi> EventDescriptor for Custom<E> {
    type EventType = E;

    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn event_delegation_key(&self) -> Cow<'static, str> {
        format!("$$${}", self.name).into()
    }

    const BUBBLES: bool = false;

    #[inline(always)]
    fn options(&self) -> Option<&web_sys::AddEventListenerOptions> {
        self.options.as_deref()
    }
}

impl<E: FromWasmAbi> Custom<E> {
    /// Creates a custom event type that can be used within
    /// [`OnAttribute::on`](crate::prelude::OnAttribute::on), for events
    /// which are not covered in the [`ev`](crate::html::event) module.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            options: None,
            _event_type: PhantomData,
        }
    }

    /// Modify the [`AddEventListenerOptions`] used for this event listener.
    ///
    /// ```rust
    /// # use tachys::prelude::*;
    /// # use tachys::html;
    /// # use tachys::html::event as ev;
    /// # fn custom_event() -> impl Render {
    /// let mut non_passive_wheel = ev::Custom::new("wheel");
    /// non_passive_wheel.options_mut().set_passive(false);
    ///
    /// let canvas =
    ///     html::element::canvas().on(non_passive_wheel, |e: ev::WheelEvent| {
    ///         // handle event
    ///     });
    /// # canvas
    /// # }
    /// ```
    ///
    /// [`AddEventListenerOptions`]: web_sys::AddEventListenerOptions
    pub fn options_mut(&mut self) -> &mut web_sys::AddEventListenerOptions {
        // It is valid to construct a `SendWrapper` here because
        // its inner data will only be accessed in the browser's main thread.
        self.options.get_or_insert_with(|| {
            SendWrapper::new(web_sys::AddEventListenerOptions::new())
        })
    }
}

macro_rules! generate_event_types {
  {$(
    $( #[$does_not_bubble:ident] )?
    $( $event:ident )+ : $web_event:ident
  ),* $(,)?} => {
    ::paste::paste! {
      $(
        #[doc = "The `" [< $($event)+ >] "` event, which receives [" $web_event "](web_sys::" $web_event ") as its argument."]
        #[derive(Copy, Clone, Debug)]
        #[allow(non_camel_case_types)]
        pub struct [<$( $event )+ >];

        impl EventDescriptor for [< $($event)+ >] {
          type EventType = web_sys::$web_event;

          #[inline(always)]
          fn name(&self) -> Cow<'static, str> {
            stringify!([< $($event)+ >]).into()
          }

          #[inline(always)]
          fn event_delegation_key(&self) -> Cow<'static, str> {
            concat!("$$$", stringify!([< $($event)+ >])).into()
          }

          const BUBBLES: bool = true $(&& generate_event_types!($does_not_bubble))?;
        }
      )*
    }
  };

  (does_not_bubble) => { false }
}

generate_event_types! {
  // =========================================================
  // WindowEventHandlersEventMap
  // =========================================================
  #[does_not_bubble]
  after print: Event,
  #[does_not_bubble]
  before print: Event,
  #[does_not_bubble]
  before unload: BeforeUnloadEvent,
  #[does_not_bubble]
  gamepad connected: GamepadEvent,
  #[does_not_bubble]
  gamepad disconnected: GamepadEvent,
  hash change: HashChangeEvent,
  #[does_not_bubble]
  language change: Event,
  #[does_not_bubble]
  message: MessageEvent,
  #[does_not_bubble]
  message error: MessageEvent,
  #[does_not_bubble]
  offline: Event,
  #[does_not_bubble]
  online: Event,
  #[does_not_bubble]
  page hide: PageTransitionEvent,
  #[does_not_bubble]
  page show: PageTransitionEvent,
  pop state: PopStateEvent,
  rejection handled: PromiseRejectionEvent,
  #[does_not_bubble]
  storage: StorageEvent,
  #[does_not_bubble]
  unhandled rejection: PromiseRejectionEvent,
  #[does_not_bubble]
  unload: Event,

  // =========================================================
  // GlobalEventHandlersEventMap
  // =========================================================
  #[does_not_bubble]
  abort: UiEvent,
  animation cancel: AnimationEvent,
  animation end: AnimationEvent,
  animation iteration: AnimationEvent,
  animation start: AnimationEvent,
  aux click: MouseEvent,
  before input: InputEvent,
  before toggle: Event, // web_sys does not include `ToggleEvent`
  #[does_not_bubble]
  blur: FocusEvent,
  #[does_not_bubble]
  can play: Event,
  #[does_not_bubble]
  can play through: Event,
  change: Event,
  click: MouseEvent,
  #[does_not_bubble]
  close: Event,
  composition end: CompositionEvent,
  composition start: CompositionEvent,
  composition update: CompositionEvent,
  context menu: MouseEvent,
  #[does_not_bubble]
  cue change: Event,
  dbl click: MouseEvent,
  drag: DragEvent,
  drag end: DragEvent,
  drag enter: DragEvent,
  drag leave: DragEvent,
  drag over: DragEvent,
  drag start: DragEvent,
  drop: DragEvent,
  #[does_not_bubble]
  duration change: Event,
  #[does_not_bubble]
  emptied: Event,
  #[does_not_bubble]
  ended: Event,
  #[does_not_bubble]
  error: ErrorEvent,
  #[does_not_bubble]
  focus: FocusEvent,
  #[does_not_bubble]
  focus in: FocusEvent,
  #[does_not_bubble]
  focus out: FocusEvent,
  form data: Event, // web_sys does not include `FormDataEvent`
  #[does_not_bubble]
  got pointer capture: PointerEvent,
  input: Event,
  #[does_not_bubble]
  invalid: Event,
  key down: KeyboardEvent,
  key press: KeyboardEvent,
  key up: KeyboardEvent,
  #[does_not_bubble]
  load: Event,
  #[does_not_bubble]
  loaded data: Event,
  #[does_not_bubble]
  loaded metadata: Event,
  #[does_not_bubble]
  load start: Event,
  lost pointer capture: PointerEvent,
  mouse down: MouseEvent,
  #[does_not_bubble]
  mouse enter: MouseEvent,
  #[does_not_bubble]
  mouse leave: MouseEvent,
  mouse move: MouseEvent,
  mouse out: MouseEvent,
  mouse over: MouseEvent,
  mouse up: MouseEvent,
  #[does_not_bubble]
  pause: Event,
  #[does_not_bubble]
  play: Event,
  #[does_not_bubble]
  playing: Event,
  pointer cancel: PointerEvent,
  pointer down: PointerEvent,
  #[does_not_bubble]
  pointer enter: PointerEvent,
  #[does_not_bubble]
  pointer leave: PointerEvent,
  pointer move: PointerEvent,
  pointer out: PointerEvent,
  pointer over: PointerEvent,
  pointer up: PointerEvent,
  #[does_not_bubble]
  progress: ProgressEvent,
  #[does_not_bubble]
  rate change: Event,
  reset: Event,
  #[does_not_bubble]
  resize: UiEvent,
  #[does_not_bubble]
  scroll: Event,
  #[does_not_bubble]
  scroll end: Event,
  security policy violation: SecurityPolicyViolationEvent,
  #[does_not_bubble]
  seeked: Event,
  #[does_not_bubble]
  seeking: Event,
  select: Event,
  #[does_not_bubble]
  selection change: Event,
  select start: Event,
  slot change: Event,
  #[does_not_bubble]
  stalled: Event,
  submit: SubmitEvent,
  #[does_not_bubble]
  suspend: Event,
  #[does_not_bubble]
  time update: Event,
  #[does_not_bubble]
  toggle: Event,
  touch cancel: TouchEvent,
  touch end: TouchEvent,
  touch move: TouchEvent,
  touch start: TouchEvent,
  transition cancel: TransitionEvent,
  transition end: TransitionEvent,
  transition run: TransitionEvent,
  transition start: TransitionEvent,
  #[does_not_bubble]
  volume change: Event,
  #[does_not_bubble]
  waiting: Event,
  webkit animation end: Event,
  webkit animation iteration: Event,
  webkit animation start: Event,
  webkit transition end: Event,
  wheel: WheelEvent,

  // =========================================================
  // WindowEventMap
  // =========================================================
  D O M Content Loaded: Event, // Hack for correct casing
  #[does_not_bubble]
  device motion: DeviceMotionEvent,
  #[does_not_bubble]
  device orientation: DeviceOrientationEvent,
  #[does_not_bubble]
  orientation change: Event,

  // =========================================================
  // DocumentAndElementEventHandlersEventMap
  // =========================================================
  copy: Event, // ClipboardEvent is unstable
  cut: Event, // ClipboardEvent is unstable
  paste: Event, // ClipboardEvent is unstable

  // =========================================================
  // DocumentEventMap
  // =========================================================
  fullscreen change: Event,
  fullscreen error: Event,
  pointer lock change: Event,
  pointer lock error: Event,
  #[does_not_bubble]
  ready state change: Event,
  visibility change: Event,
}

// Export `web_sys` event types
use super::{
    attribute::{
        maybe_next_attr_erasure_macros::next_attr_output_type, NextAttribute,
    },
    element::HasElementType,
};
#[doc(no_inline)]
pub use web_sys::{
    AnimationEvent, BeforeUnloadEvent, CompositionEvent, CustomEvent,
    DeviceMotionEvent, DeviceOrientationEvent, DragEvent, ErrorEvent, Event,
    FocusEvent, GamepadEvent, HashChangeEvent, InputEvent, KeyboardEvent,
    MessageEvent, MouseEvent, PageTransitionEvent, PointerEvent, PopStateEvent,
    ProgressEvent, PromiseRejectionEvent, SecurityPolicyViolationEvent,
    StorageEvent, SubmitEvent, TouchEvent, TransitionEvent, UiEvent,
    WheelEvent,
};
