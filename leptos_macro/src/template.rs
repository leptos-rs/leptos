use crate::attribute_value;
use leptos_hot_reload::parsing::is_component_node;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{spanned::Spanned, ExprBlock};
use rstml::node::{Node, NodeAttribute, NodeElement, KeyedAttribute, NodeBlock};
use uuid::Uuid;

pub(crate) fn render_template(cx: &Ident, nodes: &[Node]) -> TokenStream {
    let template_uid = Ident::new(
        &format!("TEMPLATE_{}", Uuid::new_v4().simple()),
        Span::call_site(),
    );

    match nodes.first() {
        Some(Node::Element(node)) => {
            root_element_to_tokens(cx, &template_uid, node)
        }
        _ => abort!(cx, "template! takes a single root element."),
    }
}

fn root_element_to_tokens(
    cx: &Ident,
    template_uid: &Ident,
    node: &NodeElement,
) -> TokenStream {
    let mut template = String::new();
    let mut navigations = Vec::new();
    let mut expressions = Vec::new();

    if is_component_node(node) {
        crate::view::component_to_tokens(cx, node, None)
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
        );

        // create the root element from which navigations and expressions will begin
        let generate_root = quote! {
            let root = #template_uid.with(|tpl| tpl.content().clone_node_with_deep(true))
                .unwrap()
                .first_child()
                .unwrap();
        };

        let span = node.name().span();

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

        let tag_name = node.name().to_string();

        quote_spanned! {
            span => {
                thread_local! {
                    static #template_uid: web_sys::HtmlTemplateElement = {
                        let document = leptos::document();
                        let el = document.create_element("template").unwrap();
                        el.set_inner_html(#template);
                        el.unchecked_into()
                    };
                }

                #generate_root

                #navigations
                #expressions

                leptos::leptos_dom::View::Element(leptos::leptos_dom::Element {
                    #[cfg(debug_assertions)]
                    name: #tag_name.into(),
                    element: root.unchecked_into(),
                    #[cfg(debug_assertions)]
                    view_marker: None
                })
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

fn attributes(node: &NodeElement) -> impl Iterator<Item = &KeyedAttribute> {
    node.attributes().iter().filter_map(|node| {
        if let NodeAttribute::Attribute(attribute) = node {
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
) -> Ident {
    // create this element
    *next_el_id += 1;
    let this_el_ident = child_ident(*next_el_id, node.name().span());

    // Open tag
    let name_str = node.name().to_string();
    let span = node.name().span();

    // CSR/hydrate, push to template
    template.push('<');
    template.push_str(&name_str);

    // attributes
    for attr in attributes(node) {
        attr_to_tokens(cx, attr, &this_el_ident, template, expressions);
    }

    // navigation for this el
    let debug_name = node.name().to_string();
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
                let #this_el_ident = #prev_sib.next_sibling().unwrap_or_else(|| panic!("error : {} => {} ", #debug_name, "nextSibling"));
                //log::debug!("=> got {}", #this_el_ident.node_name());
        }
    } else {
        quote_spanned! {
            span => let #this_el_ident = #debug_name;
                //log::debug!("first_child ({})", #debug_name);
                let #this_el_ident = #parent.first_child().unwrap_or_else(|| panic!("error: {} => {}", #debug_name, "firstChild"));
                //log::debug!("=> got {}", #this_el_ident.node_name());
        }
    };
    navigations.push(this_nav);

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
        template.push_str("/>");
        return this_el_ident;
    } else {
        template.push('>');
    }

    // iterate over children
    let mut prev_sib = prev_sib;
    for (idx, child) in node.children.iter().enumerate() {
        // set next sib (for any insertions)
        let next_sib =
            match next_sibling_node(&node.children, idx + 1, next_el_id) {
                Ok(next_sib) => next_sib,
                Err(err) => abort!(span, "{}", err),
            };

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
        );

        prev_sib = match curr_id {
            PrevSibChange::Sib(id) => Some(id),
            PrevSibChange::Parent => None,
            PrevSibChange::Skip => prev_sib,
        };
    }

    // close tag
    template.push_str("</");
    template.push_str(&name_str);
    template.push('>');

    this_el_ident
}

fn next_sibling_node(
    children: &[Node],
    idx: usize,
    next_el_id: &mut usize,
) -> Result<Option<Ident>, String> {
    if children.len() <= idx {
        Ok(None)
    } else {
        let sibling = &children[idx];

        match sibling {
            Node::Element(sibling) => {
                if is_component_node(sibling) {
                    next_sibling_node(children, idx + 1, next_el_id)
                } else {
                    Ok(Some(child_ident(*next_el_id + 1, sibling.name().span())))
                }
            }
            Node::Block(sibling) => {
                Ok(Some(child_ident(*next_el_id + 1, sibling.span())))
            }
            Node::Text(sibling) => {
                Ok(Some(child_ident(*next_el_id + 1, sibling.span())))
            }
            _ => Err("expected either an element or a block".to_string()),
        }
    }
}

fn attr_to_tokens(
    cx: &Ident,
    node: &KeyedAttribute,
    el_id: &Ident,
    template: &mut String,
    expressions: &mut Vec<TokenStream>,
) {
    let name = node.key.to_string();
    let name = name.strip_prefix('_').unwrap_or(&name);
    let name = name.strip_prefix("attr:").unwrap_or(name);

    let value = match &node.value() {
        Some(expr) => match expr {
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
        abort!(span, "node_ref not yet supported in template! macro")
    }
    // Event Handlers
    else if name.starts_with("on:") {
        let (event_type, handler) =
            crate::view::event_from_attribute_node(node, false);
        expressions.push(quote! {
            leptos::leptos_dom::add_event_helper(#el_id.unchecked_ref(), #event_type, #handler);
        })
    }
    // Properties
    else if let Some(name) = name.strip_prefix("prop:") {
        let value = attribute_value(node);

        expressions.push(quote_spanned! {
            span => leptos_dom::property(#cx, #el_id.unchecked_ref(), #name, #value.into_property(#cx))
        });
    }
    // Classes
    else if let Some(name) = name.strip_prefix("class:") {
        let value = attribute_value(node);

        expressions.push(quote_spanned! {
            span => leptos::leptos_dom::class_helper(#el_id.unchecked_ref(), #name.into(), #value.into_class(#cx))
        });
    }
    // Attributes
    else {
        match value {
            AttributeValue::Empty => {
                template.push(' ');
                template.push_str(name);
            }

            // Static attributes (i.e., just a literal given as value, not an expression)
            // are just set in the template — again, nothing programmatic
            AttributeValue::Static(value) => {
                template.push(' ');
                template.push_str(name);
                template.push_str("=\"");
                template.push_str(&value);
                template.push('"');
            }
            AttributeValue::Dynamic(value) => {
                // For client-side rendering, dynamic attributes don't need to be rendered in the template
                // They'll immediately be set synchronously before the cloned template is mounted
                expressions.push(quote_spanned! {
                    span => leptos::leptos_dom::attribute_helper(#el_id.unchecked_ref(), #name.into(), {#value}.into_attribute(#cx))
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
) -> PrevSibChange {
    match node {
        Node::Element(node) => {
            if is_component_node(node) {
                proc_macro_error::emit_error!(
                    node.name().span(),
                    "component children not allowed in template!, use view! \
                     instead"
                );
                PrevSibChange::Skip
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
                ))
            }
        }
        //TODO Should we ewerywhere use syn::expr? 
        Node::Text(node) => block_to_tokens(
            cx,
            &syn::ExprLit{
                attrs: vec![],
                lit: node.value.clone().into()
            }.into(),
            node.value.span(),
            parent,
            prev_sib,
            next_sib,
            next_el_id,
            template,
            expressions,
            navigations,
        ),
        Node::Block(NodeBlock::ValidBlock(b)) => block_to_tokens(
            cx,
            &ExprBlock {
                attrs: vec![],
                label: None,
                block: b.clone()
            }.into(),
            b.span(),
            parent,
            prev_sib,
            next_sib,
            next_el_id,
            template,
            expressions,
            navigations,
        ),

        // TODO: Do we need to handle invalid blocks?
        Node::Block(b @ NodeBlock::Invalid{..}) => block_to_tokens(
            cx,
            &syn::Expr::Verbatim(b.to_token_stream()),
            b.span(),
            parent,
            prev_sib,
            next_sib,
            next_el_id,
            template,
            expressions,
            navigations,
        ),
        _ => abort!(cx, "unexpected child node type"),
    }
}

#[allow(clippy::too_many_arguments)]
fn block_to_tokens(
    _cx: &Ident,
    value: &syn::Expr,
    span: Span,
    parent: &Ident,
    prev_sib: Option<Ident>,
    next_sib: Option<Ident>,
    next_el_id: &mut usize,
    template: &mut String,
    expressions: &mut Vec<TokenStream>,
    navigations: &mut Vec<TokenStream>,
) -> PrevSibChange {
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
                        let #name = #sibling.next_sibling().unwrap_or_else(|| panic!("error : {} => {} ", "{block}", "nextSibling"));
                        //log::debug!("\tnext sibling = {}", #name.node_name());
            }
        } else {
            quote_spanned! {
                span => //log::debug!("\\|/ first child on {}", #parent.node_name());
                        let #name = #parent.first_child().unwrap_or_else(|| panic!("error : {} => {} ", "{block}", "firstChild"));
                        //log::debug!("\tfirst child = {}", #name.node_name());
            }
        };
        (Some(name), location)
    };

    let mount_kind = match &next_sib {
        Some(child) => {
            quote! { leptos::leptos_dom::MountKind::Before(&#child.clone()) }
        }
        None => {
            quote! { leptos::leptos_dom::MountKind::Append(&#parent) }
        }
    };

    if let Some(v) = str_value {
        navigations.push(location);
        template.push_str(&v);

        if let Some(name) = name {
            PrevSibChange::Sib(name)
        } else {
            PrevSibChange::Parent
        }
    } else {
        template.push_str("<!>");
        navigations.push(location);

        expressions.push(quote! {
			leptos::leptos_dom::mount_child(#mount_kind, &{#value}.into_view(cx));
        });

        if let Some(name) = name {
            PrevSibChange::Sib(name)
        } else {
            PrevSibChange::Parent
        }
    }
}

fn child_ident(el_id: usize, span: Span) -> Ident {
    let id = format!("_el{el_id}");
    Ident::new(&id, span)
}
