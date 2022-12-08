use cfg_if::cfg_if;
use itertools::Itertools;
use std::{borrow::Cow, fmt::Display};

use crate::{hydration::HydrationCtx, CoreComponent, TopoId, View};

#[cfg(not(all(target_arch = "wasm32", feature = "web")))]
impl View {
  /// Consumes the node and renders it into an HTML string.
  pub fn render_to_string(self) -> Cow<'static, str> {
    match self {
      View::Text(node) => node.content,
      View::Component(node) => {
        let content = node
          .children
          .into_iter()
          .map(|node| node.render_to_string())
          .join("");
        cfg_if! {
          if #[cfg(debug_assertions)] {
            format!(r#"<template id="{}"></template>{content}<template id="{}"></template>"#,
              HydrationCtx::to_string(node.id, false),
              HydrationCtx::to_string(node.id, true)
            ).into()
          } else {
            format!(
              r#"{content}<template id="{}"></template>"#,
              HydrationCtx::to_string(node.id, true)
            ).into()
          }
        }
      }
      View::CoreComponent(node) => {
        let (id, wrap, content) = match node {
          CoreComponent::Unit(u) => (
            u.id,
            false,
            format!(
              "<template id={}></template>",
              HydrationCtx::to_string(u.id, true)
            )
            .into(),
          ),
          CoreComponent::DynChild(node) => {
            let child = node.child.take();
            (
              node.id,
              true,
              if let Some(child) = *child {
                child.render_to_string()
              } else {
                "".into()
              },
            )
          }
          CoreComponent::Each(node) => {
            let children = node.children.take();

            (
              node.id,
              true,
              children
                .into_iter()
                .flatten()
                .map(|node| {
                  let id = node.id;

                  let content = node.child.render_to_string();

                  #[cfg(debug_assertions)]
                  return format!(
                    "<template id=\"{}\"></template>{content}<template \
                     id=\"{}\"></template>",
                    HydrationCtx::to_string(id, false),
                    HydrationCtx::to_string(id, true),
                  );

                  #[cfg(not(debug_assertions))]
                  return format!(
                    "{content}<template id=\"{}c\"></template>",
                    HydrationCtx::to_string(id, true)
                  );
                })
                .join("")
                .into(),
            )
          }
        };

        if wrap {
          cfg_if! {
            if #[cfg(debug_assertions)] {
              format!(
                r#"<template id="{}"></template>{content}<template id="{}"></template>"#,
                HydrationCtx::to_string(id, false),
                HydrationCtx::to_string(id, true),
              ).into()
            } else {
              format!(
                r#"{content}<template id="{}c"></template>"#,
                HydrationCtx::to_string(id, true)
              ).into()
            }
          }
        } else {
          content
        }
      }
      View::Element(el) => {
        let tag_name = el.name;
        let mut has_id = false;
        let mut attrs = el
          .attrs
          .into_iter()
          .map(|(name, value)| -> Cow<'static, str> {
            if value.is_empty() {
              format!(" {name}").into()
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
          attrs.push_str(&format!(" id=\"_{}\"", el.id));
        }

        if el.is_void {
          format!("<{tag_name}{attrs}/>").into()
        } else {
          let children = el
            .children
            .into_iter()
            .map(|node| node.render_to_string())
            .join("");

          format!("<{tag_name}{attrs}>{children}</{tag_name}>").into()
        }
      }
    }
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
