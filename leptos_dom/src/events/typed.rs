//! Types for all DOM events.

use std::{borrow::Cow, marker::PhantomData};
use wasm_bindgen::convert::FromWasmAbi;

/// A trait for converting types into [web_sys events](web_sys).
pub trait EventDescriptor: Clone {
    /// The [`web_sys`] event type, such as [`web_sys::MouseEvent`].
    type EventType: FromWasmAbi;

    /// Indicates if this event bubbles. For example, `click` bubbles,
    /// but `focus` does not.
    ///
    /// If this is true, then the event will be delegated globally,
    /// otherwise, event listeners will be directly attached to the element.
    const BUBBLES: bool;

    /// The name of the event, such as `click` or `mouseover`.
    fn name(&self) -> Cow<'static, str>;

    /// The key used for event delegation.
    fn event_delegation_key(&self) -> Cow<'static, str>;

    /// Return the options for this type. This is only used when you create a [`Custom`] event
    /// handler.
    #[inline(always)]
    fn options(&self) -> &Option<web_sys::AddEventListenerOptions> {
        &None
    }
}

/// Overrides the [`EventDescriptor::BUBBLES`] value to always return
/// `false`, which forces the event to not be globally delegated.
#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
pub struct undelegated<Ev: EventDescriptor>(pub Ev);

impl<Ev: EventDescriptor> EventDescriptor for undelegated<Ev> {
    type EventType = Ev::EventType;

    #[inline(always)]
    fn name(&self) -> Cow<'static, str> {
        self.0.name()
    }

    #[inline(always)]
    fn event_delegation_key(&self) -> Cow<'static, str> {
        self.0.event_delegation_key()
    }

    const BUBBLES: bool = false;
}

/// A custom event.
#[derive(Debug)]
pub struct Custom<E: FromWasmAbi = web_sys::Event> {
    name: Cow<'static, str>,
    options: Option<web_sys::AddEventListenerOptions>,
    _event_type: PhantomData<E>,
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
    fn options(&self) -> &Option<web_sys::AddEventListenerOptions> {
        &self.options
    }
}

impl<E: FromWasmAbi> Custom<E> {
    /// Creates a custom event type that can be used within
    /// [`HtmlElement::on`](crate::HtmlElement::on), for events
    /// which are not covered in the [`ev`](crate::ev) module.
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
    /// # use leptos::*;
    /// # run_scope(create_runtime(), |cx| {
    /// # let canvas_ref: NodeRef<html::Canvas> = create_node_ref(cx);
    /// let mut non_passive_wheel = ev::Custom::<ev::WheelEvent>::new("wheel");
    /// # if false {
    /// let options = non_passive_wheel.options_mut();
    /// options.passive(false);
    /// # }
    /// canvas_ref.on_load(cx, move |canvas: HtmlElement<html::Canvas>| {
    ///     canvas.on(non_passive_wheel, move |_event| {
    ///         // Handle _event
    ///     });
    /// });
    /// # });
    /// ```
    ///
    /// [`AddEventListenerOptions`]: web_sys::AddEventListenerOptions
    pub fn options_mut(&mut self) -> &mut web_sys::AddEventListenerOptions {
        self.options
            .get_or_insert_with(web_sys::AddEventListenerOptions::new)
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

      /// An enum holding all basic event types with their respective handlers.
      ///
      /// It currently omits [`Custom`] and [`undelegated`] variants.
      #[non_exhaustive]
      pub enum EventHandler {
        $(
          #[doc = "Variant mapping [`struct@" [< $($event)+ >] "`] to its event handler type."]
          [< $($event:camel)+ >]([< $($event)+ >], Box<dyn FnMut($web_event) + 'static>),
        )*
      }

      impl ::std::fmt::Debug for EventHandler {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
          match self {
            $(
              Self::[< $($event:camel)+ >](event, _) => f
                .debug_tuple(stringify!([< $($event:camel)+ >]))
                .field(&event)
                .field(&::std::any::type_name::<Box<dyn FnMut($web_event) + 'static>>())
                .finish(),
            )*
          }
        }
      }

      impl crate::IntoEventHandler for EventHandler {
        #[inline(always)]
        fn into_event_handler(self) -> EventHandler {
          self
        }
      }

      $(
        impl<T> crate::IntoEventHandler for ([< $($event)+ >], T)
        where
          T: FnMut($web_event) + 'static
        {
          #[inline]
          fn into_event_handler(self) -> EventHandler {
            EventHandler::[< $($event:camel)+ >](self.0, Box::new(self.1))
          }
        }
        // TODO: figure out if this is even desirable, could be bit confusing
        impl<T> crate::IntoEventHandler for (T, [< $($event)+ >])
        where
          T: FnMut($web_event) + 'static
        {
          #[inline]
          fn into_event_handler(self) -> EventHandler {
            EventHandler::[< $($event:camel)+ >](self.1, Box::new(self.0))
          }
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
pub use web_sys::{
    AnimationEvent, BeforeUnloadEvent, CompositionEvent, CustomEvent,
    DeviceMotionEvent, DeviceOrientationEvent, DragEvent, ErrorEvent, Event,
    FocusEvent, GamepadEvent, HashChangeEvent, InputEvent, KeyboardEvent,
    MessageEvent, MouseEvent, PageTransitionEvent, PointerEvent, PopStateEvent,
    ProgressEvent, PromiseRejectionEvent, SecurityPolicyViolationEvent,
    StorageEvent, SubmitEvent, TouchEvent, TransitionEvent, UiEvent,
    WheelEvent,
};
