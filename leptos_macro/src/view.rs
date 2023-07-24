use crate::{attribute_value, Mode};
use convert_case::{Case::Snake, Casing};
use leptos_hot_reload::parsing::{
    block_to_primitive_expression, is_component_node, is_component_tag_name,
    value_to_string,
};
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned};
use rstml::node::{
    KeyedAttribute, Node, NodeAttribute, NodeBlock, NodeElement, NodeName,
};
use std::collections::HashMap;
use syn::{spanned::Spanned, Expr, Expr::Tuple, ExprLit, ExprPath, Lit};

#[derive(Clone, Copy)]
enum TagType {
    Unknown,
    Html,
    Svg,
    Math,
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

pub(crate) fn render_view(
    cx: &Ident,
    nodes: &[Node],
    mode: Mode,
    global_class: Option<&TokenTree>,
    call_site: Option<String>,
) -> TokenStream {
    let empty = {
        let span = Span::call_site();
        quote_spanned! {
            span => leptos::leptos_dom::Unit
        }
    };

    if mode == Mode::Ssr {
        match nodes.len() {
            0 => empty,
            1 => {
                root_node_to_tokens_ssr(cx, &nodes[0], global_class, call_site)
            }
            _ => fragment_to_tokens_ssr(
                cx,
                Span::call_site(),
                nodes,
                global_class,
                call_site,
            ),
        }
    } else {
        match nodes.len() {
            0 => empty,
            1 => node_to_tokens(
                cx,
                &nodes[0],
                TagType::Unknown,
                None,
                global_class,
                call_site,
            )
            .unwrap_or_default(),
            _ => fragment_to_tokens(
                cx,
                Span::call_site(),
                nodes,
                true,
                TagType::Unknown,
                None,
                global_class,
                call_site,
            )
            .unwrap_or(empty),
        }
    }
}

fn root_node_to_tokens_ssr(
    cx: &Ident,
    node: &Node,
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> TokenStream {
    match node {
        Node::Fragment(fragment) => fragment_to_tokens_ssr(
            cx,
            Span::call_site(),
            &fragment.children,
            global_class,
            view_marker,
        ),
        Node::Comment(_) | Node::Doctype(_) => quote! {},
        Node::Text(node) => {
            quote! {
                leptos::leptos_dom::html::text(#node)
            }
        }
        Node::RawText(r) => {
            let text = r.to_string_best();
            let text = syn::LitStr::new(&text, r.span());
            quote! {
                leptos::leptos_dom::html::text(#text)
            }
        }
        Node::Block(node) => {
            quote! {
                #node
            }
        }
        Node::Element(node) => {
            root_element_to_tokens_ssr(cx, node, global_class, view_marker)
                .unwrap_or_default()
        }
    }
}

fn fragment_to_tokens_ssr(
    cx: &Ident,
    _span: Span,
    nodes: &[Node],
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> TokenStream {
    let view_marker = if let Some(marker) = view_marker {
        quote! { .with_view_marker(#marker) }
    } else {
        quote! {}
    };
    let nodes = nodes.iter().map(|node| {
        let node = root_node_to_tokens_ssr(cx, node, global_class, None);
        quote! {
            #node.into_view(#cx)
        }
    });
    quote! {
        {
            leptos::Fragment::lazy(|| [
                #(#nodes),*
            ].to_vec())
            #view_marker
        }
    }
}

fn root_element_to_tokens_ssr(
    cx: &Ident,
    node: &NodeElement,
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> Option<TokenStream> {
    // TODO: simplify, this is checked twice, second time in `element_to_tokens_ssr` body
    if is_component_node(node) {
        if let Some(slot) = get_slot(node) {
            slot_to_tokens(cx, node, slot, None, global_class);
            None
        } else {
            Some(component_to_tokens(cx, node, global_class))
        }
    } else {
        let mut stmts_for_ide = IdeTagHelper::new();
        let mut exprs_for_compiler = Vec::<TokenStream>::new();

        let mut template = String::new();
        let mut holes = Vec::new();
        let mut chunks = Vec::new();
        element_to_tokens_ssr(
            cx,
            node,
            None,
            &mut template,
            &mut holes,
            &mut chunks,
            &mut stmts_for_ide,
            &mut exprs_for_compiler,
            true,
            global_class,
        );

        // push final chunk
        if !template.is_empty() {
            chunks.push(SsrElementChunks::String { template, holes })
        }

        let chunks = chunks.into_iter().map(|chunk| match chunk {
            SsrElementChunks::String { template, holes } => {
                if holes.is_empty() {
                    let template = template.replace("\\{", "{").replace("\\}", "}");
                    quote! {
                        leptos::leptos_dom::html::StringOrView::String(#template.into())
                    }
                } else {
                let template = template.replace("\\{", "{{").replace("\\}", "}}");
                    quote! {
                        leptos::leptos_dom::html::StringOrView::String(
                            format!(
                                #template,
                                #(#holes),*
                            )
                            .into()
                        )
                    }
                }
            }
            SsrElementChunks::View(view) => {
                quote! {
                    #[allow(unused_braces)]
                    {
                        let view = #view;
                        leptos::leptos_dom::html::StringOrView::View(std::rc::Rc::new(move || view.clone()))
                    }
                }
            },
        });

        let tag_name = node.name().to_string();
        let is_custom_element = is_custom_element(&tag_name);

        // Use any other span instead of node.name.span(), to avoid missundestanding in IDE.
        // We can use open_tag.span(), to provide simmilar(to name span) diagnostic
        // in case of expansion error, but it will also higlight "<" token.
        let typed_element_name = if is_custom_element {
            Ident::new(CUSTOM_EVENT, Span::call_site())
        } else {
            let camel_cased = camel_case_tag_name(
                tag_name
                    .trim_start_matches("svg::")
                    .trim_start_matches("math::")
                    .trim_end_matches('_'),
            );
            Ident::new(&camel_cased, Span::call_site())
        };
        let typed_element_name = if is_svg_element(&tag_name) {
            quote! { svg::#typed_element_name }
        } else if is_math_ml_element(&tag_name) {
            quote! { math::#typed_element_name }
        } else {
            quote! { html::#typed_element_name }
        };
        let full_name = if is_custom_element {
            quote! {
                leptos::leptos_dom::html::Custom::new(#tag_name)
            }
        } else {
            quote! {
                leptos::leptos_dom::#typed_element_name::default()
            }
        };
        let view_marker = if let Some(marker) = view_marker {
            quote! { .with_view_marker(#marker) }
        } else {
            quote! {}
        };
        let stmts_for_ide = stmts_for_ide.into_iter();
        Some(quote! {
        {
            #(#stmts_for_ide)*
            #(#exprs_for_compiler)*
            ::leptos::HtmlElement::from_chunks(#cx, #full_name, [#(#chunks),*])#view_marker
        }
        })
    }
}

enum SsrElementChunks {
    String {
        template: String,
        holes: Vec<TokenStream>,
    },
    View(TokenStream),
}

#[allow(clippy::too_many_arguments)]
fn element_to_tokens_ssr(
    cx: &Ident,
    node: &NodeElement,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    template: &mut String,
    holes: &mut Vec<TokenStream>,
    chunks: &mut Vec<SsrElementChunks>,
    stmts_for_ide: &mut IdeTagHelper,
    exprs_for_compiler: &mut Vec<TokenStream>,
    is_root: bool,
    global_class: Option<&TokenTree>,
) {
    if is_component_node(node) {
        if let Some(slot) = get_slot(node) {
            slot_to_tokens(cx, node, slot, parent_slots, global_class);
            return;
        }

        let component = component_to_tokens(cx, node, global_class);

        if !template.is_empty() {
            chunks.push(SsrElementChunks::String {
                template: std::mem::take(template),
                holes: std::mem::take(holes),
            })
        }

        chunks.push(SsrElementChunks::View(quote! {
          {#component}.into_view(#cx)
        }));
    } else {
        let tag_name = node.name().to_string();
        let tag_name = tag_name
            .trim_start_matches("svg::")
            .trim_start_matches("math::")
            .trim_end_matches('_');
        let is_script_or_style = tag_name == "script" || tag_name == "style";
        template.push('<');
        template.push_str(tag_name);

        #[cfg(debug_assertions)]
        stmts_for_ide.save_element_completion(node);

        let mut inner_html = None;

        for attr in node.attributes() {
            if let NodeAttribute::Attribute(attr) = attr {
                inner_html = attribute_to_tokens_ssr(
                    cx,
                    attr,
                    template,
                    holes,
                    exprs_for_compiler,
                    global_class,
                );
            }
        }

        // insert hydration ID
        let hydration_id = if is_root {
            quote! { leptos::leptos_dom::HydrationCtx::peek() }
        } else {
            quote! { leptos::leptos_dom::HydrationCtx::id() }
        };
        match node
            .attributes()
            .iter()
            .find(|node| matches!(node, NodeAttribute::Attribute(attr) if attr.key.to_string() == "id"))
        {
            Some(_) => {
                template.push_str(" leptos-hk=\"_{}\"");
            }
            None => {
                template.push_str(" id=\"_{}\"");
            }
        }
        holes.push(hydration_id);

        set_class_attribute_ssr(cx, node, template, holes, global_class);
        set_style_attribute_ssr(cx, node, template, holes);

        if is_self_closing(node) {
            template.push_str("/>");
        } else {
            template.push('>');

            if let Some(inner_html) = inner_html {
                template.push_str("{}");
                let value = inner_html;

                holes.push(quote! {
                  (#value).into_attribute(#cx).as_nameless_value_string().unwrap_or_default()
                })
            } else {
                for child in &node.children {
                    match child {
                        Node::Element(child) => {
                            element_to_tokens_ssr(
                                cx,
                                child,
                                None,
                                template,
                                holes,
                                chunks,
                                stmts_for_ide,
                                exprs_for_compiler,
                                false,
                                global_class,
                            );
                        }
                        Node::Text(text) => {
                            let value = text.value_string();
                            let value = if is_script_or_style {
                                value.into()
                            } else {
                                html_escape::encode_safe(&value)
                            };
                            template.push_str(
                                &value.replace('{', "\\{").replace('}', "\\}"),
                            );
                        }
                        Node::RawText(r) => {
                            let value = r.to_string_best();
                            let value = if is_script_or_style {
                                value.into()
                            } else {
                                html_escape::encode_safe(&value)
                            };
                            template.push_str(
                                &value.replace('{', "\\{").replace('}', "\\}"),
                            );
                        }
                        Node::Block(NodeBlock::ValidBlock(block)) => {
                            if let Some(value) =
                                block_to_primitive_expression(block)
                                    .and_then(value_to_string)
                            {
                                template.push_str(&value);
                            } else {
                                if !template.is_empty() {
                                    chunks.push(SsrElementChunks::String {
                                        template: std::mem::take(template),
                                        holes: std::mem::take(holes),
                                    })
                                }
                                chunks.push(SsrElementChunks::View(quote! {
                                    {#block}.into_view(#cx)
                                }));
                            }
                        }
                        // Keep invalid blocks for faster IDE diff (on user type)
                        Node::Block(block @ NodeBlock::Invalid { .. }) => {
                            chunks.push(SsrElementChunks::View(quote! {
                                {#block}.into_view(#cx)
                            }));
                        }
                        Node::Fragment(_) => abort!(
                            Span::call_site(),
                            "You can't nest a fragment inside an element."
                        ),
                        _ => {}
                    }
                }
            }

            template.push_str("</");
            template.push_str(tag_name);
            template.push('>');
        }
    }
}

// returns `inner_html`
fn attribute_to_tokens_ssr<'a>(
    cx: &Ident,
    attr: &'a KeyedAttribute,
    template: &mut String,
    holes: &mut Vec<TokenStream>,
    exprs_for_compiler: &mut Vec<TokenStream>,
    global_class: Option<&TokenTree>,
) -> Option<&'a syn::Expr> {
    let name = attr.key.to_string();
    if name == "ref" || name == "_ref" || name == "ref_" || name == "node_ref" {
        // ignore refs on SSR
    } else if let Some(name) = name.strip_prefix("on:") {
        let handler = attribute_value(attr);
        let (event_type, _, _) = parse_event_name(name);

        exprs_for_compiler.push(quote! {
            leptos::leptos_dom::helpers::ssr_event_listener(::leptos::ev::#event_type, #handler);
        })
    } else if name.strip_prefix("prop:").is_some()
        || name.strip_prefix("class:").is_some()
        || name.strip_prefix("style:").is_some()
    {
        // ignore props for SSR
        // ignore classes and styles: we'll handle these separately
        if name.starts_with("prop:") {
            let value = attr.value();
            exprs_for_compiler.push(quote! {
                #[allow(unused_braces)]
                { _ = #value; }
            });
        }
    } else if name == "inner_html" {
        return attr.value();
    } else {
        let name = name.replacen("attr:", "", 1);

        // special case of global_class and class attribute
        if name == "class"
            && global_class.is_some()
            && attr.value().and_then(value_to_string).is_none()
        {
            let span = attr.key.span();
            proc_macro_error::emit_error!(span, "Combining a global class (view! { cx, class = ... }) \
            and a dynamic `class=` attribute on an element causes runtime inconsistencies. You can \
            toggle individual classes dynamically with the `class:name=value` syntax. \n\nSee this issue \
            for more information and an example: https://github.com/leptos-rs/leptos/issues/773")
        };

        if name != "class" && name != "style" {
            template.push(' ');

            if let Some(value) = attr.value() {
                if let Some(value) = value_to_string(value) {
                    template.push_str(&name);
                    template.push_str("=\"");
                    template.push_str(&html_escape::encode_quoted_attribute(
                        &value,
                    ));
                    template.push('"');
                } else {
                    template.push_str("{}");
                    holes.push(quote! {
                        &{#value}.into_attribute(#cx)
                            .as_nameless_value_string()
                            .map(|a| format!("{}=\"{}\"", #name, leptos::leptos_dom::ssr::escape_attr(&a)))
                            .unwrap_or_default()
                    })
                }
            } else {
                template.push_str(&name);
            }
        }
    };
    None
}

fn set_class_attribute_ssr(
    cx: &Ident,
    node: &NodeElement,
    template: &mut String,
    holes: &mut Vec<TokenStream>,
    global_class: Option<&TokenTree>,
) {
    let (static_global_class, dyn_global_class) = match global_class {
        Some(TokenTree::Literal(lit)) => {
            let str = lit.to_string();
            // A lit here can be a string, byte_string, char, byte_char, int or float.
            // If it's a string we remove the quotes so folks can use them directly
            // without needing braces. E.g. view!{cx, class="my-class", ... }
            let str = if str.starts_with('"') && str.ends_with('"') {
                str[1..str.len() - 1].to_string()
            } else {
                str
            };
            (str, None)
        }
        None => (String::new(), None),
        Some(val) => (String::new(), Some(val)),
    };
    let static_class_attr = node
        .attributes()
        .iter()
        .filter_map(|a| match a {
            NodeAttribute::Attribute(attr)
                if attr.key.to_string() == "class" =>
            {
                attr.value().and_then(value_to_string)
            }
            _ => None,
        })
        .chain(Some(static_global_class))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    let dyn_class_attr = node
        .attributes()
        .iter()
        .filter_map(|a| {
            if let NodeAttribute::Attribute(a) = a {
                if a.key.to_string() == "class" {
                    if a.value().and_then(value_to_string).is_some()
                        || fancy_class_name(&a.key.to_string(), cx, a).is_some()
                    {
                        None
                    } else {
                        Some((a.key.span(), a.value()))
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let class_attrs = node
        .attributes()
        .iter()
        .filter_map(|node| {
            if let NodeAttribute::Attribute(node) = node {
                let name = node.key.to_string();
                if name == "class" {
                    return if let Some((_, name, value)) =
                        fancy_class_name(&name, cx, node)
                    {
                        let span = node.key.span();
                        Some((span, name, value))
                    } else {
                        None
                    };
                }
                if name.starts_with("class:") || name.starts_with("class-") {
                    let name = if name.starts_with("class:") {
                        name.replacen("class:", "", 1)
                    } else if name.starts_with("class-") {
                        name.replacen("class-", "", 1)
                    } else {
                        name
                    };
                    let value = attribute_value(node);
                    let span = node.key.span();
                    Some((span, name, value))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if !static_class_attr.is_empty()
        || !dyn_class_attr.is_empty()
        || !class_attrs.is_empty()
        || dyn_global_class.is_some()
    {
        template.push_str(" class=\"");

        template.push_str(&html_escape::encode_quoted_attribute(
            &static_class_attr,
        ));

        for (_span, value) in dyn_class_attr {
            if let Some(value) = value {
                template.push_str(" {}");
                holes.push(quote! {
                  &(#cx, #value).into_attribute(#cx).as_nameless_value_string()
                    .map(|a| leptos::leptos_dom::ssr::escape_attr(&a).to_string())
                    .unwrap_or_default()
                });
            }
        }

        for (_span, name, value) in &class_attrs {
            template.push_str(" {}");
            holes.push(quote! {
              (#cx, #value).into_class(#cx).as_value_string(#name)
            });
        }

        if let Some(dyn_global_class) = dyn_global_class {
            template.push_str(" {}");
            holes.push(quote! { #dyn_global_class });
        }

        template.push('"');
    }
}

fn set_style_attribute_ssr(
    cx: &Ident,
    node: &NodeElement,
    template: &mut String,
    holes: &mut Vec<TokenStream>,
) {
    let static_style_attr = node
        .attributes()
        .iter()
        .filter_map(|a| match a {
            NodeAttribute::Attribute(attr)
                if attr.key.to_string() == "style" =>
            {
                attr.value().and_then(value_to_string)
            }
            _ => None,
        })
        .next()
        .map(|style| format!("{style};"));

    let dyn_style_attr = node
        .attributes()
        .iter()
        .filter_map(|a| {
            if let NodeAttribute::Attribute(a) = a {
                if a.key.to_string() == "style" {
                    if a.value().and_then(value_to_string).is_some()
                        || fancy_style_name(&a.key.to_string(), cx, a).is_some()
                    {
                        None
                    } else {
                        Some((a.key.span(), a.value()))
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let style_attrs = node
        .attributes()
        .iter()
        .filter_map(|node| {
            if let NodeAttribute::Attribute(node) = node {
                let name = node.key.to_string();
                if name == "style" {
                    return if let Some((_, name, value)) =
                        fancy_style_name(&name, cx, node)
                    {
                        let span = node.key.span();
                        Some((span, name, value))
                    } else {
                        None
                    };
                }
                if name.starts_with("style:") || name.starts_with("style-") {
                    let name = if name.starts_with("style:") {
                        name.replacen("style:", "", 1)
                    } else if name.starts_with("style-") {
                        name.replacen("style-", "", 1)
                    } else {
                        name
                    };
                    let value = attribute_value(node);
                    let span = node.key.span();
                    Some((span, name, value))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if static_style_attr.is_some()
        || !dyn_style_attr.is_empty()
        || !style_attrs.is_empty()
    {
        template.push_str(" style=\"");

        template.push_str(&static_style_attr.unwrap_or_default());

        for (_span, value) in dyn_style_attr {
            if let Some(value) = value {
                template.push_str(" {};");
                holes.push(quote! {
                  &(#cx, #value).into_attribute(#cx).as_nameless_value_string()
                    .map(|a| leptos::leptos_dom::ssr::escape_attr(&a).to_string())
                    .unwrap_or_default()
                });
            }
        }

        for (_span, name, value) in &style_attrs {
            template.push_str(" {}");
            holes.push(quote! {
              (#cx, #value).into_style(#cx).as_value_string(#name).unwrap_or_default()
            });
        }

        template.push('"');
    }
}

#[allow(clippy::too_many_arguments)]
fn fragment_to_tokens(
    cx: &Ident,
    _span: Span,
    nodes: &[Node],
    lazy: bool,
    parent_type: TagType,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> Option<TokenStream> {
    let mut slots = HashMap::new();
    let has_slots = parent_slots.is_some();

    let mut nodes = nodes
        .iter()
        .filter_map(|node| {
            let node = node_to_tokens(
                cx,
                node,
                parent_type,
                has_slots.then_some(&mut slots),
                global_class,
                None,
            )?;

            Some(quote! {
                #node.into_view(#cx)
            })
        })
        .peekable();

    if nodes.peek().is_none() {
        _ = nodes.collect::<Vec<_>>();
        if let Some(parent_slots) = parent_slots {
            for (slot, mut values) in slots.drain() {
                parent_slots
                    .entry(slot)
                    .and_modify(|entry| entry.append(&mut values))
                    .or_insert(values);
            }
        }
        return None;
    }

    let view_marker = if let Some(marker) = view_marker {
        quote! { .with_view_marker(#marker) }
    } else {
        quote! {}
    };

    let tokens = if lazy {
        quote! {
            {
                leptos::Fragment::lazy(|| [
                    #(#nodes),*
                ].to_vec())
                #view_marker
            }
        }
    } else {
        quote! {
            {
                leptos::Fragment::new([
                    #(#nodes),*
                ].to_vec())
                #view_marker
            }
        }
    };

    if let Some(parent_slots) = parent_slots {
        for (slot, mut values) in slots.drain() {
            parent_slots
                .entry(slot)
                .and_modify(|entry| entry.append(&mut values))
                .or_insert(values);
        }
    }

    Some(tokens)
}

fn node_to_tokens(
    cx: &Ident,
    node: &Node,
    parent_type: TagType,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> Option<TokenStream> {
    match node {
        Node::Fragment(fragment) => fragment_to_tokens(
            cx,
            Span::call_site(),
            &fragment.children,
            true,
            parent_type,
            None,
            global_class,
            view_marker,
        ),
        Node::Comment(_) | Node::Doctype(_) => Some(quote! {}),
        Node::Text(node) => Some(quote! {
            leptos::leptos_dom::html::text(#node)
        }),
        Node::Block(node) => Some(quote! { #node }),
        Node::RawText(r) => {
            let text = r.to_string_best();
            let text = syn::LitStr::new(&text, r.span());
            Some(quote! { #text })
        }
        Node::Element(node) => element_to_tokens(
            cx,
            node,
            parent_type,
            parent_slots,
            global_class,
            view_marker,
        ),
    }
}

fn element_to_tokens(
    cx: &Ident,
    node: &NodeElement,
    mut parent_type: TagType,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> Option<TokenStream> {
    let name = node.name();
    if is_component_node(node) {
        if let Some(slot) = get_slot(node) {
            slot_to_tokens(cx, node, slot, parent_slots, global_class);
            None
        } else {
            Some(component_to_tokens(cx, node, global_class))
        }
    } else {
        let tag = name.to_string();
        // collect close_tag name to emit semantic information for IDE.
        let mut ide_helper_close_tag = IdeTagHelper::new();
        let close_tag = node.close_tag.as_ref().map(|c| &c.name);
        let name = if is_custom_element(&tag) {
            let name = node.name().to_string();
            // link custom ident to name span for IDE docs
            let custom = Ident::new("custom", name.span());
            quote! { leptos::leptos_dom::html::#custom(#cx, leptos::leptos_dom::html::Custom::new(#name)) }
        } else if is_svg_element(&tag) {
            parent_type = TagType::Svg;
            quote! { leptos::leptos_dom::svg::#name(#cx) }
        } else if is_math_ml_element(&tag) {
            parent_type = TagType::Math;
            quote! { leptos::leptos_dom::math::#name(#cx) }
        } else if is_ambiguous_element(&tag) {
            match parent_type {
                TagType::Unknown => {
                    // We decided this warning was too aggressive, but I'll leave it here in case we want it later
                    /* proc_macro_error::emit_warning!(name.span(), "The view macro is assuming this is an HTML element, \
                    but it is ambiguous; if it is an SVG or MathML element, prefix with svg:: or math::"); */
                    quote! {
                        leptos::leptos_dom::html::#name(#cx)
                    }
                }
                TagType::Html => {
                    quote! { leptos::leptos_dom::html::#name(#cx) }
                }
                TagType::Svg => quote! { leptos::leptos_dom::svg::#name(#cx) },
                TagType::Math => {
                    quote! { leptos::leptos_dom::math::#name(#cx) }
                }
            }
        } else {
            parent_type = TagType::Html;
            quote! { leptos::leptos_dom::html::#name(#cx) }
        };

        if let Some(close_tag) = close_tag {
            ide_helper_close_tag.save_tag_completion(close_tag)
        }

        let attrs = node.attributes().iter().filter_map(|node| {
            if let NodeAttribute::Attribute(node) = node {
                let name = node.key.to_string();
                let name = name.trim();
                if name.starts_with("class:")
                    || fancy_class_name(name, cx, node).is_some()
                    || name.starts_with("style:")
                    || fancy_style_name(name, cx, node).is_some()
                {
                    None
                } else {
                    Some(attribute_to_tokens(cx, node, global_class))
                }
            } else {
                None
            }
        });
        let class_attrs = node.attributes().iter().filter_map(|node| {
            if let NodeAttribute::Attribute(node) = node {
                let name = node.key.to_string();
                if let Some((fancy, _, _)) = fancy_class_name(&name, cx, node) {
                    Some(fancy)
                } else if name.trim().starts_with("class:") {
                    Some(attribute_to_tokens(cx, node, global_class))
                } else {
                    None
                }
            } else {
                None
            }
        });
        let style_attrs = node.attributes().iter().filter_map(|node| {
            if let NodeAttribute::Attribute(node) = node {
                let name = node.key.to_string();
                if let Some((fancy, _, _)) = fancy_style_name(&name, cx, node) {
                    Some(fancy)
                } else if name.trim().starts_with("style:") {
                    Some(attribute_to_tokens(cx, node, global_class))
                } else {
                    None
                }
            } else {
                None
            }
        });
        let global_class_expr = match global_class {
            None => quote! {},
            Some(class) => {
                quote! {
                    .classes(
                        #[allow(unused_braces)]
                        #class
                    )
                }
            }
        };
        let children = node.children.iter().map(|node| {
            let (child, is_static) = match node {
                Node::Fragment(fragment) => (
                    fragment_to_tokens(
                        cx,
                        Span::call_site(),
                        &fragment.children,
                        true,
                        parent_type,
                        None,
                        global_class,
                        None,
                    )
                    .unwrap_or({
                        let span = Span::call_site();
                        quote_spanned! {
                            span => leptos::leptos_dom::Unit
                        }
                    }),
                    false,
                ),
                Node::Text(node) => (quote! { #node }, true),
                Node::RawText(node) => {
                    let text = node.to_string_best();
                    let text = syn::LitStr::new(&text, node.span());
                    (quote! { #text }, true)
                }
                Node::Block(node) => (
                    quote! {
                       #node
                    },
                    false,
                ),
                Node::Element(node) => (
                    element_to_tokens(
                        cx,
                        node,
                        parent_type,
                        None,
                        global_class,
                        None,
                    )
                    .unwrap_or_default(),
                    false,
                ),
                Node::Comment(_) | Node::Doctype(_) => (quote! {}, false),
            };
            if is_static {
                quote! {
                    .child(#child)
                }
            } else {
                quote! {
                    .child((#cx, #child))
                }
            }
        });
        let view_marker = if let Some(marker) = view_marker {
            quote! { .with_view_marker(#marker) }
        } else {
            quote! {}
        };
        let ide_helper_close_tag = ide_helper_close_tag.into_iter();
        Some(quote! {
            {
            #(#ide_helper_close_tag)*
            #name
                #(#attrs)*
                #(#class_attrs)*
                #(#style_attrs)*
                #global_class_expr
                #(#children)*
                #view_marker
            }
        })
    }
}

fn attribute_to_tokens(
    cx: &Ident,
    node: &KeyedAttribute,
    global_class: Option<&TokenTree>,
) -> TokenStream {
    let span = node.key.span();
    let name = node.key.to_string();
    if name == "ref" || name == "_ref" || name == "ref_" || name == "node_ref" {
        let value = expr_to_ident(attribute_value(node));
        let node_ref = quote_spanned! { span => node_ref };

        quote! {
            .#node_ref(#value)
        }
    } else if let Some(name) = name.strip_prefix("on:") {
        let handler = attribute_value(node);

        let (event_type, is_custom, is_force_undelegated) =
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
            NodeName::Punctuated(parts) => parts.last().and_then(|last| {
                if last.to_string() == "undelegated" {
                    Some(last)
                } else {
                    None
                }
            }),
            _ => unreachable!(),
        };
        let on = match &node.key {
            NodeName::Punctuated(parts) => &parts[0],
            _ => unreachable!(),
        };
        let on = {
            let span = on.span();
            quote_spanned! {
                span => .on
            }
        };
        let event_type = if is_custom {
            event_type
        } else if let Some(ev_name) = event_name_ident {
            quote! {
                #ev_name
            }
        } else {
            event_type
        };

        let event_type = if is_force_undelegated {
            let undelegated = if let Some(undelegated) = undelegated_ident {
                quote! {
                    #undelegated
                }
            } else {
                quote! { undelegated }
            };
            quote! { ::leptos::ev::#undelegated(::leptos::ev::#event_type) }
        } else {
            quote! { ::leptos::ev::#event_type }
        };

        quote! {
            #on(#event_type, #handler)
        }
    } else if let Some(name) = name.strip_prefix("prop:") {
        let value = attribute_value(node);
        let prop = match &node.key {
            NodeName::Punctuated(parts) => &parts[0],
            _ => unreachable!(),
        };
        let prop = {
            let span = prop.span();
            quote_spanned! {
                span => .prop
            }
        };
        quote! {
            #prop(#name, (#cx, #[allow(unused_braces)] #value))
        }
    } else if let Some(name) = name.strip_prefix("class:") {
        let value = attribute_value(node);
        let class = match &node.key {
            NodeName::Punctuated(parts) => &parts[0],
            _ => unreachable!(),
        };
        let class = {
            let span = class.span();
            quote_spanned! {
                span => .class
            }
        };
        quote! {
            #class(#name, (#cx, #[allow(unused_braces)] #value))
        }
    } else if let Some(name) = name.strip_prefix("style:") {
        let value = attribute_value(node);
        let style = match &node.key {
            NodeName::Punctuated(parts) => &parts[0],
            _ => unreachable!(),
        };
        let style = {
            let span = style.span();
            quote_spanned! {
                span => .style
            }
        };
        quote! {
            #style(#name, (#cx, #[allow(unused_braces)] #value))
        }
    } else {
        let name = name.replacen("attr:", "", 1);

        if let Some((fancy, _, _)) = fancy_class_name(&name, cx, node) {
            return fancy;
        }

        // special case of global_class and class attribute
        if name == "class"
            && global_class.is_some()
            && node.value().and_then(value_to_string).is_none()
        {
            let span = node.key.span();
            proc_macro_error::emit_error!(span, "Combining a global class (view! { cx, class = ... }) \
            and a dynamic `class=` attribute on an element causes runtime inconsistencies. You can \
            toggle individual classes dynamically with the `class:name=value` syntax. \n\nSee this issue \
            for more information and an example: https://github.com/leptos-rs/leptos/issues/773")
        };

        // all other attributes
        let value = match node.value() {
            Some(value) => {
                quote! { #value }
            }
            None => quote_spanned! { span => "" },
        };

        let attr = match &node.key {
            NodeName::Punctuated(parts) => Some(&parts[0]),
            _ => None,
        };
        let attr = if let Some(attr) = attr {
            let span = attr.span();
            quote_spanned! {
                span => .attr
            }
        } else {
            quote! {
                .attr
            }
        };
        quote! {
            #attr(#name, (#cx, #value))
        }
    }
}

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

pub(crate) fn slot_to_tokens(
    cx: &Ident,
    node: &NodeElement,
    slot: &KeyedAttribute,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
) {
    let name = slot.key.to_string();
    let name = name.trim();
    let name = convert_to_snake_case(if name.starts_with("slot:") {
        name.replacen("slot:", "", 1)
    } else {
        node.name().to_string()
    });

    let component_name = ident_from_tag_name(node.name());
    let span = node.name().span();

    let Some(parent_slots) = parent_slots else {
        proc_macro_error::emit_error!(
            span,
            "slots cannot be used inside HTML elements"
        );
        return;
    };

    let attrs = node.attributes().iter().filter_map(|node| {
        if let NodeAttribute::Attribute(node) = node {
            if is_slot(node) {
                None
            } else {
                Some(node)
            }
        } else {
            None
        }
    });

    let props = attrs
        .clone()
        .filter(|attr| {
            !attr.key.to_string().starts_with("bind:")
                && !attr.key.to_string().starts_with("clone:")
        })
        .map(|attr| {
            let name = &attr.key;

            let value = attr
                .value()
                .map(|v| {
                    quote! { #v }
                })
                .unwrap_or_else(|| quote! { #name });

            quote! {
                .#name(#[allow(unused_braces)] #value)
            }
        });

    let items_to_bind = attrs
        .clone()
        .filter_map(|attr| {
            attr.key
                .to_string()
                .strip_prefix("bind:")
                .map(|ident| format_ident!("{ident}", span = attr.key.span()))
        })
        .collect::<Vec<_>>();

    let items_to_clone = attrs
        .clone()
        .filter_map(|attr| {
            attr.key
                .to_string()
                .strip_prefix("clone:")
                .map(|ident| format_ident!("{ident}", span = attr.key.span()))
        })
        .collect::<Vec<_>>();

    let mut slots = HashMap::new();
    let children = if node.children.is_empty() {
        quote! {}
    } else {
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                let marker = format!("<{component_name}/>-children");
                let view_marker = quote! { .with_view_marker(#marker) };
            } else {
                let view_marker = quote! {};
            }
        }

        let children = fragment_to_tokens(
            cx,
            span,
            &node.children,
            true,
            TagType::Unknown,
            Some(&mut slots),
            global_class,
            None,
        );

        if let Some(children) = children {
            let bindables =
                items_to_bind.iter().map(|ident| quote! { #ident, });

            let clonables = items_to_clone
                .iter()
                .map(|ident| quote! { let #ident = #ident.clone(); });

            if bindables.len() > 0 {
                quote! {
                    .children({
                        #(#clonables)*

                        move |#cx, #(#bindables)*| #children #view_marker
                    })
                }
            } else {
                quote! {
                    .children({
                        #(#clonables)*

                        Box::new(move |#cx| #children #view_marker)
                    })
                }
            }
        } else {
            quote! {}
        }
    };

    let slots = slots.drain().map(|(slot, values)| {
        let slot = Ident::new(&slot, span);
        if values.len() > 1 {
            quote! {
                .#slot([
                    #(#values)*
                ].to_vec())
            }
        } else {
            let value = &values[0];
            quote! { .#slot(#value) }
        }
    });

    let slot = quote! {
        #component_name::builder()
            #(#props)*
            #(#slots)*
            #children
            .build()
            .into(),
    };

    parent_slots
        .entry(name)
        .and_modify(|entry| entry.push(slot.clone()))
        .or_insert(vec![slot]);
}

pub(crate) fn component_to_tokens(
    cx: &Ident,
    node: &NodeElement,
    global_class: Option<&TokenTree>,
) -> TokenStream {
    let name = node.name();
    #[cfg(debug_assertions)]
    let component_name = ident_from_tag_name(node.name());
    let span = node.name().span();

    let attrs = node.attributes().iter().filter_map(|node| {
        if let NodeAttribute::Attribute(node) = node {
            Some(node)
        } else {
            None
        }
    });

    let props = attrs
        .clone()
        .filter(|attr| {
            !attr.key.to_string().starts_with("bind:")
                && !attr.key.to_string().starts_with("clone:")
                && !attr.key.to_string().starts_with("on:")
        })
        .map(|attr| {
            let name = &attr.key;

            let value = attr
                .value()
                .map(|v| {
                    quote! { #v }
                })
                .unwrap_or_else(|| quote! { #name });

            quote! {
                .#name(#[allow(unused_braces)] #value)
            }
        });

    let items_to_bind = attrs
        .clone()
        .filter_map(|attr| {
            attr.key
                .to_string()
                .strip_prefix("bind:")
                .map(|ident| format_ident!("{ident}", span = attr.key.span()))
        })
        .collect::<Vec<_>>();

    let items_to_clone = attrs
        .clone()
        .filter_map(|attr| {
            attr.key
                .to_string()
                .strip_prefix("clone:")
                .map(|ident| format_ident!("{ident}", span = attr.key.span()))
        })
        .collect::<Vec<_>>();

    let events = attrs
        .filter(|attr| attr.key.to_string().starts_with("on:"))
        .map(|attr| {
            let (event_type, handler) = event_from_attribute_node(attr, true);

            quote! {
                .on(#event_type, #handler)
            }
        })
        .collect::<Vec<_>>();

    let mut slots = HashMap::new();
    let children = if node.children.is_empty() {
        quote! {}
    } else {
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                let marker = format!("<{component_name}/>-children");
                let view_marker = quote! { .with_view_marker(#marker) };
            } else {
                let view_marker = quote! {};
            }
        }

        let children = fragment_to_tokens(
            cx,
            span,
            &node.children,
            true,
            TagType::Unknown,
            Some(&mut slots),
            global_class,
            None,
        );

        if let Some(children) = children {
            let bindables =
                items_to_bind.iter().map(|ident| quote! { #ident, });

            let clonables = items_to_clone
                .iter()
                .map(|ident| quote! { let #ident = #ident.clone(); });

            if bindables.len() > 0 {
                quote! {
                    .children({
                        #(#clonables)*

                        move |#cx, #(#bindables)*| #children #view_marker
                    })
                }
            } else {
                quote! {
                    .children({
                        #(#clonables)*

                        Box::new(move |#cx| #children #view_marker)
                    })
                }
            }
        } else {
            quote! {}
        }
    };

    let slots = slots.drain().map(|(slot, values)| {
        let slot = Ident::new(&slot, span);
        if values.len() > 1 {
            quote! {
                .#slot(vec![
                    #(#values)*
                ])
            }
        } else {
            let value = &values[0];
            quote! { .#slot(#value) }
        }
    });

    #[allow(unused_mut)] // used in debug
    let mut component = quote! {
        ::leptos::component_view(
            &#name,
            #cx,
            ::leptos::component_props_builder(&#name)
                #(#props)*
                #(#slots)*
                #children
                .build()
        )
    };

    #[cfg(debug_assertions)]
    IdeTagHelper::add_component_completion(&mut component, node);

    if events.is_empty() {
        component
    } else {
        quote! {
            #component.into_view(#cx)
            #(#events)*
        }
    }
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

    #[allow(unused_variables)]
    let (name, name_undelegated) = parse_event(&event_name);

    let event_type = TYPED_EVENTS
        .binary_search(&name)
        .map(|_| (name))
        .unwrap_or(CUSTOM_EVENT);

    let Ok(event_type) = event_type.parse::<TokenStream>() else {
        abort!(attr.key, "couldn't parse event name");
    };

    let event_type = if force_undelegated || name_undelegated {
        quote! { ::leptos::leptos_dom::ev::undelegated(::leptos::leptos_dom::ev::#event_type) }
    } else {
        quote! { ::leptos::leptos_dom::ev::#event_type }
    };
    (event_type, handler)
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

fn is_slot(node: &KeyedAttribute) -> bool {
    let key = node.key.to_string();
    let key = key.trim();
    key == "slot" || key.starts_with("slot:")
}

fn get_slot(node: &NodeElement) -> Option<&KeyedAttribute> {
    node.attributes().iter().find_map(|node| {
        if let NodeAttribute::Attribute(node) = node {
            if is_slot(node) {
                Some(node)
            } else {
                None
            }
        } else {
            None
        }
    })
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
    // Keep list alphabetized for binary search
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
    cx: &Ident,
    node: &'a KeyedAttribute,
) -> Option<(TokenStream, String, &'a Expr)> {
    // special case for complex class names:
    // e.g., Tailwind `class=("mt-[calc(100vh_-_3rem)]", true)`
    if name == "class" {
        if let Some(Tuple(tuple)) = node.value() {
            if tuple.elems.len() == 2 {
                let span = node.key.span();
                let class = quote_spanned! {
                    span => .class
                };
                let class_name = &tuple.elems[0];
                let class_name = if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s),
                    ..
                }) = class_name
                {
                    s.value()
                } else {
                    proc_macro_error::emit_error!(
                        class_name.span(),
                        "class name must be a string literal"
                    );
                    Default::default()
                };
                let value = &tuple.elems[1];
                return Some((
                    quote! {
                        #class(#class_name, (#cx, #value))
                    },
                    class_name,
                    value,
                ));
            } else {
                proc_macro_error::emit_error!(
                    tuple.span(),
                    "class tuples must have two elements."
                )
            }
        }
    }
    None
}

fn fancy_style_name<'a>(
    name: &str,
    cx: &Ident,
    node: &'a KeyedAttribute,
) -> Option<(TokenStream, String, &'a Expr)> {
    // special case for complex dynamic style names:
    if name == "style" {
        if let Some(Tuple(tuple)) = node.value() {
            if tuple.elems.len() == 2 {
                let span = node.key.span();
                let style = quote_spanned! {
                    span => .style
                };
                let style_name = &tuple.elems[0];
                let style_name = if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s),
                    ..
                }) = style_name
                {
                    s.value()
                } else {
                    proc_macro_error::emit_error!(
                        style_name.span(),
                        "style name must be a string literal"
                    );
                    Default::default()
                };
                let value = &tuple.elems[1];
                return Some((
                    quote! {
                        #style(#style_name, (#cx, #value))
                    },
                    style_name,
                    value,
                ));
            } else {
                proc_macro_error::emit_error!(
                    tuple.span(),
                    "style tuples must have two elements."
                )
            }
        }
    }
    None
}

/// Helper type to emit semantic info about tags, for IDE.
/// Implement `IntoIterator` with `Item="let _ = foo::docs;"`.
///
/// `IdeTagHelper` uses warning instead of errors everywhere,
/// it's aim is to add usability, not introduce additional typecheck in `view`/`template` code.
/// On stable `emit_warning` don't produce anything.
pub(crate) struct IdeTagHelper(Vec<TokenStream>);

// TODO: Unhandled cases:
// - svg::div, my_elements::foo - tags with custom paths, that doesnt look like component
// - my_component::Foo - components with custom paths
// - html:div - tags punctuated by `:`
// - {div}, {"div"} - any rust expression
impl IdeTagHelper {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    /// Save stmts for tag name.
    /// Emit warning if tag is component.
    pub fn save_tag_completion(&mut self, name: &NodeName) {
        let tag_name = name.to_string();
        if is_component_tag_name(&tag_name) {
            proc_macro_error::emit_warning!(
                name.span(),
                "BUG: Component tag is used in regular tag completion."
            );
        }
        for path in Self::completion_stmts(name) {
            self.0.push(quote! {
                    let _ = #path;
            });
        }
    }

    /// Save stmts for open and close tags.
    /// Emit warning if tag is component.
    pub fn save_element_completion(&mut self, node: &NodeElement) {
        self.save_tag_completion(node.name());
        if let Some(close_tag) = node.close_tag.as_ref().map(|c| &c.name) {
            self.save_tag_completion(close_tag)
        }
    }

    /// Add completion to the closing tag of the component.
    ///
    /// In order to ensure that generics are passed through correctly in the
    /// current builder pattern, this clones the whole component constructor,
    /// but it will never be used.
    ///
    /// ```no_build
    /// if false {
    ///     close_tag(cx, unreachable!())
    /// }
    /// else {
    ///     open_tag(open_tag.props().slots().children().build())
    /// }
    /// ```
    #[cfg(debug_assertions)]
    pub fn add_component_completion(
        component: &mut TokenStream,
        node: &NodeElement,
    ) {
        // emit ide helper info
        if node.close_tag.is_some() {
            let constructor = component.clone();
            *component = quote! {
                if false {
                    #[allow(unreachable_code)]
                    #constructor
                } else {
                    #component
                }
            }
        }
    }

    /// Returns `syn::Path`-like `TokenStream` to the fn in docs.
    /// If tag name is `Component` returns `None`.
    fn create_regular_tag_fn_path(name: &Ident) -> TokenStream {
        let tag_name = name.to_string();
        let namespace = if crate::view::is_svg_element(&tag_name) {
            quote! { leptos::leptos_dom::svg }
        } else if crate::view::is_math_ml_element(&tag_name) {
            quote! { leptos::leptos_dom::math }
        } else {
            // todo: check is html, and emit_warning in case of custom tag
            quote! { leptos::leptos_dom::html }
        };
        quote!( #namespace::#name)
    }

    /// Returns `syn::Path`-like `TokenStream` to the `custom` section in docs.
    fn create_custom_tag_fn_path(span: Span) -> TokenStream {
        let custom_ident = Ident::new("custom", span);
        quote! {leptos::leptos_dom::html::#custom_ident::<leptos::leptos_dom::html::Custom>}
    }

    // Extract from NodeName completion idents.
    // Custom tags (like foo-bar-baz) is mapped
    // to vec!["custom", "custom",.. ] for each token in tag, even for "-".
    // Only last ident from `Path` is used.
    fn completion_stmts(name: &NodeName) -> Vec<TokenStream> {
        match name {
            NodeName::Block(_) => vec![],
            NodeName::Punctuated(c) => c
                .pairs()
                .flat_map(|c| {
                    let mut idents =
                        vec![Self::create_custom_tag_fn_path(c.value().span())];
                    if let Some(p) = c.punct() {
                        idents.push(Self::create_custom_tag_fn_path(p.span()))
                    }
                    idents
                })
                .collect(),
            NodeName::Path(e) => e
                .path
                .segments
                .last()
                .map(|p| &p.ident)
                .map(Self::create_regular_tag_fn_path)
                .into_iter()
                .collect(),
        }
    }
}

impl IntoIterator for IdeTagHelper {
    type Item = TokenStream;
    type IntoIter = <Vec<TokenStream> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
