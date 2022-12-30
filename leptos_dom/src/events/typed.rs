//! Collection of typed events.

use std::{borrow::Cow, marker::PhantomData};
use wasm_bindgen::convert::FromWasmAbi;

/// A trait for converting types into [web_sys events](web_sys).
pub trait EventDescriptor: Clone {
  /// The [`web_sys`] event type, such as [`web_sys::MouseEvent`].
  type EventType: FromWasmAbi;

  /// The name of the event, such as `click` or `mouseover`.
  fn name(&self) -> Cow<'static, str>;

  /// Indicates if this event bubbles. For example, `click` bubbles,
  /// but `focus` does not.
  ///
  /// If this method returns true, then the event will be delegated globally,
  /// otherwise, event listeners will be directly attached to the element.
  fn bubbles(&self) -> bool {
    true
  }
}

/// Overrides the [`EventDescriptor::bubbles`] method to always return
/// `false`, which forces the event to not be globally delegated.
#[derive(Clone)]
#[allow(non_camel_case_types)]
pub struct undelegated<Ev: EventDescriptor>(pub Ev);

impl<Ev: EventDescriptor> EventDescriptor for undelegated<Ev> {
  type EventType = Ev::EventType;

  fn name(&self) -> Cow<'static, str> {
    self.0.name()
  }

  fn bubbles(&self) -> bool {
    false
  }
}

/// A custom event.
pub struct Custom<E: FromWasmAbi = web_sys::Event> {
  name: Cow<'static, str>,
  _event_type: PhantomData<E>,
}

impl<E: FromWasmAbi> Clone for Custom<E> {
  fn clone(&self) -> Self {
    Self {
      name: self.name.clone(),
      _event_type: PhantomData,
    }
  }
}

impl<E: FromWasmAbi> EventDescriptor for Custom<E> {
  type EventType = E;

  fn name(&self) -> Cow<'static, str> {
    self.name.clone()
  }

  fn bubbles(&self) -> bool {
    false
  }
}

impl<E: FromWasmAbi> Custom<E> {
  /// Creates a custom event type that can be used within
  /// [`HtmlElement::on`](crate::HtmlElement::on), for events
  /// which are not covered in the [`ev`](crate::ev) module.
  pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
    Self {
      name: name.into(),
      _event_type: PhantomData,
    }
  }
}

macro_rules! generate_event_types {
  {$(
    $( #[$does_not_bubble:ident] )?
    $event:ident : $web_sys_event:ident
  ),* $(,)?} => {

    $(
      #[doc = "The "]
      #[doc = stringify!($event)]
      #[doc = " event."]
      #[allow(non_camel_case_types)]
      #[derive(Clone, Copy)]
      pub struct $event;

      impl EventDescriptor for $event {
        type EventType = web_sys::$web_sys_event;

        fn name(&self) -> Cow<'static, str> {
          stringify!($event).into()
        }

        $(
          generate_event_types!($does_not_bubble);
        )?
      }
    )*
  };

  (does_not_bubble) => {
    fn bubbles(&self) -> bool { false }
  }
}

generate_event_types! {
  // =========================================================
  // WindowEventHandlersEventMap
  // =========================================================
  afterprint: Event,
  beforeprint: Event,
  beforeunload: BeforeUnloadEvent,
  gamepadconnected: GamepadEvent,
  gamepaddisconnected: GamepadEvent,
  hashchange: HashChangeEvent,
  languagechange: Event,
  message: MessageEvent,
  messageerror: MessageEvent,
  offline: Event,
  online: Event,
  pagehide: PageTransitionEvent,
  pageshow: PageTransitionEvent,
  popstate: PopStateEvent,
  rejectionhandled: PromiseRejectionEvent,
  storage: StorageEvent,
  unhandledrejection: PromiseRejectionEvent,
  #[does_not_bubble]
  unload: Event,

  // =========================================================
  // GlobalEventHandlersEventMap
  // =========================================================
  #[does_not_bubble]
  abort: UiEvent,
  animationcancel: AnimationEvent,
  animationend: AnimationEvent,
  animationiteration: AnimationEvent,
  animationstart: AnimationEvent,
  auxclick: MouseEvent,
  beforeinput: InputEvent,
  #[does_not_bubble]
  blur: FocusEvent,
  canplay: Event,
  canplaythrough: Event,
  change: Event,
  click: MouseEvent,
  close: Event,
  compositionend: CompositionEvent,
  compositionstart: CompositionEvent,
  compositionupdate: CompositionEvent,
  contextmenu: MouseEvent,
  cuechange: Event,
  dblclick: MouseEvent,
  drag: DragEvent,
  dragend: DragEvent,
  dragenter: DragEvent,
  dragleave: DragEvent,
  dragover: DragEvent,
  dragstart: DragEvent,
  drop: DragEvent,
  durationchange: Event,
  emptied: Event,
  ended: Event,
  #[does_not_bubble]
  error: ErrorEvent,
  #[does_not_bubble]
  focus: FocusEvent,
  #[does_not_bubble]
  focusin: FocusEvent,
  #[does_not_bubble]
  focusout: FocusEvent,
  formdata: Event, // web_sys does not include `FormDataEvent`
  gotpointercapture: PointerEvent,
  input: Event,
  invalid: Event,
  keydown: KeyboardEvent,
  keypress: KeyboardEvent,
  keyup: KeyboardEvent,
  #[does_not_bubble]
  load: Event,
  loadeddata: Event,
  loadedmetadata: Event,
  #[does_not_bubble]
  loadstart: Event,
  lostpointercapture: PointerEvent,
  mousedown: MouseEvent,
  mouseenter: MouseEvent,
  mouseleave: MouseEvent,
  mousemove: MouseEvent,
  mouseout: MouseEvent,
  mouseover: MouseEvent,
  mouseup: MouseEvent,
  pause: Event,
  play: Event,
  playing: Event,
  pointercancel: PointerEvent,
  pointerdown: PointerEvent,
  pointerenter: PointerEvent,
  pointerleave: PointerEvent,
  pointermove: PointerEvent,
  pointerout: PointerEvent,
  pointerover: PointerEvent,
  pointerup: PointerEvent,
  #[does_not_bubble]
  progress: ProgressEvent,
  ratechange: Event,
  reset: Event,
  resize: UiEvent,
  #[does_not_bubble]
  scroll: Event,
  securitypolicyviolation: SecurityPolicyViolationEvent,
  seeked: Event,
  seeking: Event,
  select: Event,
  selectionchange: Event,
  selectstart: Event,
  slotchange: Event,
  stalled: Event,
  submit: SubmitEvent,
  suspend: Event,
  timeupdate: Event,
  toggle: Event,
  touchcancel: TouchEvent,
  touchend: TouchEvent,
  touchmove: TouchEvent,
  touchstart: TouchEvent,
  transitioncancel: TransitionEvent,
  transitionend: TransitionEvent,
  transitionrun: TransitionEvent,
  transitionstart: TransitionEvent,
  volumechange: Event,
  waiting: Event,
  webkitanimationend: Event,
  webkitanimationiteration: Event,
  webkitanimationstart: Event,
  webkittransitionend: Event,
  wheel: WheelEvent,

  // =========================================================
  // WindowEventMap
  // =========================================================
  DOMContentLoaded: Event,
  devicemotion: DeviceMotionEvent,
  deviceorientation: DeviceOrientationEvent,
  orientationchange: Event,

  // =========================================================
  // DocumentAndElementEventHandlersEventMap
  // =========================================================
  copy: Event, // ClipboardEvent is unstable
  cut: Event, // ClipboardEvent is unstable
  paste: Event, // ClipboardEvent is unstable

  // =========================================================
  // DocumentEventMap
  // =========================================================
  fullscreenchange: Event,
  fullscreenerror: Event,
  pointerlockchange: Event,
  pointerlockerror: Event,
  readystatechange: Event,
  visibilitychange: Event,
}
