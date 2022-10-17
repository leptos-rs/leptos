use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, ExprPath};
use syn_rsx::{Node, NodeName, NodeType};
use uuid::Uuid;

use crate::{is_component_node, Mode};

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
    match node.node_type {
        NodeType::Doctype | NodeType::Comment => quote! {},
        NodeType::Fragment => {
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
        NodeType::Element => root_element_to_tokens(cx, template_uid, node, mode),
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

fn root_element_to_tokens(
    cx: &Ident,
    template_uid: &Ident,
    node: &Node,
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
                        let name = node.name_as_string().unwrap();
                        quote! {
                            let root = #template_uid.with(|template| #cx.get_next_element(template));
                            // //log::debug!("root = {}", root.node_name());
                        }
                    }
                };

                let span = node.name_span().unwrap();

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

#[allow(clippy::too_many_arguments)]
fn element_to_tokens(
    cx: &Ident,
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

    // Open tag
    let name_str = node.name_as_string().unwrap();
    let span = node.name_span().unwrap();

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

    // attributes
    for attr in &node.attributes {
        attr_to_tokens(
            cx,
            attr,
            &this_el_ident,
            template,
            expressions,
            navigations,
            mode,
        );
    }

    // navigation for this el
    let debug_name = debug_name(node);
    if mode != Mode::Ssr {
        let this_nav = if is_root_el {
            quote_spanned! {
                span => let #this_el_ident = #debug_name;
                    let #this_el_ident = #parent.clone().unchecked_into::<web_sys::Node>();
                    //log::debug!("=> got {}", #this_el_ident.node_name());
            }
        } else if let Some(prev_sib) = &prev_sib {
            quote_spanned! {
                span => let #this_el_ident = #debug_name;
                    //log::debug!("next_sibling ({})", #debug_name);
                    let #this_el_ident = #prev_sib.next_sibling().unwrap_throw();
                    //log::debug!("=> got {}", #this_el_ident.node_name());
            }
        } else {
            quote_spanned! {
                span => let #this_el_ident = #debug_name;
                    //log::debug!("first_child ({})", #debug_name);
                    let #this_el_ident = #parent.first_child().unwrap_throw();
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
        if is_component_node(sibling) {
            next_sibling_node(children, idx + 1, next_el_id)
        } else {
            Some(child_ident(*next_el_id + 1, sibling))
        }
    }
}

fn attr_to_tokens(
    cx: &Ident,
    node: &Node,
    el_id: &Ident,
    template: &mut String,
    expressions: &mut Vec<TokenStream>,
    navigations: &mut Vec<TokenStream>,
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
            syn::Expr::Lit(expr_lit) => {
                if matches!(expr_lit.lit, syn::Lit::Str(_)) {
                    AttributeValue::Static(node.value_as_string().unwrap())
                } else {
                    AttributeValue::Dynamic(expr)
                }
            }
            _ => AttributeValue::Dynamic(expr),
        },
        None => AttributeValue::Empty,
    };

    let span = node.name_span().unwrap();

    // refs
    if name == "ref" {
        let ident = match &node.value {
            Some(expr) => {
                if let Some(ident) = expr_to_ident(expr) {
                    quote_spanned! { span => #ident }
                } else {
                    quote_spanned! { span => compile_error!("'ref' needs to be passed a variable name") }
                }
            }
            None => {
                quote_spanned! { span => compile_error!("'ref' needs to be passed a variable name") }
            }
        };

        if mode == Mode::Ssr {
            // fake the initialization; should only be used in effects or event handlers, which will never run on the server
            // but if we don't initialize it, the compiler will complain
            navigations.push(quote_spanned! {
                span => #ident = String::new();
            });
        } else {
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
            span => leptos_dom::property(#cx, #el_id.unchecked_ref(), #name, #value.into_property(#cx))
        });
        }
    }
    // Classes
    else if name.starts_with("class:") {
        if mode == Mode::Ssr {
            // TODO class: in SSR
        } else {
            let name = name.replacen("class:", "", 1);
            let value = node.value.as_ref().expect("class: attributes need values");
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
                            leptos_buffer.push_str(&leptos_dom::escape_attr(&{#value}.into_attribute(#cx).as_value_string(#name)));
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
            let location = if let Some(sibling) = &prev_sib {
                quote_spanned! {
                    span => //log::debug!("-> next sibling");
                            let #name = #sibling.next_sibling().unwrap_throw();
                            //log::debug!("\tnext sibling = {}", #name.node_name());
                }
            } else {
                quote_spanned! {
                    span => //log::debug!("\\|/ first child on {}", #parent.node_name());
                            let #name = #parent.first_child().unwrap_throw();
                            //log::debug!("\tfirst child = {}", #name.node_name());
                }
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
            let value = node.value.as_ref().expect("no block value");

            if let Some(v) = str_value {
                if mode == Mode::Ssr {
                    expressions.push(quote::quote_spanned! {
                        span => leptos_buffer.push_str(&leptos_dom::escape_text(&#v));
                    });
                } else {
                    navigations.push(location);
                    template.push_str(&v);
                }

                PrevSibChange::Sib(name)
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
                        let el = child_ident(*next_el_id, node);
                        *next_co_id += 1;
                        let co = comment_ident(*next_co_id, node);
                        next_sib = Some(el.clone());

                        template.push_str("<!#><!/>");
                        navigations.push(quote! {
                            #location;
                            let (#el, #co) = #cx.get_next_marker(&#name);
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

                        current = Some(el);
                    }
                    // in SSR, it needs to insert the value, wrapped in comments
                    Mode::Ssr => expressions.push(quote::quote_spanned! {
                        span => leptos_buffer.push_str("<!--#-->");
                                leptos_buffer.push_str(&#value.into_child(#cx).as_child_string());
                                leptos_buffer.push_str("<!--/-->");
                    }),
                }

                PrevSibChange::Sib(name)
            }
        }
        _ => panic!("unexpected child node type"),
    }
}

#[allow(clippy::too_many_arguments)]
fn component_to_tokens(
    cx: &Ident,
    node: &Node,
    parent: Option<&Ident>,
    prev_sib: Option<Ident>,
    mut next_sib: Option<Ident>,
    template: &mut String,
    expressions: &mut Vec<TokenStream>,
    navigations: &mut Vec<TokenStream>,
    next_el_id: &mut usize,
    next_co_id: &mut usize,
    multi: bool,
    mode: Mode,
) -> PrevSibChange {
    let create_component = create_component(cx, node, mode);
    let span = node.name_span().unwrap();

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
            let name = child_ident(*next_el_id, node);
            *next_el_id += 1;
            let el = child_ident(*next_el_id, node);
            *next_co_id += 1;
            let co = comment_ident(*next_co_id, node);
            next_sib = Some(el.clone());

            let starts_at = if let Some(prev_sib) = prev_sib {
                quote::quote! {{
                    //log::debug!("starts_at = next_sibling");
                    #prev_sib.next_sibling().unwrap_throw()
                    //log::debug!("ok starts_at");
                }}
            } else {
                quote::quote! {{
                    //log::debug!("starts_at first_child");
                    #parent.first_child().unwrap_throw()
                    //log::debug!("starts_at ok");
                }}
            };

            current = Some(el.clone());

            template.push_str("<!#><!/>");
            navigations.push(quote! {
                let (#el, #co) = #cx.get_next_marker(&#starts_at);
            });

            expressions.push(quote! {
                leptos::insert(
                    #cx,
                    #parent.clone(),
                    #create_component.into_child(#cx),
                    Marker::BeforeChild(#el),
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
        None => PrevSibChange::Skip,
    }
}

fn create_component(cx: &Ident, node: &Node, mode: Mode) -> TokenStream {
    let component_name = ident_from_tag_name(node.name.as_ref().unwrap());
    let span = node.name_span().unwrap();
    let component_props_name = Ident::new(&format!("{component_name}Props"), span);

    let (initialize_children, children) = if node.children.is_empty() {
        (quote! {}, quote! {})
    } else if node.children.len() == 1 {
        let child = render_view(cx, &node.children, mode);

        if mode == Mode::Hydrate {
            (
                quote_spanned! { span => let children = vec![#child]; },
                quote_spanned! { span => .children(Box::new(move || children.clone())) },
            )
        } else {
            (
                quote! {},
                quote_spanned! { span => .children(Box::new(move || vec![#child])) },
            )
        }
    } else {
        let children = render_view(cx, &node.children, mode);

        if mode == Mode::Hydrate {
            (
                quote_spanned! { span => let children = Box::new(move || #children); },
                quote_spanned! { span => .children(children) },
            )
        } else {
            (
                quote! {},
                quote_spanned! { span => .children(Box::new(move || #children)) },
            )
        }
    };

    let props = node.attributes.iter().filter_map(|attr| {
        let attr_name = attr.name_as_string().unwrap_or_default();
        if attr_name.starts_with("on:")
            || attr_name.starts_with("prop:")
            || attr_name.starts_with("class:")
            || attr_name.starts_with("attr:")
        {
            None
        } else {
            let name = ident_from_tag_name(attr.name.as_ref().unwrap());
            let span = attr.name_span().unwrap();
            let value = attr
                .value
                .as_ref()
                .map(|v| quote_spanned! { span => #v })
                .unwrap_or_else(|| quote_spanned! { span => #name });
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
            let value = attr.value.as_ref().expect("prop: attributes need values");
            Some(quote_spanned! {
                span => leptos_dom::property(#cx, #component_name.unchecked_ref(), #name, #value.into_property(#cx))
            })
        }
        // Classes
        else if let Some(name) = attr_name.strip_prefix("class:") {
            let value = attr.value.as_ref().expect("class: attributes need values");
            Some(quote_spanned! {
                span => leptos_dom::class(#cx, #component_name.unchecked_ref(), #name, #value.into_class(#cx))
            })
        }
        // Attributes
        else if let Some(name) = attr_name.strip_prefix("attr:") {
            let value = attr.value.as_ref().expect("attr: attributes need values");
            let name = name.replace('_', "-");
            Some(quote_spanned! {
                span => leptos_dom::attribute(#cx, #component_name.unchecked_ref(), #name, #value.into_attribute(#cx))
            })
        }
        else {
            None
        }
    }).peekable();

    if other_attrs.peek().is_none() {
        quote_spanned! {
            span => create_component(#cx, move || {
                #initialize_children
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
                #initialize_children
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
