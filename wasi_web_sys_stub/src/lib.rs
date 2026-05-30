macro_rules! dummy_types {
    ($($id:ident),* $(,)?) => {
        $(
            #[derive(Clone, Default)]
            pub struct $id;
            impl wasm_bindgen::JsCast for $id {}
            impl From<wasm_bindgen::JsValue> for $id {
                fn from(_: wasm_bindgen::JsValue) -> Self {
                    Self
                }
            }
            impl AsRef<wasm_bindgen::JsValue> for $id {
                fn as_ref(&self) -> &wasm_bindgen::JsValue {
                    static VAL: wasm_bindgen::JsValue = wasm_bindgen::JsValue;
                    &VAL
                }
            }
        )*
    };
}


dummy_types![
    Element,
    SvgElement,
    Event,
    Comment,
    Text,
    Node,
    Document,
    Window,
    HtmlTemplateElement,
    DocumentFragment,
    ClassList,
    CssStyleDeclaration,
    HtmlElement,
    HtmlInputElement,
    HtmlFormElement,
    HtmlButtonElement,
    SubmitEvent,
    ShadowRoot,
    ShadowRootInit,
    ShadowRootMode,
    HtmlCollection,
    DomStringMap,
    DomTokenList,
    // Events
    AnimationEvent,
    BeforeUnloadEvent,
    ClipboardEvent,
    CompositionEvent,
    CustomEvent,
    DeviceMotionEvent,
    DeviceOrientationEvent,
    DragEvent,
    ErrorEvent,
    FocusEvent,
    GamepadEvent,
    HashChangeEvent,
    InputEvent,
    KeyboardEvent,
    MessageEvent,
    MouseEvent,
    PageTransitionEvent,
    PointerEvent,
    PopStateEvent,
    ProgressEvent,
    PromiseRejectionEvent,
    SecurityPolicyViolationEvent,
    StorageEvent,
    TouchEvent,
    TransitionEvent,
    UiEvent,
    WheelEvent,
    // HTML Element Types
    HtmlHtmlElement,
    HtmlBaseElement,
    HtmlHeadElement,
    HtmlLinkElement,
    HtmlMetaElement,
    HtmlStyleElement,
    HtmlTitleElement,
    HtmlBodyElement,
    HtmlHeadingElement,
    HtmlQuoteElement,
    HtmlDivElement,
    HtmlDListElement,
    HtmlHrElement,
    HtmlLiElement,
    HtmlOListElement,
    HtmlParagraphElement,
    HtmlPreElement,
    HtmlUListElement,
    HtmlAnchorElement,
    HtmlBrElement,
    HtmlDataElement,
    HtmlSpanElement,
    HtmlTimeElement,
    HtmlAreaElement,
    HtmlAudioElement,
    HtmlImageElement,
    HtmlMapElement,
    HtmlTrackElement,
    HtmlVideoElement,
    HtmlEmbedElement,
    HtmlIFrameElement,
    HtmlObjectElement,
    HtmlParamElement,
    HtmlPictureElement,
    HtmlSourceElement,
    HtmlCanvasElement,
    HtmlScriptElement,
    HtmlModElement,
    HtmlTableCaptionElement,
    HtmlTableColElement,
    HtmlTableElement,
    HtmlTableSectionElement,
    HtmlTableCellElement,
    HtmlTableRowElement,
    HtmlDataListElement,
    HtmlFieldSetElement,
    HtmlLabelElement,
    HtmlLegendElement,
    HtmlMeterElement,
    HtmlOptGroupElement,
    HtmlOutputElement,
    HtmlProgressElement,
    HtmlSelectElement,
    HtmlTextAreaElement,
    HtmlDetailsElement,
    HtmlDialogElement,
    HtmlMenuElement,
    HtmlSlotElement,
    HtmlOptionElement,
];

#[derive(Debug, Clone, Default)]
pub struct AddEventListenerOptions;
impl AddEventListenerOptions {
    pub fn new() -> Self {
        Self
    }
}
impl wasm_bindgen::JsCast for AddEventListenerOptions {}

impl AsRef<Node> for Element {
    fn as_ref(&self) -> &Node {
        static NODE: Node = Node;
        &NODE
    }
}

impl AsRef<Node> for SvgElement {
    fn as_ref(&self) -> &Node {
        static NODE: Node = Node;
        &NODE
    }
}
