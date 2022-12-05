use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, ExprPath};
use syn_rsx::{Node, NodeAttribute, NodeElement, NodeName};

use crate::{is_component_node, Mode};

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
) -> TokenStream {
  if nodes.is_empty() {
    let span = Span::call_site();
    quote_spanned! {
        span => leptos::Unit.into_view(#cx)
    }
  } else if nodes.len() == 1 {
    node_to_tokens(cx, &nodes[0], mode)
  } else {
    fragment_to_tokens(cx, Span::call_site(), nodes, mode)
  }
}

fn fragment_to_tokens(
  cx: &Ident,
  span: Span,
  nodes: &[Node],
  mode: Mode,
) -> TokenStream {
  let nodes = nodes.iter().map(|node| {
    let node = node_to_tokens(cx, node, mode);
    let span = node.span();
    quote_spanned! {
        span => #node.into_view(#cx),
    }
  });
  quote_spanned! {
      span => {
          vec![
              #(#nodes)*
          ].into_view(#cx)
      }
  }
}

fn node_to_tokens(cx: &Ident, node: &Node, mode: Mode) -> TokenStream {
  match node {
    Node::Fragment(fragment) => {
      fragment_to_tokens(cx, Span::call_site(), &fragment.children, mode)
    }
    Node::Comment(_) | Node::Doctype(_) => quote! {},
    Node::Text(node) => {
      let span = node.value.span();
      let value = node.value.as_ref();
      quote_spanned! {
          span => text(#value)
      }
    }
    Node::Block(node) => {
      let span = node.value.span();
      let value = node.value.as_ref();
      quote_spanned! {
          span => #value.into_view(#cx)
      }
    }
    Node::Attribute(node) => attribute_to_tokens(cx, node, mode),
    Node::Element(node) => element_to_tokens(cx, node, mode),
  }
}

fn element_to_tokens(
  cx: &Ident,
  node: &NodeElement,
  mode: Mode,
) -> TokenStream {
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
        }
        Node::Text(node) => {
          let span = node.value.span();
          let value = node.value.as_ref();
          quote_spanned! {
              span => #[allow(unused_braces)] #value
          }
        }
        Node::Block(node) => {
          let span = node.value.span();
          let value = node.value.as_ref();
          quote_spanned! {
              span => #[allow(unused_braces)] #value
          }
        }
        Node::Element(node) => element_to_tokens(cx, node, mode),
        Node::Comment(_) | Node::Doctype(_) | Node::Attribute(_) => quote! {},
      };
      quote! {
          .child(cx, #child)
      }
    });
    quote_spanned! {
        span => #name
            #(#attrs)*
            #(#children)*
            .into_view(#cx)
    }
  }
}

fn attribute_to_tokens(
  cx: &Ident,
  node: &NodeAttribute,
  mode: Mode,
) -> TokenStream {
  let span = node.key.span();
  let name = node.key.to_string();
  if name == "ref" || name == "_ref" {
    //if mode != Mode::Ssr {
    let value = node
      .value
      .as_ref()
      .and_then(|expr| expr_to_ident(expr))
      .expect("'_ref' needs to be passed a variable name");
    quote_spanned! {
        span => #[allow(unused_braces)]
                .ref(#value)
    }
    /* } else {
        todo!()
    } */
  } else if let Some(name) = name.strip_prefix("on:") {
    //if mode != Mode::Ssr {
    let span = name.span();
    let handler = node
      .value
      .as_ref()
      .expect("event listener attributes need a value")
      .as_ref();
    let event_type = TYPED_EVENTS
      .iter()
      .find(|e| **e == name)
      .copied()
      .unwrap_or("Custom");
    let event_type = event_type
      .parse::<TokenStream>()
      .expect("couldn't parse event name");

    quote_spanned! {
        span => .on(leptos::ev::#event_type, #handler)
    }
    /* } else {
        todo!()
    } */
  } else if let Some(name) = name.strip_prefix("prop:") {
    let value = node
      .value
      .as_ref()
      .expect("prop: attributes need a value")
      .as_ref();
    //if mode != Mode::Ssr {
    quote_spanned! {
        span => .prop(#cx, #name, #[allow(unused_braces)] #value)
    }
    /* } else {
        todo!()
    } */
  } else if let Some(name) = name.strip_prefix("class:") {
    let value = node
      .value
      .as_ref()
      .expect("class: attributes need a value")
      .as_ref();
    //if mode != Mode::Ssr {
    quote_spanned! {
        span => .class(#cx, #name, #[allow(unused_braces)] #value)
    }
    /* } else {
        todo!()
    } */
  } else {
    let name = name.replacen("attr:", "", 1);
    let value = match node.value.as_ref() {
      Some(value) => {
        let value = value.as_ref();
        let span = value.span();
        quote_spanned! { span => Some(#value) }
      }
      None => quote! { None },
    };
    //if mode != Mode::Ssr {
    quote_spanned! {
        span => .attr(#name, (#cx, #value))
    }
    /* } else {
        quote! { }
    } */
  }
}

fn component_to_tokens(
  cx: &Ident,
  node: &NodeElement,
  mode: Mode,
) -> TokenStream {
  let name = &node.name;
  let component_name = ident_from_tag_name(&node.name);
  let component_name_str = name.to_string();
  let span = node.name.span();
  let component_props_name =
    Ident::new(&format!("{component_name}Props"), span);

  let children = if node.children.is_empty() {
    quote! {}
  } else if node.children.len() == 1 {
    let child = component_child(cx, &node.children[0], mode);
    quote_spanned! { span => .children(Box::new(move || vec![#child])) }
  } else {
    let children = node
      .children
      .iter()
      .map(|node| component_child(cx, node, mode));
    quote_spanned! { span => .children(Box::new(move || vec![#(#children),*])) }
  };

  let props = node
    .attributes
    .iter()
    .filter_map(|node| {
      if let Node::Attribute(node) = node {
        Some(node)
      } else {
        None
      }
    })
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

  quote_spanned! {
      span => #name(
          cx,
          #component_props_name::builder()
              #(#props)*
              #children
              .build(),
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
    }
    _ => node_to_tokens(cx, node, mode),
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
