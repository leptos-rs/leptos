mod component_builder;
mod slot_helper;

use self::{
    component_builder::component_to_tokens,
    slot_helper::{get_slot, slot_to_tokens},
};
use convert_case::{Case::Snake, Casing};
use leptos_hot_reload::parsing::{is_component_node, value_to_string};
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use proc_macro_error::abort;
use quote::{quote, quote_spanned, ToTokens};
use rstml::node::{
    KeyedAttribute, Node, NodeAttribute, NodeBlock, NodeElement, NodeName,
    NodeNameFragment,
};
use std::collections::HashMap;
use syn::{spanned::Spanned, Expr, ExprRange, Lit, LitStr, RangeLimits, Stmt};

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
    let (base, should_add_view) = match nodes.len() {
        0 => {
            let span = Span::call_site();
            (
                Some(quote_spanned! {
                    span => ()
                }),
                false,
            )
        }
        1 => (
            node_to_tokens(
                &nodes[0],
                TagType::Unknown,
                None,
                global_class,
                view_marker.as_deref(),
            ),
            // only add .into_view() and view marker to a regular HTML
            // element or component, not to a <{..} /> attribute list
            match &nodes[0] {
                Node::Element(node) => !is_spread_marker(node),
                _ => false,
            },
        ),
        _ => (
            fragment_to_tokens(
                nodes,
                TagType::Unknown,
                None,
                global_class,
                view_marker.as_deref(),
            ),
            true,
        ),
    };
    base.map(|view| {
        if !should_add_view {
            view
        } else if let Some(vm) = view_marker {
            quote! {
                #view
                .into_view()
                .with_view_marker(#vm)
            }
        } else {
            quote! {
                #view.into_view()
            }
        }
    })
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
    if children.is_empty() {
        None
    } else if children.len() == 1 {
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
        let mut slots = HashMap::new();
        let nodes = nodes
            .iter()
            .filter_map(|node| {
                node_to_tokens(
                    node,
                    TagType::Unknown,
                    Some(&mut slots),
                    global_class,
                    view_marker,
                )
            })
            .collect();
        if let Some(parent_slots) = parent_slots {
            for (slot, mut values) in slots.drain() {
                parent_slots
                    .entry(slot)
                    .and_modify(|entry| entry.append(&mut values))
                    .or_insert(values);
            }
        }
        nodes
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
        if let Some(slot) = get_slot(node) {
            slot_to_tokens(node, slot, parent_slots, global_class);
            None
        } else {
            Some(component_to_tokens(node, global_class))
        }
    } else if is_spread_marker(node) {
        let mut attributes = Vec::new();
        let mut additions = Vec::new();
        for node in node.attributes() {
            match node {
                NodeAttribute::Block(block) => {
                    if let NodeBlock::ValidBlock(block) = block {
                        match block.stmts.first() {
                            Some(Stmt::Expr(
                                Expr::Range(ExprRange {
                                    start: None,
                                    limits: RangeLimits::HalfOpen(_),
                                    end: Some(end),
                                    ..
                                }),
                                _,
                            )) => {
                                additions.push(quote! { #end });
                            }
                            _ => {
                                additions.push(quote! { #block });
                            }
                        }
                    } else {
                        additions.push(quote! { #block });
                    }
                }
                NodeAttribute::Attribute(node) => {
                    if let Some(content) = attribute_absolute(node, true) {
                        attributes.push(content);
                    }
                }
            }
        }
        Some(quote! {
            (#(#attributes,)*)
            #(.add_any_attr(#additions))*
        })
    } else {
        let tag = name.to_string();
        // collect close_tag name to emit semantic information for IDE.
        /* TODO restore this
        let mut ide_helper_close_tag = IdeTagHelper::new();
        let close_tag = node.close_tag.as_ref().map(|c| &c.name);*/
        let is_custom = is_custom_element(&tag);
        let name = if is_custom {
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
                is_custom,
            ))
        } else {
            let nodes = attributes.iter().map(|node| {
                attribute_to_tokens(parent_type, node, global_class, is_custom)
            });
            Some(quote! {
                #(#nodes)*
            })
        };

        let global_class_expr = global_class.map(|class| {
            quote! { .class((#class, true)) }
        });

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
            #global_class_expr
        })
    }
}

fn is_spread_marker(node: &NodeElement) -> bool {
    match node.name() {
        NodeName::Block(block) => matches!(
            block.stmts.first(),
            Some(Stmt::Expr(
                Expr::Range(ExprRange {
                    start: None,
                    limits: RangeLimits::HalfOpen(_),
                    end: None,
                    ..
                }),
                _,
            ))
        ),
        _ => false,
    }
}

fn attribute_to_tokens(
    tag_type: TagType,
    node: &NodeAttribute,
    global_class: Option<&TokenTree>,
    is_custom: bool,
) -> TokenStream {
    match node {
        NodeAttribute::Block(node) => {
            let dotted = if let NodeBlock::ValidBlock(block) = node {
                match block.stmts.first() {
                    Some(Stmt::Expr(
                        Expr::Range(ExprRange {
                            start: None,
                            limits: RangeLimits::HalfOpen(_),
                            end: Some(end),
                            ..
                        }),
                        _,
                    )) => Some(quote! { .add_any_attr(#end) }),
                    _ => None,
                }
            } else {
                None
            };
            dotted.unwrap_or_else(|| {
                quote! {
                    .add_any_attr(#[allow(unused_braces)] { #node })
                }
            })
        }
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
            } else if let Some(name) = name.strip_prefix("use:") {
                directive_call_from_attribute_node(node, name)
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
            }
            // circumstances in which we just do unchecked attributes
            // 1) custom elements, which can have any attributes
            // 2) custom attributes and data attributes (so, anything with - in it)
            else if is_custom ||
                (name.contains('-') && !name.starts_with("aria-"))
                // TODO check: do we actually provide SVG attributes?
                // we don't provide statically-checked methods for SVG attributes
                || (tag_type == TagType::Svg && name != "inner_html")
            {
                let value = attribute_value(node);
                quote! {
                    .attr(#name, #value)
                }
            } else {
                let key = attribute_name(&node.key);
                let value = attribute_value(node);

                // special case of global_class and class attribute
                if &node.key.to_string() == "class"
                    && global_class.is_some()
                    && node.value().and_then(value_to_string).is_none()
                {
                    let span = node.key.span();
                    proc_macro_error::emit_error!(span, "Combining a global class (view! { class = ... }) \
            and a dynamic `class=` attribute on an element causes runtime inconsistencies. You can \
            toggle individual classes dynamically with the `class:name=value` syntax. \n\nSee this issue \
            for more information and an example: https://github.com/leptos-rs/leptos/issues/773")
                };

                quote! {
                    .#key(#value)
                }
            }
        }
    }
}

/// Returns attribute values with an absolute path
pub(crate) fn attribute_absolute(
    node: &KeyedAttribute,
    after_spread: bool,
) -> Option<TokenStream> {
    let contains_dash = node.key.to_string().contains('-');
    // anything that follows the x:y pattern
    match &node.key {
        NodeName::Punctuated(parts) if !contains_dash => {
            if parts.len() >= 2 {
                let id = &parts[0];
                match id {
                    NodeNameFragment::Ident(id) => {
                        let value = attribute_value(node);
                        // ignore `let:`
                        if id == "let" {
                            None
                        } else if id == "attr" {
                            let key = &parts[1];
                            let key_name = key.to_string();
                            if key_name == "class" || key_name == "style" {
                                Some(
                                    quote! { ::leptos::tachys::html::#key::#key(#value) },
                                )
                            } else {
                                Some(
                                    quote! { ::leptos::tachys::html::attribute::#key(#value) },
                                )
                            }
                        } else if id == "use" {
                            let key = &parts[1];
                            let param = if let Some(value) = node.value() {
                                quote!(::std::convert::Into::into(#value))
                            } else {
                                quote_spanned!(node.key.span()=> ().into())
                            };
                            Some(
                                quote! {
                                    ::leptos::tachys::html::directive::directive(
                                        #key,
                                        #[allow(clippy::useless_conversion)] #param
                                    )
                                },
                            )
                        } else if id == "style" || id == "class" {
                            let key = &node.key.to_string();
                            let key = key
                                .replacen("style:", "", 1)
                                .replacen("class:", "", 1);
                            Some(
                                quote! { ::leptos::tachys::html::#id::#id((#key, #value)) },
                            )
                        } else if id == "prop" {
                            let key = &node.key.to_string();
                            let key = key.replacen("prop:", "", 1);
                            Some(
                                quote! { ::leptos::tachys::html::property::#id(#key, #value) },
                            )
                        } else if id == "on" {
                            let key = &node.key.to_string();
                            let key = key.replacen("on:", "", 1);
                            let (on, ty, handler) =
                                event_type_and_handler(&key, node);
                            Some(
                                quote! { ::leptos::tachys::html::event::#on(#ty, #handler) },
                            )
                        } else {
                            proc_macro_error::abort!(
                                id.span(),
                                &format!(
                                    "`{id}:` syntax is not supported on \
                                     components"
                                )
                            );
                        }
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => after_spread.then(|| {
            let key = attribute_name(&node.key);
            let value = &node.value();
            let name = &node.key.to_string();
            if name == "class" || name == "style" {
                quote! {
                    ::leptos::tachys::html::#key::#key(#value)
                }
            }
            else if name.contains('-') && !name.starts_with("aria-") {
                quote! {
                    ::leptos::tachys::html::attribute::custom::custom_attribute(#name, #value)
                }
            }
            else {
                quote! {
                    ::leptos::tachys::html::attribute::#key(#value)
                }
            }
        }),
    }
}

pub(crate) fn event_to_tokens(
    name: &str,
    node: &KeyedAttribute,
) -> TokenStream {
    let (on, event_type, handler) = event_type_and_handler(name, node);

    quote! {
        .#on(#event_type, #handler)
    }
}

pub(crate) fn event_type_and_handler(
    name: &str,
    node: &KeyedAttribute,
) -> (TokenStream, TokenStream, TokenStream) {
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
        quote! { ::leptos::tachys::html::event::#undelegated(::leptos::tachys::html::event::#event_type) }
    } else {
        quote! { ::leptos::tachys::html::event::#event_type }
    };

    (on, event_type, handler)
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

fn convert_to_snake_case(name: String) -> String {
    if !name.is_case(Snake) {
        name.to_case(Snake)
    } else {
        name
    }
}

pub(crate) fn ident_from_tag_name(tag_name: &NodeName) -> Ident {
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
            proc_macro_error::emit_error!(
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
