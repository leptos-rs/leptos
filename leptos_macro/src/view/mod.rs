mod component_builder;

use self::component_builder::component_to_tokens;
use convert_case::{Case::Snake, Casing};
use leptos_hot_reload::parsing::is_component_node;
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use proc_macro_error::abort;
use quote::{quote, quote_spanned, ToTokens};
use rstml::node::{KeyedAttribute, Node, NodeAttribute, NodeElement, NodeName};
use std::collections::HashMap;
use syn::{spanned::Spanned, Expr, ExprPath, Lit, LitStr};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TagType {
    Unknown,
    Html,
    Svg,
    Math,
}

pub fn render_view(
    nodes: &[Node],
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> Option<TokenStream> {
    match nodes.len() {
        0 => {
            let span = Span::call_site();
            Some(quote_spanned! {
                span => ()
            })
        }
        1 => node_to_tokens(
            &nodes[0],
            TagType::Unknown,
            None,
            global_class,
            view_marker.as_deref(),
        ),
        _ => fragment_to_tokens(
            nodes,
            TagType::Unknown,
            None,
            global_class,
            view_marker.as_deref(),
        ),
    }
}

fn element_children_to_tokens(
    nodes: &[Node],
    parent_type: TagType,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    view_marker: Option<&str>,
) -> Option<TokenStream> {
    let children = children_to_tokens(
        nodes,
        parent_type,
        parent_slots,
        global_class,
        view_marker,
    )
    .into_iter()
    .map(|child| {
        quote! {
            .child(
                #[allow(unused_braces)]
                { #child }
            )
        }
    });
    Some(quote! {
        #(#children)*
    })
}

fn fragment_to_tokens(
    nodes: &[Node],
    parent_type: TagType,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    view_marker: Option<&str>,
) -> Option<TokenStream> {
    let children = children_to_tokens(
        nodes,
        parent_type,
        parent_slots,
        global_class,
        view_marker,
    );
    if children.len() == 1 {
        children.into_iter().next()
    } else {
        Some(quote! {
            (#(#children),*)
        })
    }
}

fn children_to_tokens(
    nodes: &[Node],
    parent_type: TagType,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    view_marker: Option<&str>,
) -> Vec<TokenStream> {
    if nodes.len() == 1 {
        match node_to_tokens(
            &nodes[0],
            parent_type,
            parent_slots,
            global_class,
            view_marker,
        ) {
            Some(tokens) => vec![tokens],
            None => vec![],
        }
    } else {
        nodes
            .iter()
            .filter_map(|node| {
                node_to_tokens(
                    node,
                    TagType::Unknown,
                    None,
                    global_class,
                    view_marker,
                )
            })
            .collect()
    }
}

fn node_to_tokens(
    node: &Node,
    parent_type: TagType,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    view_marker: Option<&str>,
) -> Option<TokenStream> {
    match node {
        Node::Comment(_) => None,
        Node::Doctype(node) => {
            let value = node.value.to_string_best();
            Some(quote! { ::leptos::tachys::html::doctype(#value) })
        }
        Node::Fragment(fragment) => fragment_to_tokens(
            &fragment.children,
            parent_type,
            parent_slots,
            global_class,
            view_marker,
        ),
        Node::Block(block) => Some(quote! { #block }),
        Node::Text(text) => Some(text_to_tokens(&text.value)),
        Node::RawText(raw) => {
            let text = raw.to_string_best();
            let text = syn::LitStr::new(&text, raw.span());
            Some(text_to_tokens(&text))
        }
        Node::Element(node) => element_to_tokens(
            node,
            parent_type,
            parent_slots,
            global_class,
            view_marker,
        ),
    }
}

fn text_to_tokens(text: &LitStr) -> TokenStream {
    // on nightly, can use static string optimization
    if cfg!(feature = "nightly") {
        quote! {
            ::leptos::tachys::view::static_types::Static::<#text>
        }
    }
    // otherwise, just use the literal string
    else {
        quote! { #text }
    }
}

pub(crate) fn element_to_tokens(
    node: &NodeElement,
    mut parent_type: TagType,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    view_marker: Option<&str>,
) -> Option<TokenStream> {
    let name = node.name();
    if is_component_node(node) {
        // TODO slots
        /* if let Some(slot) = get_slot(node) {
            slot_to_tokens(node, slot, parent_slots, global_class);
            None
        } else { */
        Some(component_to_tokens(node, global_class))
        //}
    } else {
        let tag = name.to_string();
        // collect close_tag name to emit semantic information for IDE.
        /* TODO restore this
        let mut ide_helper_close_tag = IdeTagHelper::new();
        let close_tag = node.close_tag.as_ref().map(|c| &c.name);*/
        let name = if is_custom_element(&tag) {
            let name = node.name().to_string();
            // link custom ident to name span for IDE docs
            let custom = Ident::new("custom", name.span());
            quote! { ::leptos::tachys::html::element::#custom(#name) }
        } else if is_svg_element(&tag) {
            parent_type = TagType::Svg;
            quote! { ::leptos::tachys::svg::#name() }
        } else if is_math_ml_element(&tag) {
            parent_type = TagType::Math;
            quote! { ::leptos::tachys::mathml::#name() }
        } else if is_ambiguous_element(&tag) {
            match parent_type {
                TagType::Unknown => {
                    // We decided this warning was too aggressive, but I'll leave it here in case we want it later
                    /* proc_macro_error::emit_warning!(name.span(), "The view macro is assuming this is an HTML element, \
                    but it is ambiguous; if it is an SVG or MathML element, prefix with svg:: or math::"); */
                    quote! {
                        ::leptos::tachys::html::element::#name()
                    }
                }
                TagType::Html => {
                    quote! { ::leptos::tachys::html::element::#name() }
                }
                TagType::Svg => {
                    quote! { ::leptos::tachys::svg::#name() }
                }
                TagType::Math => {
                    quote! { ::leptos::tachys::math::#name() }
                }
            }
        } else {
            parent_type = TagType::Html;
            quote! { ::leptos::tachys::html::element::#name() }
        };

        /* TODO restore this
        if let Some(close_tag) = close_tag {
            ide_helper_close_tag.save_tag_completion(close_tag)
        } */

        let attributes = node.attributes();
        let attributes = if attributes.len() == 1 {
            Some(attribute_to_tokens(
                parent_type,
                &attributes[0],
                global_class,
            ))
        } else {
            let nodes = attributes.iter().map(|node| {
                attribute_to_tokens(parent_type, node, global_class)
            });
            Some(quote! {
                #(#nodes)*
            })
        };

        let self_closing = is_self_closing(node);
        let children = if !self_closing {
            element_children_to_tokens(
                &node.children,
                parent_type,
                parent_slots,
                global_class,
                view_marker,
            )
        } else {
            if !node.children.is_empty() {
                let name = node.name();
                proc_macro_error::emit_error!(
                    name.span(),
                    format!(
                        "Self-closing elements like <{name}> cannot have \
                         children."
                    )
                );
            };
            None
        };

        // attributes are placed second because this allows `inner_html`
        // to object if there are already children
        Some(quote! {
            #name
            #children
            #attributes
        })
    }
}

fn attribute_to_tokens(
    tag_type: TagType,
    node: &NodeAttribute,
    global_class: Option<&TokenTree>,
) -> TokenStream {
    match node {
        NodeAttribute::Block(_) => todo!(),
        NodeAttribute::Attribute(node) => {
            let name = node.key.to_string();
            if name == "node_ref" {
                let node_ref = match &node.key {
                    NodeName::Path(path) => path.path.get_ident(),
                    _ => unreachable!(),
                };
                let value = attribute_value(node);
                quote! {
                    .#node_ref(#value)
                }
            } else if let Some(name) = name.strip_prefix("on:") {
                event_to_tokens(name, node)
            } else if let Some(name) = name.strip_prefix("class:") {
                let class = match &node.key {
                    NodeName::Punctuated(parts) => &parts[0],
                    _ => unreachable!(),
                };
                class_to_tokens(node, class.into_token_stream(), Some(name))
            } else if name == "class" {
                let class = match &node.key {
                    NodeName::Path(path) => path.path.get_ident(),
                    _ => unreachable!(),
                };
                class_to_tokens(node, class.into_token_stream(), None)
            } else if let Some(name) = name.strip_prefix("style:") {
                let style = match &node.key {
                    NodeName::Punctuated(parts) => &parts[0],
                    _ => unreachable!(),
                };
                style_to_tokens(node, style.into_token_stream(), Some(name))
            } else if name == "style" {
                let style = match &node.key {
                    NodeName::Path(path) => path.path.get_ident(),
                    _ => unreachable!(),
                };
                style_to_tokens(node, style.into_token_stream(), None)
            } else if let Some(name) = name.strip_prefix("prop:") {
                let prop = match &node.key {
                    NodeName::Punctuated(parts) => &parts[0],
                    _ => unreachable!(),
                };
                prop_to_tokens(node, prop.into_token_stream(), name)
            } else if name.contains('-')
                // we don't provide statically-checked methods for SVG attributes
                || tag_type == TagType::Svg
            {
                let value = attribute_value(node);
                quote! {
                    .attr(#name, #value)
                }
            } else {
                let key = attribute_name(&node.key);
                let value = attribute_value(node);
                quote! {
                    .#key(#value)
                }
            }
        }
    }
}

pub(crate) fn event_to_tokens(
    name: &str,
    node: &KeyedAttribute,
) -> TokenStream {
    let handler = attribute_value(node);

    let (event_type, is_custom, is_force_undelegated, is_targeted) =
        parse_event_name(name);

    let event_name_ident = match &node.key {
        NodeName::Punctuated(parts) => {
            if parts.len() >= 2 {
                Some(&parts[1])
            } else {
                None
            }
        }
        _ => unreachable!(),
    };
    let undelegated_ident = match &node.key {
        NodeName::Punctuated(parts) => {
            parts.iter().find(|part| part.to_string() == "undelegated")
        }
        _ => unreachable!(),
    };
    let on = match &node.key {
        NodeName::Punctuated(parts) => &parts[0],
        _ => unreachable!(),
    };
    let on = if is_targeted {
        Ident::new("on_target", on.span()).to_token_stream()
    } else {
        on.to_token_stream()
    };
    let event_type = if is_custom {
        event_type
    } else if let Some(ev_name) = event_name_ident {
        let span = ev_name.span();
        quote_spanned! {
            span => #ev_name
        }
    } else {
        event_type
    };

    let event_type = if is_force_undelegated {
        let undelegated = if let Some(undelegated) = undelegated_ident {
            let span = undelegated.span();
            quote_spanned! {
                span => #undelegated
            }
        } else {
            quote! { undelegated }
        };
        // TODO undelegated
        quote! { ::leptos::tachys::html::event::#undelegated(::leptos::tachys::html::event::#event_type) }
    } else {
        quote! { ::leptos::tachys::html::event::#event_type }
    };

    quote! {
        .#on(#event_type, #handler)
    }
}

fn class_to_tokens(
    node: &KeyedAttribute,
    class: TokenStream,
    class_name: Option<&str>,
) -> TokenStream {
    let value = attribute_value(node);
    if let Some(class_name) = class_name {
        quote! {
            .#class((#class_name, #value))
        }
    } else {
        quote! {
            .#class(#value)
        }
    }
}

fn style_to_tokens(
    node: &KeyedAttribute,
    style: TokenStream,
    style_name: Option<&str>,
) -> TokenStream {
    let value = attribute_value(node);
    if let Some(style_name) = style_name {
        quote! {
            .#style((#style_name, #value))
        }
    } else {
        quote! {
            .#style(#value)
        }
    }
}

fn prop_to_tokens(
    node: &KeyedAttribute,
    prop: TokenStream,
    key: &str,
) -> TokenStream {
    let value = attribute_value(node);
    quote! {
        .#prop(#key, #value)
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

fn parse_event(event_name: &str) -> (String, bool, bool) {
    let is_undelegated = event_name.contains(":undelegated");
    let is_targeted = event_name.contains(":target");
    let event_name = event_name
        .replace(":undelegated", "")
        .replace(":target", "");
    (event_name, is_undelegated, is_targeted)
}

/// Escapes Rust keywords that are also HTML attribute names
/// to their raw-identifier form.
fn attribute_name(name: &NodeName) -> TokenStream {
    let s = name.to_string();
    if s == "as" || s == "async" || s == "loop" || s == "for" || s == "type" {
        Ident::new_raw(&s, name.span()).to_token_stream()
    } else if s.starts_with("aria-") {
        Ident::new(&s.replace('-', "_"), name.span()).to_token_stream()
    } else {
        name.to_token_stream()
    }
}

fn attribute_value(attr: &KeyedAttribute) -> TokenStream {
    match attr.value() {
        Some(value) => {
            if let Expr::Lit(lit) = value {
                if cfg!(feature = "nightly") {
                    if let Lit::Str(str) = &lit.lit {
                        return quote! {
                            ::leptos::tachys::view::static_types::Static::<#str>
                        };
                    }
                }
            }
            quote! { #value }
        }
        None => quote! { true },
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

pub(crate) fn parse_event_name(name: &str) -> (TokenStream, bool, bool, bool) {
    let (name, is_force_undelegated, is_targeted) = parse_event(name);

    let (event_type, is_custom) = TYPED_EVENTS
        .binary_search(&name.as_str())
        .map(|_| (name.as_str(), false))
        .unwrap_or((CUSTOM_EVENT, true));

    let Ok(event_type) = event_type.parse::<TokenStream>() else {
        abort!(event_type, "couldn't parse event name");
    };

    let event_type = if is_custom {
        quote! { Custom::new(#name) }
    } else {
        event_type
    };
    (event_type, is_custom, is_force_undelegated, is_targeted)
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
