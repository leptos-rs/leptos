use std::collections::HashMap;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, ExprPath};
use syn_rsx::{Node, NodeAttribute, NodeElement, NodeName, NodeValueExpr};
use uuid::Uuid;

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
    let template_uid = Ident::new(
        &format!("TEMPLATE_{}", Uuid::new_v4().simple()),
        Span::call_site(),
    );

    if nodes.len() == 1 {
        first_node_to_tokens(cx, &template_uid, &nodes[0], mode)
    } else {
        let nodes = nodes
            .iter()
            .map(|node| first_node_to_tokens(cx, &template_uid, node, mode));
        quote! {
            {
                vec![
                    #(#nodes),*
                ]
            }
        }
    }
}

fn first_node_to_tokens(cx: &Ident, template_uid: &Ident, node: &Node, mode: Mode) -> TokenStream {
    match node {
        Node::Doctype(_) | Node::Comment(_) => quote! {},
        Node::Fragment(node) => {
            let nodes = node
                .children
                .iter()
                .map(|node| first_node_to_tokens(cx, template_uid, node, mode));
            quote! {
                {
                    vec![
                        #(#nodes),*
                    ]
                }
            }
        }
        Node::Element(node) => root_element_to_tokens(cx, template_uid, node, mode),
        Node::Block(node) => {
            let value = node.value.as_ref();
            quote! {
                #value
            }
        }
        Node::Text(node) => {
            let value = node.value.as_ref();
            quote! {
                #value
            }
        }
        _ => panic!("Root nodes need to be a Fragment (<></>), Element, or text."),
    }
}

fn root_element_to_tokens(
    cx: &Ident,
    template_uid: &Ident,
    node: &NodeElement,
    mode: Mode,
) -> TokenStream {
    let mut template = String::new();
    let mut navigations = Vec::new();
    let mut expressions = Vec::new();

    if is_component_node(node) {
        create_component(cx, node, mode)
    } else {
        element_to_tokens(
            cx,
            node,
            &Ident::new("root", Span::call_site()),
            None,
            &mut 0,
            &mut 0,
            &mut template,
            &mut navigations,
            &mut expressions,
            true,
            mode,
        );

        match mode {
            Mode::Ssr => {
                quote! {{
                    #(#navigations);*;

                    let mut leptos_buffer = String::new();
                    #(#expressions);*
                    leptos_buffer
                }}
            }
            _ => {
                // create the root element from which navigations and expressions will begin
                let generate_root = match mode {
                    // SSR is just going to return a format string, so no root/navigations
                    Mode::Ssr => unreachable!(),
                    // for CSR, just clone the template and take the first child as the root
                    Mode::Client => quote! {
                        let root = #template_uid.with(|template| leptos_dom::clone_template(template));
                    },
                    // for hydration, use get_next_element(), which will either draw from an SSRed node or clone the template
                    Mode::Hydrate => {
                        //let name = node.name_as_string().unwrap();
                        quote! {
                            let root = #template_uid.with(|template| #cx.get_next_element(template));
                            // //log::debug!("root = {}", root.node_name());
                        }
                    }
                };

                let span = node.name.span();

                let navigations = if navigations.is_empty() {
                    quote! {}
                } else {
                    quote! { #(#navigations);* }
                };

                let expressions = if expressions.is_empty() {
                    quote! {}
                } else {
                    quote! { #(#expressions;);* }
                };

                quote_spanned! {
                    span => {
                        thread_local! {
                            static #template_uid: web_sys::HtmlTemplateElement = leptos_dom::create_template(#template)
                        }

                        #generate_root

                        #navigations
                        #expressions

                        root
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
enum PrevSibChange {
    Sib(Ident),
    Parent,
    Skip,
}

fn attributes(node: &NodeElement) -> impl Iterator<Item = &NodeAttribute> {
    node.attributes.iter().filter_map(|node| {
        if let Node::Attribute(attribute) = node {
            Some(attribute)
        } else {
            None
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn element_to_tokens(
    cx: &Ident,
    node: &NodeElement,
    parent: &Ident,
    prev_sib: Option<Ident>,
    next_el_id: &mut usize,
    next_co_id: &mut usize,
    template: &mut String,
    navigations: &mut Vec<TokenStream>,
    expressions: &mut Vec<TokenStream>,
    is_root_el: bool,
    mode: Mode,
) -> Ident {
    // create this element
    *next_el_id += 1;
    let this_el_ident = child_ident(*next_el_id, node.name.span());

    // Open tag
    let name_str = node.name.to_string();
    let span = node.name.span();

    if mode == Mode::Ssr {
        // SSR, push directly to buffer
        expressions.push(quote::quote_spanned! {
            span => leptos_buffer.push('<');
                    leptos_buffer.push_str(#name_str);
        });
    } else {
        // CSR/hydrate, push to template
        template.push('<');
        template.push_str(&name_str);
    }

    // for SSR: add a hydration key
    if mode == Mode::Ssr && is_root_el {
        expressions.push(quote::quote_spanned! {
            span => leptos_buffer.push_str(" data-hk=\"");
                    leptos_buffer.push_str(&#cx.next_hydration_key().to_string());
                    leptos_buffer.push('"');
        });
    }

    // for SSR: merge all class: attributes and class attribute
    if mode == Mode::Ssr {
        let class_attr = attributes(node)
            .find(|a| a.key.to_string() == "class")
            .map(|node| {
                (
                    node.key.span(),
                    node.value
                        .as_ref()
                        .and_then(|n| String::try_from(n).ok())
                        .unwrap_or_default()
                        .trim()
                        .to_string(),
                )
            });

        let class_attrs = attributes(node).filter_map(|node| {
                let name = node.key.to_string();
                if name.starts_with("class:") || name.starts_with("class-") {
                    let name = if name.starts_with("class:") {
                        name.replacen("class:", "", 1)
                    } else if name.starts_with("class-") {
                        name.replacen("class-", "", 1)
                    } else {
                        name
                    };
                    let value = node.value.as_ref().expect("class: attributes need values").as_ref();
                    let span = node.key.span();
                    Some(quote_spanned! { 
                        span => leptos_buffer.push(' ');
                            leptos_buffer.push_str(&{#value}.into_class(#cx).as_value_string(#name));
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if class_attr.is_some() || !class_attrs.is_empty() {
            expressions.push(quote::quote_spanned! {
                span => leptos_buffer.push_str(" class=\"");
            });
            if let Some((span, value)) = class_attr {
                expressions.push(quote::quote_spanned! {
                    span => leptos_buffer.push_str(#value);
                });
            }
            for attr in class_attrs {
                expressions.push(attr);
            }
            expressions.push(quote::quote_spanned! {
                span => leptos_buffer.push('"');
            });
        }
    }

    // attributes
    for attr in attributes(node) {
        // SSR class attribute has just been handled
        if !(mode == Mode::Ssr && attr.key.to_string() == "class") {
            attr_to_tokens(
                cx,
                attr,
                &this_el_ident,
                template,
                expressions,
                mode,
            );
        }
    }

    // navigation for this el
    let debug_name = node.name.to_string();
    if mode != Mode::Ssr {
        let this_nav = if is_root_el {
            quote_spanned! {
                span => let #this_el_ident = #debug_name;
                    let #this_el_ident = #parent.clone().unchecked_into::<web_sys::Node>();
                    //debug!("=> got {}", #this_el_ident.node_name());
            }
        } else if let Some(prev_sib) = &prev_sib {
            quote_spanned! {
                span => let #this_el_ident = #debug_name;
                    //log::debug!("next_sibling ({})", #debug_name);
                    let #this_el_ident = #prev_sib.next_sibling().unwrap_or_else(|| ::leptos::__leptos_renderer_error(#debug_name, "nextSibling"));
                    //log::debug!("=> got {}", #this_el_ident.node_name());
            }
        } else {
            quote_spanned! {
                span => let #this_el_ident = #debug_name;
                    //log::debug!("first_child ({})", #debug_name);
                    let #this_el_ident = #parent.first_child().unwrap_or_else(|| ::leptos::__leptos_renderer_error(#debug_name, "firstChild"));
                    //log::debug!("=> got {}", #this_el_ident.node_name());
            }
        };
        navigations.push(this_nav);
    }

    // self-closing tags
    // https://developer.mozilla.org/en-US/docs/Glossary/Empty_element
    if matches!(
        name_str.as_str(),
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    ) {
        if mode == Mode::Ssr {
            expressions.push(quote::quote! {
                leptos_buffer.push_str("/>");
            });
        } else {
            template.push_str("/>");
        }
        return this_el_ident;
    } else if mode == Mode::Ssr {
        expressions.push(quote::quote! {
            leptos_buffer.push('>');
        });
    } else {
        template.push('>');
    }

    // iterate over children
    let mut prev_sib = prev_sib;
    let multi = !node.children.is_empty();
    for (idx, child) in node.children.iter().enumerate() {
        // set next sib (for any insertions)
        let next_sib = next_sibling_node(&node.children, idx + 1, next_el_id);

        let curr_id = child_to_tokens(
            cx,
            child,
            &this_el_ident,
            if idx == 0 { None } else { prev_sib.clone() },
            next_sib,
            next_el_id,
            next_co_id,
            template,
            navigations,
            expressions,
            multi,
            mode,
            idx == 0,
        );

        prev_sib = match curr_id {
            PrevSibChange::Sib(id) => Some(id),
            PrevSibChange::Parent => None,
            PrevSibChange::Skip => prev_sib,
        };
    }

    // close tag
    if mode == Mode::Ssr {
        expressions.push(quote::quote! {
            leptos_buffer.push_str("</");
            leptos_buffer.push_str(#name_str);
            leptos_buffer.push('>');
        })
    } else {
        template.push_str("</");
        template.push_str(&name_str);
        template.push('>');
    }

    this_el_ident
}

fn next_sibling_node(children: &[Node], idx: usize, next_el_id: &mut usize) -> Option<Ident> {
    if children.len() <= idx {
        None
    } else {
        let sibling = &children[idx];

        match sibling {
            Node::Element(sibling) => {
                if is_component_node(sibling) {
                    next_sibling_node(children, idx + 1, next_el_id)
                } else {
                    Some(child_ident(*next_el_id + 1, sibling.name.span()))
                }
            }
            Node::Block(sibling) => Some(child_ident(*next_el_id + 1, sibling.value.span())),
            Node::Text(sibling) => Some(child_ident(*next_el_id + 1, sibling.value.span())),
            _ => panic!("expected either an element or a block"),
        }
    }
}

fn attr_to_tokens(
    cx: &Ident,
    node: &NodeAttribute,
    el_id: &Ident,
    template: &mut String,
    expressions: &mut Vec<TokenStream>,
    mode: Mode,
) {
    let name = node.key.to_string();
    let name = if name.starts_with('_') {
        name.replacen('_', "", 1)
    } else {
        name
    };
    let name = if name.starts_with("attr:") {
        name.replacen("attr:", "", 1)
    } else {
        name
    };
    let value = match &node.value {
        Some(expr) => match expr.as_ref() {
            syn::Expr::Lit(expr_lit) => {
                if let syn::Lit::Str(s) = &expr_lit.lit {
                    AttributeValue::Static(s.value())
                } else {
                    AttributeValue::Dynamic(expr)
                }
            }
            _ => AttributeValue::Dynamic(expr),
        },
        None => AttributeValue::Empty,
    };

    let span = node.key.span();

    // refs
    if name == "ref" {
        if mode != Mode::Ssr {
            expressions.push(match &node.value {
                Some(expr) => {
                    if let Some(ident) = expr_to_ident(expr) {
                        quote_spanned! {
                            span =>
                                #ident.load(#el_id.unchecked_ref::<web_sys::Element>());
                        }
                    } else {
                        panic!("'ref' needs to be passed a variable name")
                    }
                }
                _ => panic!("'ref' needs to be passed a variable name"),
            })
        }
    }
    // Event Handlers
    else if name.starts_with("on:") {
        let handler = node
            .value
            .as_ref()
            .expect("event listener attributes need a value")
            .as_ref();

        let name = name.replacen("on:", "", 1);
        let event_type = EVENTS.get(&name.as_str()).copied().unwrap_or("Event");
        let event_type = event_type.parse::<TokenStream>().expect("couldn't parse event name");

        if mode != Mode::Ssr {
            cfg_if::cfg_if! {
                if #[cfg(feature = "stable")] {
                    if NON_BUBBLING_EVENTS.contains(&name.as_str()) {
                        expressions.push(quote_spanned! {
                            span => ::leptos::add_event_listener_undelegated(#el_id.unchecked_ref(), #name, #handler);
                        });
                    } else {
                        expressions.push(quote_spanned! {
                            span => ::leptos::add_event_listener(#el_id.unchecked_ref(), #name, #handler);
                        });
                    }
                } else {
                    if NON_BUBBLING_EVENTS.contains(&name.as_str()) {
                        expressions.push(quote_spanned! {
                            span => ::leptos::add_event_listener_undelegated::<web_sys::#event_type>(#el_id.unchecked_ref(), #name, #handler);
                        });
                    } else {
                        expressions.push(quote_spanned! {
                            span => ::leptos::add_event_listener::<web_sys::#event_type>(#el_id.unchecked_ref(), #name, #handler);
                        });
                    }
                }
            }
        } else {
            
            // this is here to avoid warnings about unused signals
            // that are used in event listeners. I'm open to better solutions.
            expressions.push(quote_spanned! {
                span => let _  = ssr_event_listener::<web_sys::#event_type>(#handler);
            });
        }
    }
    // Properties
    else if name.starts_with("prop:") {
        let name = name.replacen("prop:", "", 1);
        // can't set properties in SSR
        if mode != Mode::Ssr {
            let value = node
                .value
                .as_ref()
                .expect("prop: blocks need values")
                .as_ref();
            expressions.push(quote_spanned! {
                span => leptos_dom::property(#cx, #el_id.unchecked_ref(), #name, #value.into_property(#cx))
            });
        }
    }
    // Classes
    else if name.starts_with("class:") {
        let name = name.replacen("class:", "", 1);
        if mode == Mode::Ssr {
            // handled separately because they need to be merged
        } else {
            let value = node
                .value
                .as_ref()
                .expect("class: attributes need values")
                .as_ref();
            expressions.push(quote_spanned! {
                span => leptos_dom::class(#cx, #el_id.unchecked_ref(), #name, #value.into_class(#cx))
            });
        }
    }
    // Attributes
    else {
        match (value, mode) {
            // Boolean attributes: only name present in template, no value
            // Nothing set programmatically
            (AttributeValue::Empty, Mode::Ssr) => {
                expressions.push(quote::quote_spanned! {
                    span => leptos_buffer.push(' ');
                            leptos_buffer.push_str(#name);
                });
            }
            (AttributeValue::Empty, _) => {
                template.push(' ');
                template.push_str(&name);
            }

            // Static attributes (i.e., just a literal given as value, not an expression)
            // are just set in the template — again, nothing programmatic
            (AttributeValue::Static(value), Mode::Ssr) => {
                expressions.push(quote::quote_spanned! {
                    span => leptos_buffer.push(' ');
                            leptos_buffer.push_str(#name);
                            leptos_buffer.push_str("=\"");
                            leptos_buffer.push_str(&leptos_dom::escape_attr(&#value));
                            leptos_buffer.push('"');
                });
            }
            (AttributeValue::Static(value), _) => {
                template.push(' ');
                template.push_str(&name);
                template.push_str("=\"");
                template.push_str(&value);
                template.push('"');
            }

            // Dynamic attributes are handled differently depending on the rendering mode
            (AttributeValue::Dynamic(value), Mode::Ssr) => {
                expressions.push(quote_spanned! {
                    span => leptos_buffer.push(' ');
                            leptos_buffer.push_str(&{#value}.into_attribute(#cx).as_value_string(#name));
                });
            }
            (AttributeValue::Dynamic(value), _) => {
                // For client-side rendering, dynamic attributes don't need to be rendered in the template
                // They'll immediately be set synchronously before the cloned template is mounted
                expressions.push(quote_spanned! {
                    span => leptos_dom::attribute(#cx, #el_id.unchecked_ref(), #name, {#value}.into_attribute(#cx))
                });
            }
        }
    }
}

enum AttributeValue<'a> {
    Static(String),
    Dynamic(&'a syn::Expr),
    Empty,
}

#[allow(clippy::too_many_arguments)]
fn child_to_tokens(
    cx: &Ident,
    node: &Node,
    parent: &Ident,
    prev_sib: Option<Ident>,
    next_sib: Option<Ident>,
    next_el_id: &mut usize,
    next_co_id: &mut usize,
    template: &mut String,
    navigations: &mut Vec<TokenStream>,
    expressions: &mut Vec<TokenStream>,
    multi: bool,
    mode: Mode,
    is_first_child: bool,
) -> PrevSibChange {
    match node {
        Node::Element(node) => {
            if is_component_node(node) {
                component_to_tokens(
                    cx,
                    node,
                    Some(parent),
                    prev_sib,
                    next_sib,
                    template,
                    expressions,
                    navigations,
                    next_el_id,
                    next_co_id,
                    multi,
                    mode,
                    is_first_child,
                )
            } else {
                PrevSibChange::Sib(element_to_tokens(
                    cx,
                    node,
                    parent,
                    prev_sib,
                    next_el_id,
                    next_co_id,
                    template,
                    navigations,
                    expressions,
                    false,
                    mode,
                ))
            }
        }
        Node::Text(node) => block_to_tokens(
            cx,
            &node.value,
            node.value.span(),
            parent,
            prev_sib,
            next_sib,
            next_el_id,
            next_co_id,
            template,
            expressions,
            navigations,
            mode,
        ),
        Node::Block(node) => block_to_tokens(
            cx,
            &node.value,
            node.value.span(),
            parent,
            prev_sib,
            next_sib,
            next_el_id,
            next_co_id,
            template,
            expressions,
            navigations,
            mode,
        ),
        _ => panic!("unexpected child node type"),
    }
}

#[allow(clippy::too_many_arguments)]
fn block_to_tokens(
    cx: &Ident,
    value: &NodeValueExpr,
    span: Span,
    parent: &Ident,
    prev_sib: Option<Ident>,
    next_sib: Option<Ident>,
    next_el_id: &mut usize,
    next_co_id: &mut usize,
    template: &mut String,
    expressions: &mut Vec<TokenStream>,
    navigations: &mut Vec<TokenStream>,
    mode: Mode,
) -> PrevSibChange {
    let value = value.as_ref();
    let str_value = match value {
        syn::Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Str(s) => Some(s.value()),
            syn::Lit::Char(c) => Some(c.value().to_string()),
            syn::Lit::Int(i) => Some(i.base10_digits().to_string()),
            syn::Lit::Float(f) => Some(f.base10_digits().to_string()),
            _ => None,
        },
        _ => None,
    };
    let current: Option<Ident> = None;

    // code to navigate to this text node

    let (name, location) = /* if is_first_child && mode == Mode::Client {
        (None, quote! { })
    } 
    else */ {
        *next_el_id += 1;
        let name = child_ident(*next_el_id, span);
        let location = if let Some(sibling) = &prev_sib {
            quote_spanned! {
                span => //log::debug!("-> next sibling");
                        let #name = #sibling.next_sibling().unwrap_or_else(|| ::leptos::__leptos_renderer_error("{block}", "nextSibling"));
                        //log::debug!("\tnext sibling = {}", #name.node_name());
            }
        } else {
            quote_spanned! {
                span => //log::debug!("\\|/ first child on {}", #parent.node_name());
                        let #name = #parent.first_child().unwrap_or_else(|| ::leptos::__leptos_renderer_error("{block}", "firstChild"));
                        //log::debug!("\tfirst child = {}", #name.node_name());
            }
        };
        (Some(name), location)
    };

    let before = match &next_sib {
        Some(child) => quote! { leptos::Marker::BeforeChild(#child.clone()) },
        None => {
            /* if multi {
                quote! { leptos::Marker::LastChild }
            } else {
                quote! { leptos::Marker::LastChild }
            } */
            quote! { leptos::Marker::LastChild }
        }
    };

    if let Some(v) = str_value {
        if mode == Mode::Ssr {
            expressions.push(quote::quote_spanned! {
                span => leptos_buffer.push_str(&leptos_dom::escape_text(&#v));
            });
        } else {
            navigations.push(location);
            template.push_str(&v);
        }

        if let Some(name) = name {
            PrevSibChange::Sib(name)
        } else {
            PrevSibChange::Parent
        }
    } else {
        // these markers are one of the primary templating differences across modes
        match mode {
            // in CSR, simply insert a comment node: it will be picked up and replaced with the value
            Mode::Client => {
                template.push_str("<!>");
                navigations.push(location);

                let current = match current {
                    Some(i) => quote! { Some(#i.into_child(#cx)) },
                    None => quote! { None },
                };
                expressions.push(quote! {
                    leptos::insert(
                        #cx,
                        #parent.clone(),
                        #value.into_child(#cx),
                        #before,
                        #current,
                    );
                });
            }
            // when hydrating, a text node will be generated by SSR; in the hydration/CSR template,
            // wrap it with comments that mark where it begins and ends
            Mode::Hydrate => {
                //*next_el_id += 1;
                let el = child_ident(*next_el_id, span);
                *next_co_id += 1;
                let co = comment_ident(*next_co_id, span);
                //next_sib = Some(el.clone());

                template.push_str("<!#><!/>");
                let end = Ident::new(&format!("{co}_end"), span);

                navigations.push(quote! {
                    #location;
                    let (#el, #co) = #cx.get_next_marker(&#name);
                    let #end = #co.last().cloned().unwrap_or_else(|| #el.next_sibling().unwrap_throw());
                    //log::debug!("get_next_marker => {}", #el.node_name());
                });

                expressions.push(quote! {
                    leptos::insert(
                        #cx,
                        #parent.clone(),
                        #value.into_child(#cx),
                        #before,
                        Some(Child::Nodes(#co)),
                    );
                });

                return PrevSibChange::Sib(end);

                //current = Some(el);
            }
            // in SSR, it needs to insert the value, wrapped in comments
            Mode::Ssr => expressions.push(quote::quote_spanned! {
                span => leptos_buffer.push_str("<!--#-->");
                        leptos_buffer.push_str(&#value.into_child(#cx).as_child_string());
                        leptos_buffer.push_str("<!--/-->");
            }),
        }

        if let Some(name) = name {
            PrevSibChange::Sib(name)
        } else {
            PrevSibChange::Parent
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn component_to_tokens(
    cx: &Ident,
    node: &NodeElement,
    parent: Option<&Ident>,
    prev_sib: Option<Ident>,
    next_sib: Option<Ident>,
    template: &mut String,
    expressions: &mut Vec<TokenStream>,
    navigations: &mut Vec<TokenStream>,
    next_el_id: &mut usize,
    next_co_id: &mut usize,
    multi: bool,
    mode: Mode,
    is_first_child: bool,
) -> PrevSibChange {
    let component_name = ident_from_tag_name(&node.name);
    let component_name = format!("<{component_name}/>");
    let create_component = create_component(cx, node, mode);
    let span = node.name.span();

    let mut current = None;

    if let Some(parent) = parent {
        let before = match &next_sib {
            Some(child) => quote! { leptos::Marker::BeforeChild(#child.clone()) },
            None => {
                if multi {
                    quote! { leptos::Marker::LastChild }
                } else {
                    quote! { leptos::Marker::NoChildren }
                }
            }
        };

        if mode == Mode::Ssr {
            expressions.push(quote::quote_spanned! {
                span => // TODO wrap components but use get_next_element() instead of first_child/next_sibling?
                        leptos_buffer.push_str("<!--#-->");
                        leptos_buffer.push_str(&#create_component.into_child(#cx).as_child_string());
                        leptos_buffer.push_str("<!--/-->");

            });
        } else if mode == Mode::Hydrate {
            //let name = child_ident(*next_el_id, node);
            *next_el_id += 1;
            let el = child_ident(*next_el_id, node.name.span());
            *next_co_id += 1;
            let co = comment_ident(*next_co_id, node.name.span());
            //next_sib = Some(el.clone());

            let starts_at = if let Some(prev_sib) = prev_sib {
                quote::quote! {{
                    //log::debug!("starts_at = next_sibling");
                    #prev_sib.next_sibling().unwrap_or_else(|| ::leptos::__leptos_renderer_error(#component_name, "nextSibling"))
                    //log::debug!("ok starts_at");
                }}
            } else {
                quote::quote! {{
                    //log::debug!("starts_at first_child");
                    #parent.first_child().unwrap_or_else(|| ::leptos::__leptos_renderer_error(#component_name, "firstChild"))
                    //log::debug!("starts_at ok");
                }}
            };

            current = Some(el.clone());

            template.push_str("<!#><!/>");
            navigations.push(quote! {
                let (#el, #co) = #cx.get_next_marker(&#starts_at);
            });

            let before = if next_sib.is_none() {
                quote::quote! { Marker::LastChild }
            } else {
                quote::quote! { Marker::BeforeChild(#el) }
            };

            expressions.push(quote! {
                leptos::insert(
                    #cx,
                    #parent.clone(),
                    #create_component.into_child(#cx),
                    #before,
                    Some(Child::Nodes(#co)),
                );
            });
        } else {
            expressions.push(quote! {
                leptos::insert(
                    #cx,
                    #parent.clone(),
                    #create_component.into_child(#cx),
                    #before,
                    None,
                );
            });
        }
    } else {
        expressions.push(create_component)
    }

    match current {
        Some(el) => PrevSibChange::Sib(el),
        None => {
            if is_first_child {
                PrevSibChange::Parent
            } else {
                PrevSibChange::Skip
            }
        }
    }
}

fn create_component(cx: &Ident, node: &NodeElement, mode: Mode) -> TokenStream {
    let component_name = ident_from_tag_name(&node.name);
    let span = node.name.span();
    let component_props_name = Ident::new(&format!("{component_name}Props"), span);

    let children = if node.children.is_empty() {
        quote! {}
    } else if node.children.len() == 1 {
        let child = render_view(cx, &node.children, mode);
        quote_spanned! { span => .children(Box::new(move || vec![#child])) }
    } else {
        let children = render_view(cx, &node.children, mode);
        quote_spanned! { span => .children(Box::new(move || #children)) }
    };

    let props = attributes(node).filter_map(|attr| {
        let attr_name = attr.key.to_string();
        if attr_name.starts_with("on:")
            || attr_name.starts_with("prop:")
            || attr_name.starts_with("class:")
            || attr_name.starts_with("attr:")
        {
            None
        } else {
            let name = ident_from_tag_name(&attr.key);
            let span = attr.key.span();
            let value = attr
                .value
                .as_ref()
                .map(|v| {
                    let v = v.as_ref();
                    quote_spanned! { span => #v }
                })
                .unwrap_or_else(|| quote_spanned! { span => #name });
            Some(quote_spanned! {
                span => .#name(#value)
            })
        }
    });

    let mut other_attrs = attributes(node).filter_map(|attr| {
        let attr_name = attr.key.to_string();
        let span = attr.key.span();
        let value = attr.value.as_ref().map(|e| e.as_ref());
        // Event Listeners
        if let Some(event_name) = attr_name.strip_prefix("on:") {
            let handler = attr
                .value
                .as_ref()
                .expect("on: event listener attributes need a value")
                .as_ref();
            if NON_BUBBLING_EVENTS.contains(&event_name) {
                Some(quote_spanned! {
                    span => ::leptos::add_event_listener_undelegated(#component_name.unchecked_ref(), #event_name, #handler);
                })
            } else if let Some(event_type) = EVENTS.get(event_name).map(|&e| e.parse::<TokenStream>().unwrap_or_default()) {
                Some(quote_spanned! {
                    span => ::leptos::add_event_listener::<#event_type>(#component_name.unchecked_ref(), #event_name, #handler);
                })
            } else {
                Some(quote_spanned! {
                    span => ::leptos::add_event_listener::<web_sys::Event>(#component_name.unchecked_ref(), #event_name, #handler)
                })
            }
        }
        // Properties
        else if let Some(name) = attr_name.strip_prefix("prop:") {
            Some(quote_spanned! {
                span => leptos_dom::property(#cx, #component_name.unchecked_ref(), #name, #value.into_property(#cx))
            })
        }
        // Classes
        else if let Some(name) = attr_name.strip_prefix("class:") {
            Some(quote_spanned! {
                span => leptos_dom::class(#cx, #component_name.unchecked_ref(), #name, #value.into_class(#cx))
            })
        }
        // Attributes
        else { attr_name.strip_prefix("attr:").map(|name| quote_spanned! {
                span => leptos_dom::attribute(#cx, #component_name.unchecked_ref(), #name, #value.into_attribute(#cx))
            }) }
    }).peekable();

    if other_attrs.peek().is_none() {
        quote_spanned! {
            span => create_component(#cx, move || {
                #component_name(
                    #cx,
                    #component_props_name::builder()
                        #(#props)*
                        #children
                        .build(),
                )
            })
        }
    } else {
        quote_spanned! {
            span => create_component(#cx, move || {
                let #component_name = #component_name(
                    #cx,
                    #component_props_name::builder()
                        #(#props)*
                        #children
                        .build(),
                );
                #(#other_attrs);*;
                #component_name
            })
        }
    }
}

/* fn span(node: &Node) -> Span {
    node.name_span()
        .unwrap_or_else(|| node.value.as_ref().unwrap().span())
} */

fn child_ident(el_id: usize, span: Span) -> Ident {
    let id = format!("_el{el_id}");
    Ident::new(&id, span)
}

fn comment_ident(co_id: usize, span: Span) -> Ident {
    let id = format!("_co{co_id}");
    Ident::new(&id, span)
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
