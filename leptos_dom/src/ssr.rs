use cfg_if::cfg_if;
use itertools::Itertools;
use std::{borrow::Cow, fmt::Display};

use crate::{CoreComponent, View};

#[cfg(not(all(target_arch = "wasm32", feature = "web")))]
impl View {
  /// Consumes the node and renders it into an HTML string.
  pub fn render_to_string(self) -> Cow<'static, str> {
    self.render_to_string_with_id(Default::default())
  }

  fn render_to_string_with_id(self, id: TopoId) -> Cow<'static, str> {
    match self {
      View::Text(node) => node.content,
      View::Component(node) => {
        let depth = id.first_child().depth;
        let content = node
          .children
          .into_iter()
          .enumerate()
          .map(|(offset, node)| {
            node.render_to_string_with_id(TopoId { depth, offset })
          })
          .join("");
        cfg_if! {
          if #[cfg(debug_assertions)] {
            format!(r#"<template id="{id}o"/>{content}<template id="{id}c"/>"#).into()
          } else {
            format!(r#"{content}<template id="{id}c"/>"#).into()
          }
        }
      }
      View::CoreComponent(node) => {
        let content = match node {
          CoreComponent::Unit(_) => " ".into(),
          CoreComponent::DynChild(node) => {
            let child = node.child.take();
            if let Some(child) = *child {
              child.render_to_string_with_id(id.first_child())
            } else {
              " ".into()
            }
          }
          CoreComponent::Each(node) => {
            let children = node.children.take();
            let depth = id.first_child().depth;
            children
              .into_iter()
              .flatten()
              .enumerate()
              .map(|(offset, node)| {
                node
                  .child
                  .render_to_string_with_id(TopoId { depth, offset })
              })
              .join("")
              .into()
          }
        };

        //node.children.into_iter().enumerate().map(|(offset, node)| node.render_to_string_with_id(TopoId { depth: children_depth, offset })).join("");
        cfg_if! {
          if #[cfg(debug_assertions)] {
            format!(r#"<template id="{id}o"/>{content}<template id="{id}c"/>"#).into()
          } else {
            format!(r#"{content}<template id="{id}c"/>"#).into()
          }
        }
      }
      View::Element(el) => {
        let tag_name = el.name;
        let mut has_id = false;
        let mut attrs = el
          .attrs
          .into_iter()
          .map(|(name, value)| {
            if value.is_empty() {
              name
            } else {
              if name == "id" {
                has_id = true;
              }
              format!(
                " {name}=\"{}\"",
                html_escape::encode_double_quoted_attribute(&value)
              )
              .into()
            }
          })
          .join("");

        if !has_id && el.dynamic {
          attrs.push_str(&format!(" id=\"{id}\""));
        }

        if el.is_void {
          format!("<{tag_name}{attrs}/>").into()
        } else {
          let depth = id.depth + 1;
          let starting_offset = id.offset + 1;
          let children = el
            .children
            .into_iter()
            .enumerate()
            .map(|(offset, node)| {
              node.render_to_string_with_id(TopoId {
                depth,
                offset: starting_offset + offset,
              })
            })
            .join("");
          format!("<{tag_name}{attrs}>{children}</{tag_name}>").into()
        }
      }
    }
  }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
struct TopoId {
  depth: usize,
  offset: usize,
}

impl TopoId {
  fn first_child(&self) -> TopoId {
    TopoId {
      depth: self.depth + 1,
      offset: 0,
    }
  }

  fn next_sibling(&self) -> TopoId {
    TopoId {
      depth: self.depth,
      offset: self.offset + 1,
    }
  }
}

impl Display for TopoId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!(
      "{}-{}-{}",
      self.depth,
      self.offset,
      self.depth + self.offset
    ))
  }
}

#[cfg(test)]
mod tests {
  #[test]
  fn simple_ssr_test() {
    use leptos::*;

    _ = create_scope(create_runtime(), |cx| {
      let (value, set_value) = create_signal(cx, 0);
      let rendered = view! {
        cx,
        <div>
            <button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
            <span>"Value: " {move || value.get().to_string()} "!"</span>
            <button on:click=move |_| set_value.update(|value| *value += 1)>"+1"</button>
        </div>
    }.render_to_string();

      assert_eq!(
        rendered,
        "<div><button id=\"1-1-2\">-1</button><span>Value: <template \
         id=\"2-4-6o\"/> <template id=\"2-4-6c\"/>!</span><button \
         id=\"1-3-4\">+1</button></div>"
      );
    });
  }

  #[test]
  fn ssr_test_with_components() {
    use leptos::*;

    #[component]
    fn Counter(cx: Scope, initial_value: i32) -> View {
      let (value, set_value) = create_signal(cx, initial_value);
      view! {
          cx,
          <div>
              <button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
              <span>"Value: " {move || value.get().to_string()} "!"</span>
              <button on:click=move |_| set_value.update(|value| *value += 1)>"+1"</button>
          </div>
      }
    }

    _ = create_scope(create_runtime(), |cx| {
      let rendered = view! {
          cx,
          <div class="counters">
              <Counter initial_value=1/>
              <Counter initial_value=2/>
          </div>
      }
      .render_to_string();

      assert_eq!(
        rendered,
        "<div class=\"counters\"><template id=\"1-1-2o\"/><div><button \
         id=\"3-1-4\">-1</button><span>Value: <template id=\"4-4-8o\"/> \
         <template id=\"4-4-8c\"/>!</span><button \
         id=\"3-3-6\">+1</button></div><template id=\"1-1-2c\"/><template \
         id=\"1-2-3o\"/><div><button id=\"3-1-4\">-1</button><span>Value: \
         <template id=\"4-4-8o\"/> <template id=\"4-4-8c\"/>!</span><button \
         id=\"3-3-6\">+1</button></div><template id=\"1-2-3c\"/></div>"
      );
    });
  }

  #[test]
  fn test_classes() {
    use leptos::*;

    _ = create_scope(create_runtime(), |cx| {
      let (value, set_value) = create_signal(cx, 5);
      let rendered = view! {
          cx,
          <div class="my big" class:a={move || value.get() > 10} class:red=true class:car={move || value.get() > 1}></div>
      }.render_to_string();

      assert_eq!(
        rendered,
        "<div class=\"my big red car\" id=\"0-0-0\"></div>"
      );
    });
  }
}
