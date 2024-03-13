use super::{Attr, AttributeValue};
use crate::renderer::Renderer;
use std::{fmt::Debug, marker::PhantomData};

pub trait AttributeKey {
    const KEY: &'static str;
}

macro_rules! attributes {
	($($key:ident $html:literal),* $(,)?) => {
        paste::paste! {
            $(
                pub fn $key<V, Rndr>(value: V) -> Attr<[<$key:camel>], V, Rndr>
				where V: AttributeValue<Rndr>,
                  Rndr: Renderer
                {
                    Attr([<$key:camel>], value, PhantomData)
                }

				#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
				pub struct [<$key:camel>];

				impl AttributeKey for [<$key:camel>] {
					const KEY: &'static str = $html;
				}
            )*
		}
    }
}

// TODO attribute names with underscores should be kebab-cased
attributes! {
    // HTML
    abbr "abbr", // [],
    accept_charset "accept-charset", // [],
    accept "accept", // [],
    accesskey "accesskey", // [], // [GlobalAttribute]
    action "action", // [],
    align "align", // [],
    allow "allow", // [],
    allowfullscreen "allowfullscreen", // [],
    allowpaymentrequest "allowpaymentrequest", // [],
    alt "alt", // [],
    aria_atomic "aria-atomic", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_busy "aria-busy", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_controls "aria-controls", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_current "aria-current", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_describedby "aria-describedby", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_description "aria-description", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_details "aria-details", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_disabled "aria-disabled", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_dropeffect "aria-dropeffect", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_errormessage "aria-errormessage", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_flowto "aria-flowto", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_grabbed "aria-grabbed", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_haspopup "aria-haspopup", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_hidden "aria-hidden", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_invalid "aria-invalid", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_keyshortcuts "aria-keyshortcuts", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_label "aria-label", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_labelledby "aria-labelledby", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_live "aria-live", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_owns "aria-owns", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_relevant "aria-relevant", // [], // [GlobalAttribute] // [AriaAttribute],
    aria_roledescription "aria-roledescription", // [], // [GlobalAttribute] // [AriaAttribute],
    r#as "as", // [],
    r#async "async", // [],
    autocapitalize "autocapitalize", // [], // [GlobalAttribute]
    autocomplete "autocomplete", // [],
    autofocus "autofocus", // [], // [GlobalAttribute]
    autoplay "autoplay", // [],
    background "background", // [],
    bgcolor "bgcolor", // [],
    blocking "blocking", // [],
    border "border", // [],
    buffered "buffered", // [],
    capture "capture", // [],
    challenge "challenge", // [],
    charset "charset", // [],
    checked "checked", // [],
    cite "cite", // [],
    // class is handled in ../class.rs instead
    //class "class", // [],
    code "code", // [],
    color "color", // [],
    cols "cols", // [],
    colspan "colspan", // [],
    content "content", // [],
    contenteditable "contenteditable", // [], // [GlobalAttribute]
    contextmenu "contextmenu", // [], // [GlobalAttribute]
    controls "controls", // [],
    controlslist "controlslist", // [],
    coords "coords", // [],
    crossorigin "crossorigin", // [],
    csp "csp", // [],
    data "data", // [],
    datetime "datetime", // [],
    decoding "decoding", // [],
    default "default", // [],
    defer "defer", // [],
    dir "dir", // [], // [GlobalAttribute]
    dirname "dirname", // [],
    disabled "disabled", // [],
    disablepictureinpicture "disablepictureinpicture", // [],
    disableremoteplayback "disableremoteplayback", // [],
    download "download", // [],
    draggable "draggable", // [], // [GlobalAttribute]
    enctype "enctype", // [],
    enterkeyhint "enterkeyhint", // [], // [GlobalAttribute]
    exportparts "exportparts", // [], // [GlobalAttribute]
    fetchpriority "fetchprioty", // [],
    r#for "for", // [],
    form "form", // [],
    formaction "formaction", // [],
    formenctype "formenctype", // [],
    formmethod "formmethod", // [],
    formnovalidate "formnovalidate", // [],
    formtarget "formtarget", // [],
    headers "headers", // [],
    height "height", // [],
    hidden "hidden", // [], // [GlobalAttribute]
    high "high", // [],
    href "href", // [],
    hreflang "hreflang", // [],
    http_equiv "http-equiv", // [],
    icon "icon", // [],
    id "id", // [], // [GlobalAttribute]
    imagesizes "imagesizes",
    imagesrcset "imagesrcset",
    importance "importance", // [],
    inert "inert", // [], // [GlobalAttribute]
    inputmode "inputmode", // [], // [GlobalAttribute]
    integrity "integrity", // [],
    intrinsicsize "intrinsicsize", // [],
    is "is", // [], // [GlobalAttribute]
    ismap "ismap", // [],
    itemid "itemid", // [], // [GlobalAttribute]
    itemprop "itemprop", // [], // [GlobalAttribute]
    itemref "itemref", // [], // [GlobalAttribute]
    itemscope "itemscope", // [], // [GlobalAttribute]
    itemtype "itemtype", // [], // [GlobalAttribute]
    keytype "keytype", // [],
    kind "kind", // [],
    label "label", // [],
    lang "lang", // [], // [GlobalAttribute]
    language "language", // [],
    list "list", // [],
    loading "loading", // [],
    r#loop "loop", // [],
    low "low", // [],
    manifest "manifest", // [],
    max "max", // [],
    maxlength "maxlength", // [],
    media "media", // [],
    method "method", // [],
    min "min", // [],
    minlength "minlength", // [],
    multiple "multiple", // [],
    muted "muted", // [],
    name "name", // [],
    nomodule "nomodule", // [],
    nonce "nonce", // [], // [GlobalAttribute]
    novalidate "novalidate", // [],
    open "open", // [],
    optimum "optimum", // [],
    part "part", // [], // [GlobalAttribute]
    pattern "pattern", // [],
    ping "ping", // [],
    placeholder "placeholder", // [],
    playsinline "playsinline", // [],
    popover "popover", // [], // [GlobalAttribute]
    poster "poster", // [],
    preload "preload", // [],
    radiogroup "radiogroup", // [],
    readonly "readonly", // [],
    referrerpolicy "referrerpolicy", // [],
    rel "rel", // [],
    required "required", // [],
    reversed "reversed", // [],
    role "role", // [], // [GlobalAttribute] // [AriaAttribute],
    rows "rows", // [],
    rowspan "rowspan", // [],
    sandbox "sandbox", // [],
    scope "scope", // [],
    scoped "scoped", // [],
    selected "selected", // [],
    shape "shape", // [],
    size "size", // [],
    sizes "sizes", // [],
    slot "slot", // [], // [GlobalAttribute]
    span "span", // [],
    spellcheck "spellcheck", // [], // [GlobalAttribute]
    src "src", // [],
    srcdoc "srcdoc", // [],
    srclang "srclang", // [],
    srcset "srcset", // [],
    start "start", // [],
    step "step", // [],
    // style is handled in ../style.rs instead
    // style "style", // [],
    summary "summary", // [],
    tabindex "tabindex", // [], // [GlobalAttribute]
    target "target", // [],
    title "title", // [], // [GlobalAttribute]
    translate "translate", // [], // [GlobalAttribute]
    r#type "type", // [],
    usemap "usemap", // [],
    value "value", // [],
    virtualkeyboardpolicy "virtualkeyboardpolicy", // [], // [GlobalAttribute]
    width "width", // [],
    wrap "wrap", // [],
    // Event Handler Attributes
    onabort "onabort", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onautocomplete "onautocomplete", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onautocompleteerror "onautocompleteerror", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onblur "onblur", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    oncancel "oncancel", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    oncanplay "oncanplay", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    oncanplaythrough "oncanplaythrough", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onchange "onchange", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onclick "onclick", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onclose "onclose", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    oncontextmenu "oncontextmenu", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    oncuechange "oncuechange", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    ondblclick "ondblclick", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    ondrag "ondrag", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    ondragend "ondragend", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    ondragenter "ondragenter", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    ondragleave "ondragleave", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    ondragover "ondragover", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    ondragstart "ondragstart", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    ondrop "ondrop", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    ondurationchange "ondurationchange", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onemptied "onemptied", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onended "onended", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onerror "onerror", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onfocus "onfocus", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    oninput "oninput", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    oninvalid "oninvalid", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onkeydown "onkeydown", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onkeypress "onkeypress", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onkeyup "onkeyup", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onload "onload", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onloadeddata "onloadeddata", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onloadedmetadata "onloadedmetadata", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onloadstart "onloadstart", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onmousedown "onmousedown", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onmouseenter "onmouseenter", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onmouseleave "onmouseleave", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onmousemove "onmousemove", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onmouseout "onmouseout", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onmouseover "onmouseover", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onmouseup "onmouseup", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onmousewheel "onmousewheel", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onpause "onpause", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onplay "onplay", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onplaying "onplaying", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onprogress "onprogress", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onratechange "onratechange", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onreset "onreset", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onresize "onresize", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onscroll "onscroll", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onseeked "onseeked", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onseeking "onseeking", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onselect "onselect", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onshow "onshow", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onsort "onsort", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onstalled "onstalled", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onsubmit "onsubmit", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onsuspend "onsuspend", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    ontimeupdate "ontimeupdate", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    ontoggle "ontoggle", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onvolumechange "onvolumechange", // [], // [GlobalAttribute] // [EventHandlerAttribute],
    onwaiting "onwaiting", // [], // [GlobalAttribute] // [EventHandlerAttribute],

    // MathML attributes that aren't in HTML
    accent "accent",
    accentunder "accentunder",
    columnalign "columnalign",
    columnlines "columnlines",
    columnspacing "columnspacing",
    columnspan "columnspan",
    depth "depth",
    display "display",
    displaystyle "displaystyle",
    fence "fence",
    frame "frame",
    framespacing "framespacing",
    linethickness "linethickness",
    lspace "lspace",
    mathbackground "mathbackground",
    mathcolor "mathcolor",
    mathsize "mathsize",
    mathvariant "mathvariant",
    maxsize "maxsize",
    minsize "minsize",
    movablelimits "movablelimits",
    notation "notation",
    rowalign "rowalign",
    rowlines "rowlines",
    rowspacing "rowspacing",
    rspace "rspace",
    scriptlevel "scriptlevel",
    separator "separator",
    stretchy "stretchy",
    symmetric "symmetric",
    voffset "voffset",
    xmlns "xmlns",
}
