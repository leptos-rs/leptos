use crate::{
    html::attribute::Attribute,
    renderer::{CastFrom, DomRenderer},
    view::{Position, ToTemplate},
};
use std::{
    borrow::Cow,
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
use wasm_bindgen::convert::FromWasmAbi;

pub struct Targeted<E, T, R> {
    event: E,
    el_ty: PhantomData<T>,
    rndr: PhantomData<R>,
}

impl<E, T, R> Targeted<E, T, R> {
    pub fn into_inner(self) -> E {
        self.event
    }

    pub fn target(&self) -> T
    where
        T: CastFrom<R::Element>,
        R: DomRenderer,
        R::Event: From<E>,
        E: Clone,
    {
        let ev = R::Event::from(self.event.clone());
        R::event_target(&ev)
    }
}

impl<E, T, R> Deref for Targeted<E, T, R> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}

impl<E, T, R> DerefMut for Targeted<E, T, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.event
    }
}

impl<E, T, R> From<E> for Targeted<E, T, R> {
    fn from(event: E) -> Self {
        Targeted {
            event,
            el_ty: PhantomData,
            rndr: PhantomData,
        }
    }
}

pub fn on<E, R>(event: E, cb: impl FnMut(E::EventType) + 'static) -> On<R>
where
    E: EventDescriptor + Send + 'static,
    E::EventType: 'static,
    R: DomRenderer,
    E::EventType: From<R::Event>,
{
    let mut cb = send_wrapper::SendWrapper::new(cb);
    On {
        name: event.name(),
        setup: Box::new(move |el| {
            let cb = Box::new(move |ev: R::Event| {
                let ev = E::EventType::from(ev);
                cb(ev);
            }) as Box<dyn FnMut(R::Event) + Send>;

            if E::BUBBLES && cfg!(feature = "delegation") {
                R::add_event_listener_delegated(
                    el,
                    event.name(),
                    event.event_delegation_key(),
                    cb,
                )
            } else {
                R::add_event_listener(el, &event.name(), cb)
            }
        }),
        ty: PhantomData,
    }
}

pub fn on_target<E, T, R>(
    event: E,
    mut cb: impl FnMut(Targeted<E::EventType, T, R>) + 'static,
) -> On<R>
where
    E: EventDescriptor + Send + 'static,
    E::EventType: 'static,
    R: DomRenderer,
    E::EventType: From<R::Event>,
{
    on(event, move |ev| cb(ev.into()))
}

pub struct On<R: DomRenderer> {
    name: Cow<'static, str>,
    #[allow(clippy::type_complexity)]
    setup: Box<dyn FnOnce(&R::Element) -> Box<dyn FnOnce(&R::Element)> + Send>,
    ty: PhantomData<R>,
}

impl<R> Debug for On<R>
where
    R: DomRenderer,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("On").field(&self.name).finish()
    }
}

impl<R> Attribute<R> for On<R>
where
    R: DomRenderer,
{
    const MIN_LENGTH: usize = 0;
    // a function that can be called once to remove the event listener
    type State = (R::Element, Option<Box<dyn FnOnce(&R::Element)>>);

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
    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        let cleanup = (self.setup)(el);
        (el.clone(), Some(cleanup))
    }

    #[inline(always)]
    fn build(self, el: &R::Element) -> Self::State {
        let cleanup = (self.setup)(el);
        (el.clone(), Some(cleanup))
    }

    #[inline(always)]
    fn rebuild(self, state: &mut Self::State) {
        let (el, prev_cleanup) = state;
        if let Some(prev) = prev_cleanup.take() {
            prev(el);
        }
        *prev_cleanup = Some((self.setup)(el));
    }
}

impl<R> NextAttribute<R> for On<R>
where
    R: DomRenderer,
{
    type Output<NewAttr: Attribute<R>> = (Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute<R>>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        (self, new_attr)
    }
}

impl<R> ToTemplate for On<R>
where
    R: DomRenderer,
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
use super::attribute::NextAttribute;
pub use web_sys::{
    AnimationEvent, BeforeUnloadEvent, CompositionEvent, CustomEvent,
    DeviceMotionEvent, DeviceOrientationEvent, DragEvent, ErrorEvent, Event,
    FocusEvent, GamepadEvent, HashChangeEvent, InputEvent, KeyboardEvent,
    MessageEvent, MouseEvent, PageTransitionEvent, PointerEvent, PopStateEvent,
    ProgressEvent, PromiseRejectionEvent, SecurityPolicyViolationEvent,
    StorageEvent, SubmitEvent, TouchEvent, TransitionEvent, UiEvent,
    WheelEvent,
};
