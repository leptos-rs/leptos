use std::collections::HashMap;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, ExprPath};
use syn_rsx::{Node, NodeAttribute, NodeElement, NodeName};

use crate::{is_component_node, Mode};

const NON_BUBBLING_EVENTS: [&str; 11] = [
    "load",
    "unload",
    "scroll",
    "focus",
    "blur",
    "loadstart",
    "progress",
    "error",
    "abort",
    "load",
    "loadend",
];

lazy_static::lazy_static! {
    // Specialized event type
    // https://github.com/yewstack/yew/blob/d422b533ea19a09cddf9b31ecd6cd5e5ce35ce3f/packages/yew/src/html/listener/events.rs
    static ref EVENTS: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("auxclick", "MouseEvent");
        m.insert("click", "MouseEvent");

        m.insert("contextmenu", "MouseEvent");
        m.insert("dblclick", "MouseEvent");

        m.insert("drag", "DragEvent");
        m.insert("dragend", "DragEvent");
        m.insert("dragenter", "DragEvent");
        m.insert("dragexit", "DragEvent");
        m.insert("dragleave", "DragEvent");
        m.insert("dragover", "DragEvent");
        m.insert("dragstart", "DragEvent");
        m.insert("drop", "DragEvent");

        m.insert("blur", "FocusEvent");
        m.insert("focus", "FocusEvent");
        m.insert("focusin", "FocusEvent");
        m.insert("focusout", "FocusEvent");

        m.insert("keydown", "KeyboardEvent");
        m.insert("keypress", "KeyboardEvent");
        m.insert("keyup", "KeyboardEvent");

        m.insert("loadstart", "ProgressEvent");
        m.insert("progress", "ProgressEvent");
        m.insert("loadend", "ProgressEvent");

        m.insert("mousedown", "MouseEvent");
        m.insert("mouseenter", "MouseEvent");
        m.insert("mouseleave", "MouseEvent");
        m.insert("mousemove", "MouseEvent");
        m.insert("mouseout", "MouseEvent");
        m.insert("mouseover", "MouseEvent");
        m.insert("mouseup", "MouseEvent");
        m.insert("wheel", "WheelEvent");

        m.insert("input", "InputEvent");

        m.insert("submit", "SubmitEvent");

        m.insert("animationcancel", "AnimationEvent");
        m.insert("animationend", "AnimationEvent");
        m.insert("animationiteration", "AnimationEvent");
        m.insert("animationstart", "AnimationEvent");

        m.insert("gotpointercapture", "PointerEvent");
        m.insert("lostpointercapture", "PointerEvent");
        m.insert("pointercancel", "PointerEvent");
        m.insert("pointerdown", "PointerEvent");
        m.insert("pointerenter", "PointerEvent");
        m.insert("pointerleave", "PointerEvent");
        m.insert("pointermove", "PointerEvent");
        m.insert("pointerout", "PointerEvent");
        m.insert("pointerover", "PointerEvent");
        m.insert("pointerup", "PointerEvent");

        m.insert("touchcancel", "TouchEvent");
        m.insert("touchend", "TouchEvent");

        m.insert("transitioncancel", "TransitionEvent");
        m.insert("transitionend", "TransitionEvent");
        m.insert("transitionrun", "TransitionEvent");
        m.insert("transitionstart", "TransitionEvent");
        m
    };
}

pub(crate) fn render_view(cx: &Ident, nodes: &[Node], mode: Mode) -> TokenStream {
    if nodes.is_empty() {
        let span = Span::call_site();
        quote_spanned! {
            span => leptos::Unit.into_node(#cx)
        }
    }
    else if nodes.len() == 1 {
        node_to_tokens(cx, &nodes[0], mode)
    } else {
        fragment_to_tokens(cx, Span::call_site(), nodes, mode)
    }
}

fn fragment_to_tokens(cx: &Ident, span: Span, nodes: &[Node], mode: Mode) -> TokenStream {
    let nodes = nodes
            .iter()
            .map(|node| {
                let node  = node_to_tokens(cx, node, mode);
                let span = node.span();
                quote_spanned! {
                    span => #node.into_node(#cx),
                }
            });
    quote_spanned! {
        span => {
            vec![
                #(#nodes)*
            ].into_node(#cx)
        }
    }
}

fn node_to_tokens(cx: &Ident, node: &Node, mode: Mode) -> TokenStream {
    match node {
        Node::Fragment(fragment) => {
            fragment_to_tokens(cx, Span::call_site(), &fragment.children, mode)
        },
        Node::Comment(_) | Node::Doctype(_) => quote! { },
        Node::Text(node) => {
            let span = node.value.span();
            let value = node.value.as_ref();
            quote_spanned! {
                span => text(#value)
            }
        },
        Node::Block(node) => {
            let span = node.value.span();
            let value = node.value.as_ref();
            quote_spanned! {
                span => #value.into_node(#cx)
            }
        },
        Node::Attribute(node) => attribute_to_tokens(cx, node, mode),
        Node::Element(node) => element_to_tokens(cx, node, mode),
    }
}

fn element_to_tokens(cx: &Ident, node: &NodeElement, mode: Mode) -> TokenStream {
    let span = node.name.span();
    if is_component_node(node) {
        component_to_tokens(cx, node, mode)
    } else {
        let name = if is_custom_element(&node.name) {
            let name = node.name.to_string();
            quote_spanned! { span => custom(#cx, #name) }
        } else {
            let name = &node.name;
            quote_spanned! { span => #name(#cx) }
        };
        let attrs = node.attributes.iter().filter_map(|node| {
            if let Node::Attribute(node) = node {
                Some(attribute_to_tokens(cx, node, mode))
            } else {
                None
            }
        });
        let children = node.children.iter().map(|node| {
            let child = match node {
                Node::Fragment(fragment) => {
                    fragment_to_tokens(cx, Span::call_site(), &fragment.children, mode)
                },
                Node::Text(node) => {
                    let span = node.value.span();
                    let value = node.value.as_ref();
                    quote_spanned! {
                        span => #[allow(unused_braces)] #value
                    }
                },
                Node::Block(node) => {
                    let span = node.value.span();
                    let value = node.value.as_ref();
                    quote_spanned! {
                        span => #[allow(unused_braces)] #value
                    }
                },
                Node::Element(node) => element_to_tokens(cx, node, mode),
                Node::Comment(_) | Node::Doctype(_) | Node::Attribute(_) => quote! { },
            };
            quote! {
                ._child(cx, #child)
            }
        });
        quote_spanned! {
            span => #name
                #(#attrs)*
                #(#children)*
                .into_node(#cx)
        }
    }
}

fn attribute_to_tokens(cx: &Ident, node: &NodeAttribute, mode: Mode) -> TokenStream {
    let span = node.key.span();
    let name = node.key.to_string();
    if name == "ref" || name == "_ref" {
        if mode != Mode::Ssr {
            let value = node.value.as_ref().and_then(|expr| expr_to_ident(expr)).expect("'_ref' needs to be passed a variable name");
            quote_spanned! {
                span => ._ref(#[allow(unused_braces)] #value)
            }
        } else {
            todo!()
        }
    } else if let Some(name) = name.strip_prefix("on:") {
        if mode != Mode::Ssr {
            let handler = node
                .value
                .as_ref()
                .expect("event listener attributes need a value")
                .as_ref();
            let event_type = EVENTS.get(&name).copied().unwrap_or("Event");
            let event_type = event_type.parse::<TokenStream>().expect("couldn't parse event name");

            if NON_BUBBLING_EVENTS.contains(&name) {
                quote_spanned! {
                    span => .on::<leptos::web_sys::#event_type>(#name, #[allow(unused_braces)] #handler)
                }
            } else {
                quote_spanned! {
                    span => .on_delegated::<leptos::web_sys::#event_type>(#name, #[allow(unused_braces)] #handler)
                }
            }
        } else {
            todo!()
        }
    } else if let Some(name) = name.strip_prefix("prop:") {
        let value = node.value.as_ref().expect("prop: attributes need a value").as_ref();
        if mode != Mode::Ssr {
            quote_spanned! {
                span => ._prop(#cx, #name, #[allow(unused_braces)] #value)
            }
        } else {
            todo!()
        }
    } else if let Some(name) = name.strip_prefix("class:") {
        let value = node.value.as_ref().expect("class: attributes need a value").as_ref();
        if mode != Mode::Ssr {
            quote_spanned! {
                span => ._class(#cx, #name, #[allow(unused_braces)] #value)
            }
        } else {
            todo!()
        }
    } else {
        let name = name.replacen("attr:", "", 1);
        let value = match node.value.as_ref() {
            Some(value) => {
                let value = value.as_ref();
                let span = value.span();
                quote_spanned! { span => Some(#value) }
            },
            None => quote! { None }
        };
        if mode != Mode::Ssr {
            quote_spanned! {
                span => ._attr(#cx, #name, #[allow(unused_braces)] #value)
            }
        } else {
            quote! { }
        }
    }
}

fn component_to_tokens(cx: &Ident, node: &NodeElement, mode: Mode) -> TokenStream {
    let name = &node.name;
    let component_name = ident_from_tag_name(&node.name);
    let component_name_str = name.to_string();
    let span = node.name.span();
    let component_props_name = Ident::new(&format!("{component_name}Props"), span);

    let children = if node.children.is_empty() {
        quote! {}
    } else if node.children.len() == 1 {
        let child = component_child(cx, &node.children[0], mode);
        quote_spanned! { span => .children(Box::new(move || vec![#child])) }
    } else {
        let children = node.children.iter()
            .map(|node| component_child(cx, node, mode));
        quote_spanned! { span => .children(Box::new(move || vec![#(#children),*])) }
    };

    let props = node.attributes.iter()
        .filter_map(|node| if let Node::Attribute(node) = node { Some(node) } else { None })
        .map(|attr| {
                let name = &attr.key;
                let span = attr.key.span();
                let value = attr
                    .value
                    .as_ref()
                    .map(|v| {
                        let v = v.as_ref();
                        quote_spanned! { span => #v }
                    })
                    .unwrap_or_else(|| quote_spanned! { span => #name });

            quote_spanned! {
                span => .#name(#[allow(unused_braces)] #value)
            }
        });

    let component_itself = quote_spanned! { 
        span => #name(
            cx,
            #component_props_name::builder()
                #(#props)*
                #children
                .build(),
        )
    };
    
    quote_spanned! {
        span => leptos::Component::new(
            #component_name_str,
            move |cx| #component_itself
        )
    }
}

fn component_child(cx: &Ident, node: &Node, mode: Mode) -> TokenStream {
    match node {
        Node::Block(node) => {
            let span = node.value.span();
            let value = node.value.as_ref();
            quote_spanned! {
                span => #value
            }
        },
        _ => node_to_tokens(cx, node, mode)
    }
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
        NodeName::Block(_) => panic!("blocks not allowed in tag-name position"),
        _ => Ident::new(
            &tag_name.to_string().replace(['-', ':'], "_"),
            tag_name.span(),
        ),
    }
}

fn expr_to_ident(expr: &syn::Expr) -> Option<&ExprPath> {
    match expr {
        syn::Expr::Block(block) => block.block.stmts.last().and_then(|stmt| {
            if let syn::Stmt::Expr(expr) = stmt {
                expr_to_ident(expr)
            } else {
                None
            }
        }),
        syn::Expr::Path(path) => Some(path),
        _ => None,
    }
}

fn is_custom_element(name: &NodeName) -> bool {
    name.to_string().contains('-')
}