mod component_builder;
mod slot_helper;
mod utils;

use self::{
    component_builder::component_to_tokens,
    slot_helper::{get_slot, slot_to_tokens},
};
use convert_case::{
    Case::{Snake, UpperCamel},
    Casing,
};
use leptos_hot_reload::parsing::{is_component_node, value_to_string};
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use proc_macro_error2::abort;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use rstml::node::{
    CustomNode, KVAttributeValue, KeyedAttribute, Node, NodeAttribute,
    NodeBlock, NodeElement, NodeName, NodeNameFragment,
};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
};
use syn::{
    spanned::Spanned, Expr, Expr::Tuple, ExprLit, ExprRange, Lit, LitStr,
    RangeLimits, Stmt,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TagType {
    Unknown,
    Html,
    Svg,
    Math,
}

pub fn render_view(
    nodes: &mut [Node],
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
    disable_inert_html: bool,
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
                &mut nodes[0],
                TagType::Unknown,
                None,
                global_class,
                view_marker.as_deref(),
                true,
                disable_inert_html,
            ),
            // only add View wrapper and view marker to a regular HTML
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
                disable_inert_html,
            ),
            true,
        ),
    };
    base.map(|view| {
        if !should_add_view {
            view
        } else if let Some(vm) = view_marker {
            quote! {
                ::leptos::prelude::View::new(
                    #view
                )
                .with_view_marker(#vm)
            }
        } else {
            quote! {
                ::leptos::prelude::View::new(
                    #view
                )
            }
        }
    })
}

fn is_inert_element(orig_node: &Node<impl CustomNode>) -> bool {
    // do not use this if the top-level node is not an Element,
    // or if it's an element with no children and no attrs
    match orig_node {
        Node::Element(el) => {
            if el.attributes().is_empty() && el.children.is_empty() {
                return false;
            }

            // also doesn't work if the top-level element is an SVG/MathML element
            let el_name = el.name().to_string();
            if is_svg_element(&el_name) || is_math_ml_element(&el_name) {
                return false;
            }
        }
        _ => return false,
    }

    // otherwise, walk over all the nodes to make sure everything is inert
    let mut nodes = VecDeque::from([orig_node]);

    while let Some(current_element) = nodes.pop_front() {
        match current_element {
            Node::Text(_) | Node::RawText(_) => {}
            Node::Element(node) => {
                if is_component_node(node) {
                    return false;
                }
                if is_spread_marker(node) {
                    return false;
                }

                match node.name() {
                    NodeName::Block(_) => return false,
                    _ => {
                        // check all attributes
                        for attr in node.attributes() {
                            match attr {
                                NodeAttribute::Block(_) => return false,
                                NodeAttribute::Attribute(attr) => {
                                    let static_key =
                                        !matches!(attr.key, NodeName::Block(_));

                                    let static_value = match attr
                                        .possible_value
                                        .to_value()
                                    {
                                        None => true,
                                        Some(value) => {
                                            matches!(&value.value, KVAttributeValue::Expr(expr) if {
                                                if let Expr::Lit(lit) = expr {
                                                    matches!(&lit.lit, Lit::Str(_))
                                                } else {
                                                    false
                                                }
                                            })
                                        }
                                    };

                                    if !static_key || !static_value {
                                        return false;
                                    }
                                }
                            }
                        }

                        // check all children
                        nodes.extend(&node.children);
                    }
                }
            }
            _ => return false,
        }
    }

    true
}

enum Item<'a, T> {
    Node(&'a Node<T>, bool),
    ClosingTag(String),
}

enum InertElementBuilder<'a> {
    GlobalClass {
        global_class: &'a TokenTree,
        strs: Vec<GlobalClassItem<'a>>,
        buffer: String,
    },
    NoGlobalClass {
        buffer: String,
    },
}

impl<'a> ToTokens for InertElementBuilder<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            InertElementBuilder::GlobalClass { strs, .. } => {
                tokens.extend(quote! {
                    [#(#strs),*].join("")
                });
            }
            InertElementBuilder::NoGlobalClass { buffer } => {
                tokens.extend(quote! {
                    #buffer
                })
            }
        }
    }
}

enum GlobalClassItem<'a> {
    Global(&'a TokenTree),
    String(String),
}

impl<'a> ToTokens for GlobalClassItem<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let addl_tokens = match self {
            GlobalClassItem::Global(v) => v.to_token_stream(),
            GlobalClassItem::String(v) => v.to_token_stream(),
        };
        tokens.extend(addl_tokens);
    }
}

impl<'a> InertElementBuilder<'a> {
    fn new(global_class: Option<&'a TokenTree>) -> Self {
        match global_class {
            None => Self::NoGlobalClass {
                buffer: String::new(),
            },
            Some(global_class) => Self::GlobalClass {
                global_class,
                strs: Vec::new(),
                buffer: String::new(),
            },
        }
    }

    fn push(&mut self, c: char) {
        match self {
            InertElementBuilder::GlobalClass { buffer, .. } => buffer.push(c),
            InertElementBuilder::NoGlobalClass { buffer } => buffer.push(c),
        }
    }

    fn push_str(&mut self, s: &str) {
        match self {
            InertElementBuilder::GlobalClass { buffer, .. } => {
                buffer.push_str(s)
            }
            InertElementBuilder::NoGlobalClass { buffer } => buffer.push_str(s),
        }
    }

    fn push_class(&mut self, class: &str) {
        match self {
            InertElementBuilder::GlobalClass {
                global_class,
                strs,
                buffer,
            } => {
                buffer.push_str(" class=\"");
                strs.push(GlobalClassItem::String(std::mem::take(buffer)));
                strs.push(GlobalClassItem::Global(global_class));
                buffer.push(' ');
                buffer.push_str(class);
                buffer.push('"');
            }
            InertElementBuilder::NoGlobalClass { buffer } => {
                buffer.push_str(" class=\"");
                buffer.push_str(class);
                buffer.push('"');
            }
        }
    }

    fn finish(&mut self) {
        match self {
            InertElementBuilder::GlobalClass { strs, buffer, .. } => {
                strs.push(GlobalClassItem::String(std::mem::take(buffer)));
            }
            InertElementBuilder::NoGlobalClass { .. } => {}
        }
    }
}

fn inert_element_to_tokens(
    node: &Node<impl CustomNode>,
    escape_text: bool,
    global_class: Option<&TokenTree>,
) -> Option<TokenStream> {
    let mut html = InertElementBuilder::new(global_class);
    let mut nodes = VecDeque::from([Item::Node(node, escape_text)]);

    while let Some(current) = nodes.pop_front() {
        match current {
            Item::ClosingTag(tag) => {
                // closing tag
                html.push_str("</");
                html.push_str(&tag);
                html.push('>');
            }
            Item::Node(current, escape) => {
                match current {
                    Node::RawText(raw) => {
                        let text = raw.to_string_best();
                        let text = if escape {
                            html_escape::encode_text(&text)
                        } else {
                            text.into()
                        };
                        html.push_str(&text);
                    }
                    Node::Text(text) => {
                        let text = text.value_string();
                        let text = if escape {
                            html_escape::encode_text(&text)
                        } else {
                            text.into()
                        };
                        html.push_str(&text);
                    }
                    Node::Element(node) => {
                        let self_closing = is_self_closing(node);
                        let el_name = node.name().to_string();
                        let escape = el_name != "script"
                            && el_name != "style"
                            && el_name != "textarea";

                        // opening tag
                        html.push('<');
                        html.push_str(&el_name);

                        for attr in node.attributes() {
                            if let NodeAttribute::Attribute(attr) = attr {
                                let attr_name = attr.key.to_string();
                                // trim r# from raw identifiers like r#as
                                let attr_name =
                                    attr_name.trim_start_matches("r#");
                                if attr_name != "class" {
                                    html.push(' ');
                                    html.push_str(attr_name);
                                }

                                if let Some(value) =
                                    attr.possible_value.to_value()
                                {
                                    if let KVAttributeValue::Expr(Expr::Lit(
                                        lit,
                                    )) = &value.value
                                    {
                                        if let Lit::Str(txt) = &lit.lit {
                                            let value = txt.value();
                                            let value = html_escape::encode_double_quoted_attribute(&value);
                                            if attr_name == "class" {
                                                html.push_class(&value);
                                            } else {
                                                html.push_str("=\"");
                                                html.push_str(&value);
                                                html.push('"');
                                            }
                                        }
                                    }
                                };
                            }
                        }

                        html.push('>');

                        // render all children
                        if !self_closing {
                            nodes.push_front(Item::ClosingTag(el_name));
                            let children = node.children.iter().rev();
                            for child in children {
                                nodes.push_front(Item::Node(child, escape));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    html.finish();

    Some(quote! {
        ::leptos::tachys::html::InertElement::new(#html)
    })
}

fn element_children_to_tokens(
    nodes: &mut [Node<impl CustomNode>],
    parent_type: TagType,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    view_marker: Option<&str>,
    disable_inert_html: bool,
) -> Option<TokenStream> {
    let children = children_to_tokens(
        nodes,
        parent_type,
        parent_slots,
        global_class,
        view_marker,
        false,
        disable_inert_html,
    );
    if children.is_empty() {
        None
    } else if children.len() == 1 {
        let child = &children[0];
        Some(quote! {
            .child(
                #[allow(unused_braces)]
                { #child }
            )
        })
    } else if children.len() > 16 {
        // implementations of various traits used in routing and rendering are implemented for
        // tuples of sizes 0, 1, 2, 3, ... N. N varies but is > 16. The traits are also implemented
        // for tuples of tuples, so if we have more than 16 items, we can split them out into
        // multiple tuples.
        let chunks = children.chunks(16).map(|children| {
            quote! {
                (#(#children),*)
            }
        });
        Some(quote! {
            .child(
                (#(#chunks),*)
            )
        })
    } else {
        Some(quote! {
            .child(
                (#(#children),*)
            )
        })
    }
}

fn fragment_to_tokens(
    nodes: &mut [Node<impl CustomNode>],
    parent_type: TagType,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    view_marker: Option<&str>,
    disable_inert_html: bool,
) -> Option<TokenStream> {
    let children = children_to_tokens(
        nodes,
        parent_type,
        parent_slots,
        global_class,
        view_marker,
        true,
        disable_inert_html,
    );
    if children.is_empty() {
        None
    } else if children.len() == 1 {
        children.into_iter().next()
    } else if children.len() > 16 {
        // implementations of various traits used in routing and rendering are implemented for
        // tuples of sizes 0, 1, 2, 3, ... N. N varies but is > 16. The traits are also implemented
        // for tuples of tuples, so if we have more than 16 items, we can split them out into
        // multiple tuples.
        let chunks = children.chunks(16).map(|children| {
            quote! {
                (#(#children),*)
            }
        });
        Some(quote! {
             (#(#chunks),*)
        })
    } else {
        Some(quote! {
            (#(#children),*)
        })
    }
}

fn children_to_tokens(
    nodes: &mut [Node<impl CustomNode>],
    parent_type: TagType,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    view_marker: Option<&str>,
    top_level: bool,
    disable_inert_html: bool,
) -> Vec<TokenStream> {
    if nodes.len() == 1 {
        match node_to_tokens(
            &mut nodes[0],
            parent_type,
            parent_slots,
            global_class,
            view_marker,
            top_level,
            disable_inert_html,
        ) {
            Some(tokens) => vec![tokens],
            None => vec![],
        }
    } else {
        let mut slots = HashMap::new();
        let nodes = nodes
            .iter_mut()
            .filter_map(|node| {
                node_to_tokens(
                    node,
                    TagType::Unknown,
                    Some(&mut slots),
                    global_class,
                    view_marker,
                    top_level,
                    disable_inert_html,
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
    node: &mut Node<impl CustomNode>,
    parent_type: TagType,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    view_marker: Option<&str>,
    top_level: bool,
    disable_inert_html: bool,
) -> Option<TokenStream> {
    let is_inert = !disable_inert_html && is_inert_element(node);

    match node {
        Node::Comment(_) => None,
        Node::Doctype(node) => {
            let value = node.value.to_string_best();
            Some(quote! { ::leptos::tachys::html::doctype(#value) })
        }
        Node::Fragment(fragment) => fragment_to_tokens(
            &mut fragment.children,
            parent_type,
            parent_slots,
            global_class,
            view_marker,
            disable_inert_html,
        ),
        Node::Block(block) => {
            Some(quote! { ::leptos::prelude::IntoRender::into_render(#block) })
        }
        Node::Text(text) => Some(text_to_tokens(&text.value)),
        Node::RawText(raw) => {
            let text = raw.to_string_best();
            let text = syn::LitStr::new(&text, raw.span());
            Some(text_to_tokens(&text))
        }
        Node::Element(el_node) => {
            if !top_level && is_inert {
                let el_name = el_node.name().to_string();
                let escape = el_name != "script"
                    && el_name != "style"
                    && el_name != "textarea";
                inert_element_to_tokens(node, escape, global_class)
            } else {
                element_to_tokens(
                    el_node,
                    parent_type,
                    parent_slots,
                    global_class,
                    view_marker,
                    disable_inert_html,
                )
            }
        }
        Node::Custom(node) => Some(node.to_token_stream()),
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
    node: &mut NodeElement<impl CustomNode>,
    mut parent_type: TagType,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    view_marker: Option<&str>,
    disable_inert_html: bool,
) -> Option<TokenStream> {
    // attribute sorting:
    //
    // the `class` and `style` attributes overwrite individual `class:` and `style:` attributes
    // when they are set. as a result, we're going to sort the attributes so that `class` and
    // `style` always come before all other attributes.

    // if there's a spread marker, we don't want to move `class` or `style` before it
    // so let's only sort attributes that come *before* a spread marker
    let spread_position = node
        .attributes()
        .iter()
        .position(|n| match n {
            NodeAttribute::Block(node) => as_spread_attr(node).is_some(),
            _ => false,
        })
        .unwrap_or_else(|| node.attributes().len());

    // now, sort the attributes
    node.attributes_mut()[0..spread_position].sort_by(|a, b| {
        let key_a = match a {
            NodeAttribute::Attribute(attr) => match &attr.key {
                NodeName::Path(attr) => {
                    attr.path.segments.first().map(|n| n.ident.to_string())
                }
                _ => None,
            },
            _ => None,
        };
        let key_b = match b {
            NodeAttribute::Attribute(attr) => match &attr.key {
                NodeName::Path(attr) => {
                    attr.path.segments.first().map(|n| n.ident.to_string())
                }
                _ => None,
            },
            _ => None,
        };
        match (key_a.as_deref(), key_b.as_deref()) {
            (Some("class"), _) | (Some("style"), _) => Ordering::Less,
            (_, Some("class")) | (_, Some("style")) => Ordering::Greater,
            _ => Ordering::Equal,
        }
    });

    // check for duplicate attribute names and emit an error for all subsequent ones
    let mut names = HashSet::new();
    for attr in node.attributes() {
        if let NodeAttribute::Attribute(attr) = attr {
            let mut name = attr.key.to_string();
            if let Some(tuple_name) = tuple_name(&name, attr) {
                name.push(':');
                name.push_str(&tuple_name);
            }
            if names.contains(&name) {
                proc_macro_error2::emit_error!(
                    attr.span(),
                    format!("This element already has a `{name}` attribute.")
                );
            } else {
                names.insert(name);
            }
        }
    }

    let name = node.name();
    if is_component_node(node) {
        if let Some(slot) = get_slot(node) {
            let slot = slot.clone();
            slot_to_tokens(
                node,
                &slot,
                parent_slots,
                global_class,
                disable_inert_html,
            );
            None
        } else {
            Some(component_to_tokens(node, global_class, disable_inert_html))
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
            let name = if tag == "use" || tag == "use_" {
                Ident::new_raw("use", name.span()).to_token_stream()
            } else {
                name.to_token_stream()
            };
            quote! { ::leptos::tachys::svg::#name() }
        } else if is_math_ml_element(&tag) {
            parent_type = TagType::Math;
            quote! { ::leptos::tachys::mathml::#name() }
        } else if is_ambiguous_element(&tag) {
            match parent_type {
                TagType::Unknown => {
                    // We decided this warning was too aggressive, but I'll leave it here in case we want it later
                    /* proc_macro_error2::emit_warning!(name.span(), "The view macro is assuming this is an HTML element, \
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
                &mut node.children,
                parent_type,
                parent_slots,
                global_class,
                view_marker,
                disable_inert_html,
            )
        } else {
            if !node.children.is_empty() {
                let name = node.name();
                proc_macro_error2::emit_error!(
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

fn is_spread_marker(node: &NodeElement<impl CustomNode>) -> bool {
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

fn as_spread_attr(node: &NodeBlock) -> Option<Option<&Expr>> {
    if let NodeBlock::ValidBlock(block) = node {
        match block.stmts.first() {
            Some(Stmt::Expr(
                Expr::Range(ExprRange {
                    start: None,
                    limits: RangeLimits::HalfOpen(_),
                    end,
                    ..
                }),
                _,
            )) => Some(end.as_deref()),
            _ => None,
        }
    } else {
        None
    }
}

fn attribute_to_tokens(
    tag_type: TagType,
    node: &NodeAttribute,
    global_class: Option<&TokenTree>,
    is_custom: bool,
) -> TokenStream {
    match node {
        NodeAttribute::Block(node) => as_spread_attr(node)
            .flatten()
            .map(|end| {
                quote! {
                    .add_any_attr(#end)
                }
            })
            .unwrap_or_else(|| {
                quote! {
                    .add_any_attr(#[allow(unused_braces)] { #node })
                }
            }),
        NodeAttribute::Attribute(node) => {
            let name = node.key.to_string();
            if name == "node_ref" {
                let node_ref = match &node.key {
                    NodeName::Path(path) => path.path.get_ident(),
                    _ => unreachable!(),
                };
                let value = attribute_value(node, false);
                quote! {
                    .#node_ref(#value)
                }
            } else if let Some(name) = name.strip_prefix("use:") {
                directive_call_from_attribute_node(node, name)
            } else if let Some(name) = name.strip_prefix("on:") {
                event_to_tokens(name, node)
            } else if let Some(name) = name.strip_prefix("bind:") {
                two_way_binding_to_tokens(name, node)
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
                let value = attribute_value(node, true);
                quote! {
                    .attr(#name, #value)
                }
            } else {
                let key = attribute_name(&node.key);
                let value = attribute_value(node, true);

                // special case of global_class and class attribute
                if &node.key.to_string() == "class"
                    && global_class.is_some()
                    && node.value().and_then(value_to_string).is_none()
                {
                    let span = node.key.span();
                    proc_macro_error2::emit_error!(span, "Combining a global class (view! { class = ... }) \
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
    let key = node.key.to_string();
    let contains_dash = key.contains('-');
    let attr_aira = key.starts_with("attr:aria-");
    // anything that follows the x:y pattern
    match &node.key {
        NodeName::Punctuated(parts) if !contains_dash || attr_aira => {
            if parts.len() >= 2 {
                let id = &parts[0];
                match id {
                    NodeNameFragment::Ident(id) => {
                        // ignore `let:` and `clone:`
                        if id == "let" || id == "clone" {
                            None
                        } else if id == "attr" {
                        let value = attribute_value(node, true);
                            let key = &parts[1];
                            let key_name = key.to_string();
                            if key_name == "class" || key_name == "style" {
                                Some(
                                    quote! { ::leptos::tachys::html::#key::#key(#value) },
                                )
                            } else if key_name == "aria" {
                                let value = attribute_value(node, true);
                                let mut parts_iter = parts.iter();
                                parts_iter.next();
                                let fn_name = parts_iter.map(|p| p.to_string()).collect::<Vec<String>>().join("_");
                                let key = Ident::new(&fn_name, key.span());
                                Some(
                                    quote! { ::leptos::tachys::html::attribute::#key(#value) },
                                )
                            } else {
                                Some(
                                    quote! { ::leptos::tachys::html::attribute::#key(#value) },
                                )
                            }
                        } else if id == "use" {
                            let key = &parts[1];
                            let param = if let Some(value) = node.value() {
                                quote!(#value)
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
                            let value = attribute_value(node, false);
                            let key = &node.key.to_string();
                            let key = key
                                .replacen("style:", "", 1)
                                .replacen("class:", "", 1);
                            Some(
                                quote! { ::leptos::tachys::html::#id::#id((#key, #value)) },
                            )
                        } else if id == "prop" {
                            let value = attribute_value(node, false);
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
                            proc_macro_error2::abort!(
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

pub(crate) fn two_way_binding_to_tokens(
    name: &str,
    node: &KeyedAttribute,
) -> TokenStream {
    let value = attribute_value(node, false);

    let ident =
        format_ident!("{}", name.to_case(UpperCamel), span = node.key.span());

    quote! {
        .bind(::leptos::attr::#ident, #value)
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
    let handler = attribute_value(node, false);

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
        quote! { #ev_name }
    } else {
        event_type
    };

    let event_type = if is_force_undelegated {
        let undelegated = if let Some(undelegated) = undelegated_ident {
            quote! { #undelegated }
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
    let value = attribute_value(node, false);
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
    let value = attribute_value(node, false);
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
    let value = attribute_value(node, false);
    quote! {
        .#prop(#key, #value)
    }
}

fn is_custom_element(tag: &str) -> bool {
    tag.contains('-')
}

fn is_self_closing(node: &NodeElement<impl CustomNode>) -> bool {
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

fn attribute_value(
    attr: &KeyedAttribute,
    is_attribute_proper: bool,
) -> TokenStream {
    match attr.possible_value.to_value() {
        None => quote! { true },
        Some(value) => match &value.value {
            KVAttributeValue::Expr(expr) => {
                if let Expr::Lit(lit) = expr {
                    if cfg!(feature = "nightly") {
                        if let Lit::Str(str) = &lit.lit {
                            return quote! {
                                ::leptos::tachys::view::static_types::Static::<#str>
                            };
                        }
                    }
                }

                if matches!(expr, Expr::Lit(_)) || !is_attribute_proper {
                    quote! {
                        #expr
                    }
                } else {
                    quote! {
                        ::leptos::prelude::IntoAttributeValue::into_attribute_value(#expr)
                    }
                }
            }
            // any value in braces: expand as-is to give proper r-a support
            KVAttributeValue::InvalidBraced(block) => {
                if is_attribute_proper {
                    quote! {
                        ::leptos::prelude::IntoAttributeValue::into_attribute_value(#block)
                    }
                } else {
                    quote! {
                        #block
                    }
                }
            }
        },
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

pub(crate) fn directive_call_from_attribute_node(
    attr: &KeyedAttribute,
    directive_name: &str,
) -> TokenStream {
    let handler = syn::Ident::new(directive_name, attr.key.span());

    let param = if let Some(value) = attr.value() {
        quote!(#value)
    } else {
        quote_spanned!(attr.key.span()=> ().into())
    };

    quote! { .directive(#handler, #[allow(clippy::useless_conversion)] #param) }
}

fn tuple_name(name: &str, node: &KeyedAttribute) -> Option<String> {
    if name == "style" || name == "class" {
        if let Some(Tuple(tuple)) = node.value() {
            {
                if tuple.elems.len() == 2 {
                    let style_name = &tuple.elems[0];
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }) = style_name
                    {
                        return Some(s.value());
                    }
                }
            }
        }
    }

    None
}
