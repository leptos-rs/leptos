use crate::{attribute_value, Mode};
use convert_case::{Case::Snake, Casing};
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned, ToTokens};
use rstml::node::{KeyedAttribute, Node, NodeElement, NodeName};
use syn::{
    spanned::Spanned,
    Expr::{self, Tuple},
    ExprArray, ExprLit, ExprPath, Lit,
};

pub mod client_builder;
pub mod client_template;
pub mod component_builder;
pub mod ide_helper;
pub mod server_template;
pub mod slot_helper;
#[cfg(test)]
mod tests;

pub(crate) use ide_helper::*;

pub(crate) fn render_view(
    nodes: &[Node],
    mode: Mode,
    global_class: Option<&TokenTree>,
    call_site: Option<String>,
) -> TokenStream {
    let empty = {
        let span = Span::call_site();
        quote_spanned! {
            span=> ::leptos::leptos_dom::Unit
        }
    };

    if mode == Mode::Ssr {
        match nodes.len() {
            0 => empty,
            1 => server_template::root_node_to_tokens_ssr(
                &nodes[0],
                global_class,
                call_site,
            ),
            _ => server_template::fragment_to_tokens_ssr(
                nodes,
                global_class,
                call_site,
            ),
        }
    } else {
        match nodes.len() {
            0 => empty,
            1 => client_builder::node_to_tokens(
                &nodes[0],
                client_builder::TagType::Unknown,
                None,
                global_class,
                call_site,
            )
            .unwrap_or_default(),
            _ => client_builder::fragment_to_tokens(
                nodes,
                true,
                client_builder::TagType::Unknown,
                None,
                global_class,
                call_site,
            )
            .unwrap_or(empty),
        }
    }
}

// Keep list alphabetized for binary search
const TYPED_EVENTS: [&str; 126] = [
    "DOMContentLoaded",
    "abort",
    "afterprint",
    "animationcancel",
    "animationend",
    "animationiteration",
    "animationstart",
    "auxclick",
    "beforeinput",
    "beforeprint",
    "beforeunload",
    "blur",
    "canplay",
    "canplaythrough",
    "change",
    "click",
    "close",
    "compositionend",
    "compositionstart",
    "compositionupdate",
    "contextmenu",
    "copy",
    "cuechange",
    "cut",
    "dblclick",
    "devicemotion",
    "deviceorientation",
    "drag",
    "dragend",
    "dragenter",
    "dragleave",
    "dragover",
    "dragstart",
    "drop",
    "durationchange",
    "emptied",
    "ended",
    "error",
    "focus",
    "focusin",
    "focusout",
    "formdata",
    "fullscreenchange",
    "fullscreenerror",
    "gamepadconnected",
    "gamepaddisconnected",
    "gotpointercapture",
    "hashchange",
    "input",
    "invalid",
    "keydown",
    "keypress",
    "keyup",
    "languagechange",
    "load",
    "loadeddata",
    "loadedmetadata",
    "loadstart",
    "lostpointercapture",
    "message",
    "messageerror",
    "mousedown",
    "mouseenter",
    "mouseleave",
    "mousemove",
    "mouseout",
    "mouseover",
    "mouseup",
    "offline",
    "online",
    "orientationchange",
    "pagehide",
    "pageshow",
    "paste",
    "pause",
    "play",
    "playing",
    "pointercancel",
    "pointerdown",
    "pointerenter",
    "pointerleave",
    "pointerlockchange",
    "pointerlockerror",
    "pointermove",
    "pointerout",
    "pointerover",
    "pointerup",
    "popstate",
    "progress",
    "ratechange",
    "readystatechange",
    "rejectionhandled",
    "reset",
    "resize",
    "scroll",
    "securitypolicyviolation",
    "seeked",
    "seeking",
    "select",
    "selectionchange",
    "selectstart",
    "slotchange",
    "stalled",
    "storage",
    "submit",
    "suspend",
    "timeupdate",
    "toggle",
    "touchcancel",
    "touchend",
    "touchmove",
    "touchstart",
    "transitioncancel",
    "transitionend",
    "transitionrun",
    "transitionstart",
    "unhandledrejection",
    "unload",
    "visibilitychange",
    "volumechange",
    "waiting",
    "webkitanimationend",
    "webkitanimationiteration",
    "webkitanimationstart",
    "webkittransitionend",
    "wheel",
];

const CUSTOM_EVENT: &str = "Custom";

pub(crate) fn parse_event_name(name: &str) -> (TokenStream, bool, bool) {
    let (name, is_force_undelegated) = parse_event(name);

    let (event_type, is_custom) = TYPED_EVENTS
        .binary_search(&name)
        .map(|_| (name, false))
        .unwrap_or((CUSTOM_EVENT, true));

    let Ok(event_type) = event_type.parse::<TokenStream>() else {
        abort!(event_type, "couldn't parse event name");
    };

    let event_type = if is_custom {
        quote! { Custom::new(#name) }
    } else {
        event_type
    };
    (event_type, is_custom, is_force_undelegated)
}

fn expr_to_ident(expr: &syn::Expr) -> Option<&ExprPath> {
    match expr {
        syn::Expr::Block(block) => block.block.stmts.last().and_then(|stmt| {
            if let syn::Stmt::Expr(expr, ..) = stmt {
                expr_to_ident(expr)
            } else {
                None
            }
        }),
        syn::Expr::Path(path) => Some(path),
        _ => None,
    }
}

fn convert_to_snake_case(name: String) -> String {
    if !name.is_case(Snake) {
        name.to_case(Snake)
    } else {
        name
    }
}

fn is_custom_element(tag: &str) -> bool {
    tag.contains('-')
}

fn is_self_closing(node: &NodeElement) -> bool {
    // self-closing tags
    // https://developer.mozilla.org/en-US/docs/Glossary/Empty_element
    [
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link",
        "meta", "param", "source", "track", "wbr",
    ]
    .binary_search(&node.name().to_string().as_str())
    .is_ok()
}

fn camel_case_tag_name(tag_name: &str) -> String {
    let mut chars = tag_name.chars();
    let first = chars.next();
    let underscore = if tag_name == "option" { "_" } else { "" };
    first
        .map(|f| f.to_ascii_uppercase())
        .into_iter()
        .chain(chars)
        .collect::<String>()
        + underscore
}

fn is_svg_element(tag: &str) -> bool {
    // Keep list alphabetized for binary search
    [
        "animate",
        "animateMotion",
        "animateTransform",
        "circle",
        "clipPath",
        "defs",
        "desc",
        "discard",
        "ellipse",
        "feBlend",
        "feColorMatrix",
        "feComponentTransfer",
        "feComposite",
        "feConvolveMatrix",
        "feDiffuseLighting",
        "feDisplacementMap",
        "feDistantLight",
        "feDropShadow",
        "feFlood",
        "feFuncA",
        "feFuncB",
        "feFuncG",
        "feFuncR",
        "feGaussianBlur",
        "feImage",
        "feMerge",
        "feMergeNode",
        "feMorphology",
        "feOffset",
        "fePointLight",
        "feSpecularLighting",
        "feSpotLight",
        "feTile",
        "feTurbulence",
        "filter",
        "foreignObject",
        "g",
        "hatch",
        "hatchpath",
        "image",
        "line",
        "linearGradient",
        "marker",
        "mask",
        "metadata",
        "mpath",
        "path",
        "pattern",
        "polygon",
        "polyline",
        "radialGradient",
        "rect",
        "set",
        "stop",
        "svg",
        "switch",
        "symbol",
        "text",
        "textPath",
        "tspan",
        "use",
        "use_",
        "view",
    ]
    .binary_search(&tag)
    .is_ok()
}

fn is_math_ml_element(tag: &str) -> bool {
    // Keep list alphabetized for binary search
    [
        "annotation",
        "maction",
        "math",
        "menclose",
        "merror",
        "mfenced",
        "mfrac",
        "mi",
        "mmultiscripts",
        "mn",
        "mo",
        "mover",
        "mpadded",
        "mphantom",
        "mprescripts",
        "mroot",
        "mrow",
        "ms",
        "mspace",
        "msqrt",
        "mstyle",
        "msub",
        "msubsup",
        "msup",
        "mtable",
        "mtd",
        "mtext",
        "mtr",
        "munder",
        "munderover",
        "semantics",
    ]
    .binary_search(&tag)
    .is_ok()
}

fn is_ambiguous_element(tag: &str) -> bool {
    tag == "a" || tag == "script" || tag == "title"
}

fn parse_event(event_name: &str) -> (&str, bool) {
    if let Some(event_name) = event_name.strip_suffix(":undelegated") {
        (event_name, true)
    } else {
        (event_name, false)
    }
}

fn fancy_class_name<'a>(
    name: &str,

    node: &'a KeyedAttribute,
) -> Option<(TokenStream, String, &'a Expr)> {
    // special case for complex class names:
    // e.g., Tailwind `class=("mt-[calc(100vh_-_3rem)]", true)`
    if name == "class" {
        if let Some(Tuple(tuple)) = node.value() {
            if tuple.elems.len() == 2 {
                let class = quote_spanned! {
                    node.key.span()=> .class
                };
                let class_name = &tuple.elems[0];
                let value = &tuple.elems[1];

                match class_name {
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }) => {
                        let class_name = s.value();
                        return Some((
                            quote! {
                                #class(#class_name, #value)
                            },
                            class_name,
                            value,
                        ));
                    }

                    Expr::Array(ExprArray { elems, .. }) => {
                        let (tokens, class_name): (Vec<_>, Vec<_>) = elems
                            .iter()
                            .map(|elem| match elem {
                                Expr::Lit(ExprLit {
                                    lit: Lit::Str(s), ..
                                }) => {
                                    let class_name = s.value();
                                    let tokens = quote! {
                                        #class(#class_name, #value)
                                    };
                                    (tokens, class_name)
                                }

                                _ => {
                                    proc_macro_error2::emit_error!(
                                        elem.span(),
                                        "class name elements must be string \
                                         literals"
                                    );

                                    (TokenStream::new(), Default::default())
                                }
                            })
                            .unzip();

                        let class_name = class_name.join(" ");
                        return Some((
                            quote! { #(#tokens)*},
                            class_name,
                            value,
                        ));
                    }

                    _ => {
                        proc_macro_error2::emit_error!(
                            class_name.span(),
                            "class name must be a string literal or array of \
                             string literals"
                        );
                        let class_name = Default::default();
                        return Some((
                            quote! {
                                #class(#class_name, #value)
                            },
                            class_name,
                            value,
                        ));
                    }
                }
            } else {
                proc_macro_error2::emit_error!(
                    tuple.span(),
                    "class tuples must have two elements."
                )
            }
        }
    }
    None
}

fn ident_from_tag_name(tag_name: &NodeName) -> Ident {
    match tag_name {
        NodeName::Path(path) => path
            .path
            .segments
            .iter()
            .last()
            .map(|segment| segment.ident.clone())
            .expect("element needs to have a name"),
        NodeName::Block(_) => {
            let span = tag_name.span();
            proc_macro_error2::emit_error!(
                span,
                "blocks not allowed in tag-name position"
            );
            Ident::new("", span)
        }
        _ => Ident::new(
            &tag_name.to_string().replace(['-', ':'], "_"),
            tag_name.span(),
        ),
    }
}

fn fancy_style_name<'a>(
    name: &str,

    node: &'a KeyedAttribute,
) -> Option<(TokenStream, String, &'a Expr)> {
    // special case for complex dynamic style names:
    if name == "style" {
        if let Some(Tuple(tuple)) = node.value() {
            if tuple.elems.len() == 2 {
                let style = quote_spanned! {
                    node.key.span()=> .style
                };
                let style_name = &tuple.elems[0];
                let style_name = if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s),
                    ..
                }) = style_name
                {
                    s.value()
                } else {
                    proc_macro_error2::emit_error!(
                        style_name.span(),
                        "style name must be a string literal"
                    );
                    Default::default()
                };
                let value = &tuple.elems[1];
                return Some((
                    quote! {
                        #style(#style_name, #value)
                    },
                    style_name,
                    value,
                ));
            } else {
                proc_macro_error2::emit_error!(
                    tuple.span(),
                    "style tuples must have two elements."
                )
            }
        }
    }
    None
}

pub(crate) fn event_from_attribute_node(
    attr: &KeyedAttribute,
    force_undelegated: bool,
) -> (TokenStream, &Expr) {
    let event_name = attr
        .key
        .to_string()
        .strip_prefix("on:")
        .expect("expected `on:` directive")
        .to_owned();

    let handler = attribute_value(attr);

    let (event_type, is_custom, name_undelegated) =
        parse_event_name(&event_name);

    // HACK(chrisp60): in the code above, the original span information is lost
    // as the event name is parsed from a stringified version of the tokens.
    //
    // This assumes that the attribute key is structured as "on:some_event_name" and
    // just skips the "on:" part, isolating the "some_event_name" tokens. In turn,
    // we keep the span information from the original event identifier.
    //
    // .nth(2) is because syn parses follows
    // token 0: "on"
    // token 1: ":"
    // token 2: "event"
    //
    // There are cleaners ways to do this but this is a legacy branch.
    let original_tokens = attr
        .key
        .to_token_stream()
        .into_iter()
        .nth(2)
        .expect("tokens following on:"); // see previous call to .expect in this same function

    // is_custom wraps the event type in a struct definition, so don't use
    // our original tokens.
    let absolute_ev = if is_custom {
        quote! { ::leptos::leptos_dom::ev::#event_type }
    } else {
        quote! { ::leptos::leptos_dom::ev::#original_tokens }
    };

    let event_type = if force_undelegated || name_undelegated {
        quote! { ::leptos::leptos_dom::ev::undelegated(#absolute_ev) }
    } else {
        quote! { ::leptos::leptos_dom::ev::#absolute_ev }
    };
    (event_type, handler)
}

pub(crate) fn directive_call_from_attribute_node(
    attr: &KeyedAttribute,
    directive_name: &str,
) -> TokenStream {
    let handler = syn::Ident::new(directive_name, attr.key.span());

    let param = if let Some(value) = attr.value() {
        quote!(::std::convert::Into::into(#value))
    } else {
        quote_spanned!(attr.key.span()=> ().into())
    };

    quote! { .directive(#handler, #[allow(clippy::useless_conversion)] #param) }
}
