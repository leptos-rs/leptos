use crate::{attribute_value, Mode};
use leptos_hot_reload::parsing::{is_component_node, value_to_string};
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned};
use syn::{spanned::Spanned, Expr, ExprLit, ExprPath, Lit};
use syn_rsx::{Node, NodeAttribute, NodeElement, NodeName, NodeValueExpr};

#[derive(Clone, Copy)]
enum TagType {
    Unknown,
    Html,
    Svg,
    Math,
}

const TYPED_EVENTS: [&str; 126] = [
    "afterprint",
    "beforeprint",
    "beforeunload",
    "gamepadconnected",
    "gamepaddisconnected",
    "hashchange",
    "languagechange",
    "message",
    "messageerror",
    "offline",
    "online",
    "pagehide",
    "pageshow",
    "popstate",
    "rejectionhandled",
    "storage",
    "unhandledrejection",
    "unload",
    "abort",
    "animationcancel",
    "animationend",
    "animationiteration",
    "animationstart",
    "auxclick",
    "beforeinput",
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
    "cuechange",
    "dblclick",
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
    "gotpointercapture",
    "input",
    "invalid",
    "keydown",
    "keypress",
    "keyup",
    "load",
    "loadeddata",
    "loadedmetadata",
    "loadstart",
    "lostpointercapture",
    "mousedown",
    "mouseenter",
    "mouseleave",
    "mousemove",
    "mouseout",
    "mouseover",
    "mouseup",
    "pause",
    "play",
    "playing",
    "pointercancel",
    "pointerdown",
    "pointerenter",
    "pointerleave",
    "pointermove",
    "pointerout",
    "pointerover",
    "pointerup",
    "progress",
    "ratechange",
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
    "volumechange",
    "waiting",
    "webkitanimationend",
    "webkitanimationiteration",
    "webkitanimationstart",
    "webkittransitionend",
    "wheel",
    "DOMContentLoaded",
    "devicemotion",
    "deviceorientation",
    "orientationchange",
    "copy",
    "cut",
    "paste",
    "fullscreenchange",
    "fullscreenerror",
    "pointerlockchange",
    "pointerlockerror",
    "readystatechange",
    "visibilitychange",
];

pub(crate) fn render_view(
    cx: &Ident,
    nodes: &[Node],
    mode: Mode,
    global_class: Option<&TokenTree>,
    call_site: Option<String>,
) -> TokenStream {
    if mode == Mode::Ssr {
        match nodes.len() {
            0 => {
                let span = Span::call_site();
                quote_spanned! {
                    span => leptos::leptos_dom::Unit
                }
            }
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
            0 => {
                let span = Span::call_site();
                quote_spanned! {
                    span => leptos::leptos_dom::Unit
                }
            }
            1 => node_to_tokens(
                cx,
                &nodes[0],
                TagType::Unknown,
                global_class,
                call_site,
            ),
            _ => fragment_to_tokens(
                cx,
                Span::call_site(),
                nodes,
                true,
                TagType::Unknown,
                global_class,
                call_site,
            ),
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
        Node::Comment(_) | Node::Doctype(_) | Node::Attribute(_) => quote! {},
        Node::Text(node) => {
            let value = node.value.as_ref();
            quote! {
                leptos::leptos_dom::html::text(#value)
            }
        }
        Node::Block(node) => {
            let value = node.value.as_ref();
            quote! {
                #[allow(unused_braces)]
                #value
            }
        }
        Node::Element(node) => {
            root_element_to_tokens_ssr(cx, node, global_class, view_marker)
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
            leptos::Fragment::lazy(|| vec![
                #(#nodes),*
            ])
            #view_marker
        }
    }
}

fn root_element_to_tokens_ssr(
    cx: &Ident,
    node: &NodeElement,
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> TokenStream {
    if is_component_node(node) {
        component_to_tokens(cx, node, global_class)
    } else {
        let mut exprs_for_compiler = Vec::<TokenStream>::new();

        let mut template = String::new();
        let mut holes = Vec::new();
        let mut chunks = Vec::new();
        element_to_tokens_ssr(
            cx,
            node,
            &mut template,
            &mut holes,
            &mut chunks,
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
                    quote! {
                        leptos::leptos_dom::html::StringOrView::String(#template.into())
                    }
                } else {
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

        let tag_name = node.name.to_string();
        let is_custom_element = is_custom_element(&tag_name);
        let typed_element_name = if is_custom_element {
            Ident::new("Custom", node.name.span())
        } else {
            let camel_cased = camel_case_tag_name(
                &tag_name.replace("svg::", "").replace("math::", ""),
            );
            Ident::new(&camel_cased, node.name.span())
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
        quote! {
        {
            #(#exprs_for_compiler)*
            ::leptos::HtmlElement::from_chunks(#cx, #full_name, [#(#chunks),*])#view_marker
        }
        }
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
    template: &mut String,
    holes: &mut Vec<TokenStream>,
    chunks: &mut Vec<SsrElementChunks>,
    exprs_for_compiler: &mut Vec<TokenStream>,
    is_root: bool,
    global_class: Option<&TokenTree>,
) {
    if is_component_node(node) {
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
        let tag_name = node
            .name
            .to_string()
            .replace("svg::", "")
            .replace("math::", "");
        let is_script_or_style = tag_name == "script" || tag_name == "style";
        template.push('<');
        template.push_str(&tag_name);

        let mut inner_html = None;

        for attr in &node.attributes {
            if let Node::Attribute(attr) = attr {
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
            .attributes
            .iter()
            .find(|node| matches!(node, Node::Attribute(attr) if attr.key.to_string() == "id"))
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

        if is_self_closing(node) {
            template.push_str("/>");
        } else {
            template.push('>');

            if let Some(inner_html) = inner_html {
                template.push_str("{}");
                let value = inner_html.as_ref();

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
                                template,
                                holes,
                                chunks,
                                exprs_for_compiler,
                                false,
                                global_class,
                            );
                        }
                        Node::Text(text) => {
                            if let Some(value) = value_to_string(&text.value) {
                                let value = if is_script_or_style {
                                    value.into()
                                } else {
                                    html_escape::encode_safe(&value)
                                };
                                template.push_str(
                                    &value
                                        .replace('{', "{{")
                                        .replace('}', "}}"),
                                );
                            } else {
                                template.push_str("{}");
                                let value = text.value.as_ref();

                                holes.push(quote! {
                                  #value.into_view(#cx).render_to_string(#cx)
                                })
                            }
                        }
                        Node::Block(block) => {
                            if let Some(value) = value_to_string(&block.value) {
                                template.push_str(&value);
                            } else {
                                let value = block.value.as_ref();

                                if !template.is_empty() {
                                    chunks.push(SsrElementChunks::String {
                                        template: std::mem::take(template),
                                        holes: std::mem::take(holes),
                                    })
                                }
                                chunks.push(SsrElementChunks::View(quote! {
                                  {#value}.into_view(#cx)
                                }));
                            }
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
            template.push_str(&node.name.to_string());
            template.push('>');
        }
    }
}

// returns `inner_html`
fn attribute_to_tokens_ssr<'a>(
    cx: &Ident,
    node: &'a NodeAttribute,
    template: &mut String,
    holes: &mut Vec<TokenStream>,
    exprs_for_compiler: &mut Vec<TokenStream>,
    global_class: Option<&TokenTree>,
) -> Option<&'a NodeValueExpr> {
    let name = node.key.to_string();
    if name == "ref" || name == "_ref" || name == "ref_" || name == "node_ref" {
        // ignore refs on SSR
    } else if let Some(name) = name.strip_prefix("on:") {
        let handler = attribute_value(node);
        let (event_type, _, _) = parse_event_name(name);

        exprs_for_compiler.push(quote! {
            leptos::leptos_dom::helpers::ssr_event_listener(::leptos::ev::#event_type, #handler);
        })
    } else if name.strip_prefix("prop:").is_some()
        || name.strip_prefix("class:").is_some()
    {
        // ignore props for SSR
        // ignore classes: we'll handle these separately
    } else if name == "inner_html" {
        return node.value.as_ref();
    } else {
        let name = name.replacen("attr:", "", 1);

        // special case of global_class and class attribute
        if name == "class"
            && global_class.is_some()
            && node.value.as_ref().and_then(value_to_string).is_none()
        {
            let span = node.key.span();
            proc_macro_error::emit_error!(span, "Combining a global class (view! { cx, class = ... }) \
            and a dynamic `class=` attribute on an element causes runtime inconsistencies. You can \
            toggle individual classes dynamically with the `class:name=value` syntax. \n\nSee this issue \
            for more information and an example: https://github.com/leptos-rs/leptos/issues/773")
        };

        if name != "class" {
            template.push(' ');

            if let Some(value) = node.value.as_ref() {
                if let Some(value) = value_to_string(value) {
                    template.push_str(&name);
                    template.push_str("=\"");
                    template.push_str(&value);
                    template.push('"');
                } else {
                    template.push_str("{}");
                    let value = value.as_ref();
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
        .attributes
        .iter()
        .filter_map(|a| match a {
            Node::Attribute(attr) if attr.key.to_string() == "class" => {
                attr.value.as_ref().and_then(value_to_string)
            }
            _ => None,
        })
        .chain(Some(static_global_class))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    let dyn_class_attr = node
        .attributes
        .iter()
        .filter_map(|a| {
            if let Node::Attribute(a) = a {
                if a.key.to_string() == "class" {
                    if a.value.as_ref().and_then(value_to_string).is_some()
                        || fancy_class_name(&a.key.to_string(), cx, a).is_some()
                    {
                        None
                    } else {
                        Some((a.key.span(), &a.value))
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
        .attributes
        .iter()
        .filter_map(|node| {
            if let Node::Attribute(node) = node {
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

        template.push_str(&static_class_attr);

        for (_span, value) in dyn_class_attr {
            if let Some(value) = value {
                template.push_str(" {}");
                let value = value.as_ref();
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

fn fragment_to_tokens(
    cx: &Ident,
    _span: Span,
    nodes: &[Node],
    lazy: bool,
    parent_type: TagType,
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> TokenStream {
    let nodes = nodes.iter().map(|node| {
        let node = node_to_tokens(cx, node, parent_type, global_class, None);

        quote! {
            #node.into_view(#cx)
        }
    });

    let view_marker = if let Some(marker) = view_marker {
        quote! { .with_view_marker(#marker) }
    } else {
        quote! {}
    };

    if lazy {
        quote! {
            {
                leptos::Fragment::lazy(|| vec![
                    #(#nodes),*
                ])
                #view_marker
            }
        }
    } else {
        quote! {
            {
                leptos::Fragment::new(vec![
                    #(#nodes),*
                ])
                #view_marker
            }
        }
    }
}

fn node_to_tokens(
    cx: &Ident,
    node: &Node,
    parent_type: TagType,
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> TokenStream {
    match node {
        Node::Fragment(fragment) => fragment_to_tokens(
            cx,
            Span::call_site(),
            &fragment.children,
            true,
            parent_type,
            global_class,
            view_marker,
        ),
        Node::Comment(_) | Node::Doctype(_) => quote! {},
        Node::Text(node) => {
            let value = node.value.as_ref();
            quote! {
                leptos::leptos_dom::html::text(#value)
            }
        }
        Node::Block(node) => {
            let value = node.value.as_ref();
            quote! { #value }
        }
        Node::Attribute(node) => attribute_to_tokens(cx, node, global_class),
        Node::Element(node) => {
            element_to_tokens(cx, node, parent_type, global_class, view_marker)
        }
    }
}

fn element_to_tokens(
    cx: &Ident,
    node: &NodeElement,
    mut parent_type: TagType,
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> TokenStream {
    if is_component_node(node) {
        component_to_tokens(cx, node, global_class)
    } else {
        let tag = node.name.to_string();
        let name = if is_custom_element(&tag) {
            let name = node.name.to_string();
            quote! { leptos::leptos_dom::html::custom(#cx, leptos::leptos_dom::html::Custom::new(#name)) }
        } else if is_svg_element(&tag) {
            let name = &node.name;
            parent_type = TagType::Svg;
            quote! { leptos::leptos_dom::svg::#name(#cx) }
        } else if is_math_ml_element(&tag) {
            let name = &node.name;
            parent_type = TagType::Math;
            quote! { leptos::leptos_dom::math::#name(#cx) }
        } else if is_ambiguous_element(&tag) {
            let name = &node.name;
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
            let name = &node.name;
            parent_type = TagType::Html;
            quote! { leptos::leptos_dom::html::#name(#cx) }
        };
        let attrs = node.attributes.iter().filter_map(|node| {
            if let Node::Attribute(node) = node {
                if node.key.to_string().trim().starts_with("class:") {
                    None
                } else {
                    Some(attribute_to_tokens(cx, node, global_class))
                }
            } else {
                None
            }
        });
        let class_attrs = node.attributes.iter().filter_map(|node| {
            if let Node::Attribute(node) = node {
                if node.key.to_string().trim().starts_with("class:") {
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
                        global_class,
                        None,
                    ),
                    false,
                ),
                Node::Text(node) => {
                    if let Some(primitive) = value_to_string(&node.value) {
                        (quote! { #primitive }, true)
                    } else {
                        let value = node.value.as_ref();
                        (
                            quote! {
                                #[allow(unused_braces)] #value
                            },
                            false,
                        )
                    }
                }
                Node::Block(node) => {
                    if let Some(primitive) = value_to_string(&node.value) {
                        (quote! { #primitive }, true)
                    } else {
                        let value = node.value.as_ref();
                        (
                            quote! {
                                #[allow(unused_braces)] #value
                            },
                            false,
                        )
                    }
                }
                Node::Element(node) => (
                    element_to_tokens(
                        cx,
                        node,
                        parent_type,
                        global_class,
                        None,
                    ),
                    false,
                ),
                Node::Comment(_) | Node::Doctype(_) | Node::Attribute(_) => {
                    (quote! {}, false)
                }
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
        quote! {
            #name
                #(#attrs)*
                #(#class_attrs)*
                #global_class_expr
                #(#children)*
                #view_marker
        }
    }
}

fn attribute_to_tokens(
    cx: &Ident,
    node: &NodeAttribute,
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
                if last == "undelegated" {
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
    } else {
        let name = name.replacen("attr:", "", 1);

        if let Some((fancy, _, _)) = fancy_class_name(&name, cx, node) {
            return fancy;
        }

        // special case of global_class and class attribute
        if name == "class"
            && global_class.is_some()
            && node.value.as_ref().and_then(value_to_string).is_none()
        {
            let span = node.key.span();
            proc_macro_error::emit_error!(span, "Combining a global class (view! { cx, class = ... }) \
            and a dynamic `class=` attribute on an element causes runtime inconsistencies. You can \
            toggle individual classes dynamically with the `class:name=value` syntax. \n\nSee this issue \
            for more information and an example: https://github.com/leptos-rs/leptos/issues/773")
        };

        // all other attributes
        let value = match node.value.as_ref() {
            Some(value) => {
                let value = value.as_ref();

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

    let event_type = TYPED_EVENTS
        .iter()
        .find(|e| **e == name)
        .copied()
        .unwrap_or("Custom");
    let is_custom = event_type == "Custom";

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

pub(crate) fn component_to_tokens(
    cx: &Ident,
    node: &NodeElement,
    global_class: Option<&TokenTree>,
) -> TokenStream {
    let name = &node.name;
    let component_name = ident_from_tag_name(&node.name);
    let span = node.name.span();

    let attrs = node.attributes.iter().filter_map(|node| {
        if let Node::Attribute(node) = node {
            Some(node)
        } else {
            None
        }
    });

    let props = attrs
        .clone()
        .filter(|attr| {
            !attr.key.to_string().starts_with("clone:")
                && !attr.key.to_string().starts_with("on:")
        })
        .map(|attr| {
            let name = &attr.key;

            let value = attr
                .value
                .as_ref()
                .map(|v| {
                    let v = v.as_ref();
                    quote! { #v }
                })
                .unwrap_or_else(|| quote! { #name });

            quote! {
                .#name(#[allow(unused_braces)] #value)
            }
        });

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
            global_class,
            None,
        );

        let clonables = items_to_clone
            .iter()
            .map(|ident| quote! { let #ident = #ident.clone(); });

        quote! {
            .children({
                #(#clonables)*

                Box::new(move |#cx| #children #view_marker)
            })
        }
    };

    let component = quote! {
        #name(
            #cx,
            ::leptos::component_props_builder(&#name)
                #(#props)*
                #children
                .build()
        )
    };

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
    attr: &NodeAttribute,
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
        .iter()
        .find(|e| **e == name)
        .copied()
        .unwrap_or("Custom");

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

fn is_custom_element(tag: &str) -> bool {
    tag.contains('-')
}

fn is_self_closing(node: &NodeElement) -> bool {
    // self-closing tags
    // https://developer.mozilla.org/en-US/docs/Glossary/Empty_element
    matches!(
        node.name.to_string().as_str(),
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
    )
}

fn camel_case_tag_name(tag_name: &str) -> String {
    let mut chars = tag_name.chars();
    let first = chars.next();
    let underscore = if tag_name == "option" || tag_name == "use" {
        "_"
    } else {
        ""
    };
    first
        .map(|f| f.to_ascii_uppercase())
        .into_iter()
        .chain(chars)
        .collect::<String>()
        + underscore
}

fn is_svg_element(tag: &str) -> bool {
    matches!(
        tag,
        "animate"
            | "animateMotion"
            | "animateTransform"
            | "circle"
            | "clipPath"
            | "defs"
            | "desc"
            | "discard"
            | "ellipse"
            | "feBlend"
            | "feColorMatrix"
            | "feComponentTransfer"
            | "feComposite"
            | "feConvolveMatrix"
            | "feDiffuseLighting"
            | "feDisplacementMap"
            | "feDistantLight"
            | "feDropShadow"
            | "feFlood"
            | "feFuncA"
            | "feFuncB"
            | "feFuncG"
            | "feFuncR"
            | "feGaussianBlur"
            | "feImage"
            | "feMerge"
            | "feMergeNode"
            | "feMorphology"
            | "feOffset"
            | "fePointLight"
            | "feSpecularLighting"
            | "feSpotLight"
            | "feTile"
            | "feTurbulence"
            | "filter"
            | "foreignObject"
            | "g"
            | "hatch"
            | "hatchpath"
            | "image"
            | "line"
            | "linearGradient"
            | "marker"
            | "mask"
            | "metadata"
            | "mpath"
            | "path"
            | "pattern"
            | "polygon"
            | "polyline"
            | "radialGradient"
            | "rect"
            | "set"
            | "stop"
            | "svg"
            | "switch"
            | "symbol"
            | "text"
            | "textPath"
            | "tspan"
            | "use"
            | "use_"
            | "view"
    )
}

fn is_math_ml_element(tag: &str) -> bool {
    matches!(
        tag,
        "math"
            | "mi"
            | "mn"
            | "mo"
            | "ms"
            | "mspace"
            | "mtext"
            | "menclose"
            | "merror"
            | "mfenced"
            | "mfrac"
            | "mpadded"
            | "mphantom"
            | "mroot"
            | "mrow"
            | "msqrt"
            | "mstyle"
            | "mmultiscripts"
            | "mover"
            | "mprescripts"
            | "msub"
            | "msubsup"
            | "msup"
            | "munder"
            | "munderover"
            | "mtable"
            | "mtd"
            | "mtr"
            | "maction"
            | "annotation"
            | "semantics"
    )
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
    node: &'a NodeAttribute,
) -> Option<(TokenStream, String, &'a Expr)> {
    // special case for complex class names:
    // e.g., Tailwind `class=("mt-[calc(100vh_-_3rem)]", true)`
    if name == "class" {
        if let Some(expr) = node.value.as_ref() {
            if let syn::Expr::Tuple(tuple) = expr.as_ref() {
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
    }
    None
}
