use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, ExprPath};
use syn_rsx::{Node, NodeName, NodeType};
use uuid::Uuid;

use crate::{is_component_node, Mode};

pub(crate) fn render_view(nodes: &[Node], mode: Mode) -> TokenStream {
    let template_uid = Ident::new(
        &format!("TEMPLATE_{}", Uuid::new_v4().simple()),
        Span::call_site(),
    );

    if nodes.len() == 1 {
        first_node_to_tokens(&template_uid, &nodes[0], mode)
    } else {
        let nodes = nodes
            .iter()
            .map(|node| first_node_to_tokens(&template_uid, node, mode));
        quote! {
            {
                vec![
                    #(#nodes),*
                ]
            }
        }
    }
}

fn first_node_to_tokens(template_uid: &Ident, node: &Node, mode: Mode) -> TokenStream {
    match node.node_type {
        NodeType::Doctype | NodeType::Comment => quote! {},
        NodeType::Fragment => {
            let nodes = node
                .children
                .iter()
                .map(|node| first_node_to_tokens(template_uid, node, mode));
            quote! {
                {
                    vec![
                        #(#nodes),*
                    ]
                }
            }
        }
        NodeType::Element => root_element_to_tokens(template_uid, node, mode),
        NodeType::Block => node
            .value
            .as_ref()
            .map(|value| quote! { #value })
            .expect("root Block node with no value"),
        NodeType::Text => {
            let value = node.value_as_string().unwrap();
            quote! {
                #value
            }
        }
        _ => panic!("Root nodes need to be a Fragment (<></>), Element, or text."),
    }
}

fn root_element_to_tokens(template_uid: &Ident, node: &Node, mode: Mode) -> TokenStream {
    let mut template = String::new();
    let mut navigations = Vec::new();
    let mut expressions = Vec::new();

    if is_component_node(node) {
        create_component(node, mode)
    } else {
        element_to_tokens(
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
                quote! {
                    format!(
                        #template,
                        #(#expressions),*
                    )
                }
            }
            _ => {
                // create the root element from which navigations and expressions will begin
                let generate_root = match mode {
                    // SSR is just going to return a format string, so no root/navigations
                    Mode::Ssr => unreachable!(),
                    // for CSR, just clone the template and take the first child as the root
                    Mode::Client => quote! {
                        let root = #template_uid.with(|template| leptos_dom::clone_template(template)).first_element_child().unwrap_throw();
                    },
                    // for hydration, use get_next_element(), which will either draw from an SSRed node or clone the template
                    Mode::Hydrate => quote! {
                        let root = #template_uid.with(|template| cx.get_next_element(template));
                    },
                };

                quote! {
                    thread_local! {
                        static #template_uid: web_sys::HtmlTemplateElement = leptos_dom::create_template(#template);
                    };

                    #generate_root

                    #(#navigations);*
                    #(#expressions);*;

                    root
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

#[allow(clippy::too_many_arguments)]
fn element_to_tokens(
    node: &Node,
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
    let this_el_ident = child_ident(*next_el_id, node);

    // TEMPLATE: open tag — true for any mode
    let name_str = node.name_as_string().unwrap();
    template.push('<');
    template.push_str(&name_str);

    // for SSR: add a hydration key
    if mode == Mode::Ssr && is_root_el {
        template.push_str(" data-hk=\"{}\"");
        expressions.push(quote! {
            cx.next_hydration_key()
        });
    }

    // attributes
    for attr in &node.attributes {
        attr_to_tokens(attr, &this_el_ident, template, expressions, mode);
    }

    // navigation for this el
    let debug_name = debug_name(node);
    let span = span(node);
    let this_nav = if is_root_el {
        quote_spanned! {
            span => let #this_el_ident = #debug_name;
                let #this_el_ident = #parent.clone().unchecked_into::<web_sys::Node>();
        }
    } else if let Some(prev_sib) = &prev_sib {
        quote_spanned! {
            span => let #this_el_ident = #debug_name;
                let #this_el_ident = #prev_sib.next_sibling().unwrap_throw();
        }
    } else {
        quote_spanned! {
            span => let #this_el_ident = #debug_name;
                let #this_el_ident = #parent.first_child().unwrap_throw();
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
    let multi = node.children.len() >= 2;
    for (idx, child) in node.children.iter().enumerate() {
        // set next sib (for any insertions)
        let next_sib = node.children.get(idx + 1).and_then(|next_sib| {
            if is_component_node(next_sib) {
                None
            } else {
                Some(child_ident(*next_el_id + 1, next_sib))
            }
        });

        let curr_id = child_to_tokens(
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
        );

        prev_sib = match curr_id {
            PrevSibChange::Sib(id) => Some(id),
            PrevSibChange::Parent => None,
            PrevSibChange::Skip => prev_sib,
        };
    }

    // TEMPLATE: close tag (same for any mode)
    template.push_str("</");
    template.push_str(&name_str);
    template.push('>');

    this_el_ident
}

fn attr_to_tokens(
    node: &Node,
    el_id: &Ident,
    template: &mut String,
    expressions: &mut Vec<TokenStream>,
    mode: Mode,
) {
    let name = node
        .name_as_string()
        .expect("Attribute nodes must have strings as names.");
    let name = if name.starts_with('_') {
        name.replacen('_', "", 1)
    } else {
        name
    };
    let value = match &node.value {
        Some(expr) => match expr {
            syn::Expr::Lit(_) => AttributeValue::Static(node.value_as_string().unwrap()),
            _ => AttributeValue::Dynamic(expr),
        },
        None => AttributeValue::Empty,
    };

    let span = node.name_span().unwrap();

    // refs
    if name == "ref" {
        // refs mean nothing in SSR
        if mode != Mode::Ssr {
            expressions.push(match &node.value {
                Some(expr) => {
                    if let Some(ident) = expr_to_ident(expr) {
                        quote_spanned! {
                            span =>
                                // we can't pass by reference because the _el won't live long enough (it's dropped when template returns)
                                // so we clone here; this will be unnecessary if it's the last attribute, but very necessary otherwise
                                #ident = #el_id.clone().unchecked_into::<web_sys::Element>();
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
        if mode != Mode::Ssr {
            let event_name = name.replacen("on:", "", 1);
            let handler = node
                .value
                .as_ref()
                .expect("event listener attributes need a value");
            expressions.push(quote_spanned! {
                span => add_event_listener(#el_id.unchecked_ref(), #event_name, #handler);
            });
        }
    }
    // Properties
    else if name.starts_with("prop:") {
        // can't set properties in SSR
        if mode != Mode::Ssr {
            let name = name.replacen("prop:", "", 1);
            let value = node.value.as_ref().expect("prop: blocks need values");
            expressions.push(quote_spanned! {
            span => leptos_dom::property(cx, #el_id.unchecked_ref(), #name, #value.into_property(cx))
        });
        }
    }
    // Classes
    else if name.starts_with("class:") {
        if mode == Mode::Ssr {
            todo!()
        } else {
            let name = name.replacen("class:", "", 1);
            let value = node.value.as_ref().expect("class: attributes need values");
            expressions.push(quote_spanned! {
                span => leptos_dom::class(cx, #el_id.unchecked_ref(), #name, #value.into_class(cx))
            });
        }
    }
    // Attributes
    else {
        match value {
            // Boolean attributes: only name present in template, no value
            // Nothing set programmatically
            AttributeValue::Empty => {
                template.push(' ');
                template.push_str(&name);
            }

            // Static attributes (i.e., just a literal given as value, not an expression)
            // are just set in the template — again, nothing programmatic
            AttributeValue::Static(value) => {
                template.push(' ');
                template.push_str(&name);
                template.push_str("=\"");
                template.push_str(&value);
                template.push('"');
            }

            // Dynamic attributes are handled differently depending on the rendering mode
            AttributeValue::Dynamic(value) => {
                if mode == Mode::Ssr {
                    template.push_str(" {}");
                    expressions.push(quote_spanned! {
                        span => #value.into_attribute(cx).as_value_string(#name)
                    });
                } else {
                    // For client-side rendering, dynamic attributes don't need to be rendered in the template
                    // They'll immediately be set synchronously before the cloned template is mounted
                    expressions.push(quote_spanned! {
                        span => leptos_dom::attribute(cx, #el_id.unchecked_ref(), #name, #value.into_attribute(cx))
                    });
                }
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
    node: &Node,
    parent: &Ident,
    prev_sib: Option<Ident>,
    mut next_sib: Option<Ident>,
    next_el_id: &mut usize,
    next_co_id: &mut usize,
    template: &mut String,
    navigations: &mut Vec<TokenStream>,
    expressions: &mut Vec<TokenStream>,
    multi: bool,
    mode: Mode,
) -> PrevSibChange {
    match node.node_type {
        NodeType::Element => {
            if is_component_node(node) {
                component_to_tokens(
                    node,
                    Some(parent),
                    next_sib,
                    template,
                    expressions,
                    multi,
                    mode,
                )
            } else {
                PrevSibChange::Sib(element_to_tokens(
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
        NodeType::Text | NodeType::Block => {
            let str_value = node.value.as_ref().and_then(|expr| match expr {
                syn::Expr::Lit(lit) => match &lit.lit {
                    syn::Lit::Str(s) => Some(s.value()),
                    syn::Lit::Char(c) => Some(c.value().to_string()),
                    syn::Lit::Int(i) => Some(i.base10_digits().to_string()),
                    syn::Lit::Float(f) => Some(f.base10_digits().to_string()),
                    _ => None,
                },
                _ => None,
            });
            let mut current: Option<Ident> = None;

            // code to navigate to this text node
            let span = node
                .value
                .as_ref()
                .map(|val| val.span())
                .unwrap_or_else(Span::call_site);

            *next_el_id += 1;
            let name = child_ident(*next_el_id, node);
            let location = if let Some(sibling) = prev_sib {
                quote_spanned! {
                    span => let #name = #sibling.next_sibling().unwrap_throw();
                }
            } else {
                quote_spanned! {
                    span => let #name = #parent.first_child().unwrap_throw();
                }
            };

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

            if let Some(v) = str_value {
                navigations.push(location);
                template.push_str(&v);

                PrevSibChange::Sib(name)
            } else /* if next_sib.is_some() */ {
                // these markers are one of the primary templating differences across modes
                match mode {
                    // in CSR, simply insert a comment node: it will be picked up and replaced with the value
                    Mode::Client => {
                        template.push_str("<!>");
                        navigations.push(location);
                    }
                    // when hydrating, a text node will be generated by SSR; in the hydration/CSR template,
                    // wrap it with comments that mark where it begins and ends
                    Mode::Hydrate => {
                        *next_el_id += 1;
                        let el = child_ident(*next_el_id, node);
                        *next_co_id += 1;
                        let co = comment_ident(*next_co_id, node);
                        next_sib = Some(el.clone());

                        template.push_str("<!#><!/>");
                        navigations.push(quote! {
                            #location;
                            let (#el, #co) = cx.get_next_marker(&#name);
                        });

                        current = Some(co);
                    }
                    // in SSR, it needs to both wrap with comments and insert a hole for an existing value
                    Mode::Ssr => {
                        template.push_str("<!--#-->{}<!--/-->");
                        // TODO push expression here, or elsewhere?
                    }
                }

                let value = node.value_as_block().expect("no block value");

                if mode == Mode::Ssr {
                    expressions.push(quote! {
                        #value.into_child(cx).as_child_string()
                    });
                } else {
                    let current = match current {
                        Some(i) => quote! { Some(#i.into_child(cx)) },
                        None => quote! { None },
                    };
                    expressions.push(quote! {
                        leptos::insert(
                            cx,
                            #parent.clone(),
                            #value.into_child(cx),
                            #before,
                            #current,
                        );
                    });
                }

                PrevSibChange::Sib(name)
            }
        }
        _ => panic!("unexpected child node type"),
    }
}

#[allow(clippy::too_many_arguments)]
fn component_to_tokens(
    node: &Node,
    parent: Option<&Ident>,
    next_sib: Option<Ident>,
    template: &mut String,
    expressions: &mut Vec<TokenStream>,
    multi: bool,
    mode: Mode,
) -> PrevSibChange {
    let create_component = create_component(node, mode);

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
            template.push_str("<!--#-->{}<!--/-->");
            expressions.push(quote! {
                #create_component.into_child(cx).as_child_string()
            });
        } else {
            expressions.push(quote! {
                leptos::insert(
                    cx,
                    #parent.clone(),
                    #create_component.into_child(cx),
                    #before,
                    None,
                );
            });
        }
    } else {
        expressions.push(create_component)
    }

    PrevSibChange::Skip
}

fn create_component(node: &Node, mode: Mode) -> TokenStream {
    let component_name = ident_from_tag_name(node.name.as_ref().unwrap());
    let span = node.name_span().unwrap();
    let component_props_name = Ident::new(&format!("{component_name}Props"), span);

    let children = if node.children.is_empty() {
        quote! {}
    } else if node.children.len() == 1 {
        let child = render_view(&node.children, mode);
        quote! { .children(vec![#child]) }
    } else {
        let children = render_view(&node.children, mode);
        quote! { .children(#children) }
    };

    let props = node.attributes.iter().filter_map(|attr| {
        let attr_name = attr.name_as_string().unwrap_or_default();
        if attr_name.strip_prefix("on:").is_some() {
            None
        } else {
            let name = ident_from_tag_name(attr.name.as_ref().unwrap());
            let value = attr.value.as_ref().expect("component props need values");
            let span = attr.name_span().unwrap();
            Some(quote_spanned! {
                span => .#name(#value)
            })
        }
    });

    let mut other_attrs = node.attributes.iter().filter_map(|attr| {
        let attr_name = attr.name_as_string().unwrap_or_default();
        if let Some(event_name) = attr_name.strip_prefix("on:") {
            let span = attr.name_span().unwrap();
            let handler = attr
                .value
                .as_ref()
                .expect("event listener attributes need a value");
            Some(quote_spanned! {
                span => add_event_listener(#component_name.unchecked_ref(), #event_name, #handler)
            })
        }
        // Properties
        else if let Some(name) = attr_name.strip_prefix("prop:") {
            let value = node.value.as_ref().expect("prop: blocks need values");
            Some(quote_spanned! {
                span => leptos_dom::property(cx, #component_name.unchecked_ref(), #name, #value.into_property(cx))
            })
        }
        // Classes
        else if let Some(name) = attr_name.strip_prefix("class:") {
            let value = node.value.as_ref().expect("class: attributes need values");
            Some(quote_spanned! {
                span => leptos_dom::class(cx, #component_name.unchecked_ref(), #name, #value.into_class(cx))
            })
        }
        else {
            None
        }
    }).peekable();

    // TODO children

    if other_attrs.peek().is_none() {
        quote_spanned! {
            span => create_component(cx, move || {
                #component_name(
                    cx,
                    #component_props_name::builder()
                        #(#props)*
                        #children
                        .build(),
                )
            })
        }
    } else {
        quote_spanned! {
            span => create_component(cx, move || {
                let #component_name = #component_name(
                    cx,
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

fn debug_name(node: &Node) -> String {
    node.name_as_string().unwrap_or_else(|| {
        node.value_as_string()
            .expect("expected either node name or value")
    })
}

fn span(node: &Node) -> Span {
    node.name_span()
        .unwrap_or_else(|| node.value.as_ref().unwrap().span())
}

fn child_ident(el_id: usize, node: &Node) -> Ident {
    let id = format!("_el{el_id}");
    match node.node_type {
        NodeType::Element => Ident::new(&id, node.name_span().unwrap()),
        NodeType::Text | NodeType::Block => Ident::new(&id, node.value.as_ref().unwrap().span()),
        _ => panic!("invalid child node type"),
    }
}

fn comment_ident(co_id: usize, node: &Node) -> Ident {
    let id = format!("_co{co_id}");
    match node.node_type {
        NodeType::Element => Ident::new(&id, node.name_span().unwrap()),
        NodeType::Text | NodeType::Block => Ident::new(&id, node.value.as_ref().unwrap().span()),
        _ => panic!("invalid child node type"),
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
            &tag_name.to_string().replace('-', "_").replace(':', "_"),
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
