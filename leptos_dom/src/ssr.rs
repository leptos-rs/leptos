#![cfg(not(all(target_arch = "wasm32", feature = "web")))]

use crate::{CoreComponent, HydrationCtx, IntoView, View};
use cfg_if::cfg_if;
use futures::{stream::FuturesUnordered, Stream, StreamExt};
use itertools::Itertools;
use leptos_reactive::*;
use std::borrow::Cow;

/// Renders the given function to a static HTML string.
///
/// ```
/// # cfg_if::cfg_if! { if #[cfg(not(any(feature = "csr", feature = "hydrate")))] {
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view;
/// let html = render_to_string(|cx| view! { cx,
///   <p>"Hello, world!"</p>
/// });
/// assert_eq!(html, r#"<p>Hello, world!</p>"#);
/// # }}
/// ```
pub fn render_to_string<F, N>(f: F) -> String
where
  F: FnOnce(Scope) -> N + 'static,
  N: IntoView,
{
  let runtime = leptos_reactive::create_runtime();
  HydrationCtx::reset_id();

  let html = leptos_reactive::run_scope(runtime, |cx| {
    f(cx).into_view(cx).render_to_string(cx)
  });

  runtime.dispose();

  #[cfg(debug_assertions)]
  {
    format!("<style>[leptos]{{display:none;}}</style>{html}")
  }

  #[cfg(not(debug_assertions))]
  format!("<style>l-m{{display:none;}}</style>{html}")
}

/// Renders a function to a stream of HTML strings.
///
/// This renders:
/// 1) the application shell
///   a) HTML for everything that is not under a `<Suspense/>`,
///   b) the `fallback` for any `<Suspense/>` component that is not already resolved, and
///   c) JavaScript necessary to receive streaming [Resource](leptos_reactive::Resource) data.
/// 2) streaming [Resource](leptos_reactive::Resource) data. Resources begin loading on the
///    server and are sent down to the browser to resolve. On the browser, if the app sees that
///    it is waiting for a resource to resolve from the server, it doesn't run it initially.
/// 3) HTML fragments to replace each `<Suspense/>` fallback with its actual data as the resources
///    read under that `<Suspense/>` resolve.
pub fn render_to_stream(
  view: impl FnOnce(Scope) -> View + 'static,
) -> impl Stream<Item = String> {
  render_to_stream_with_prefix(view, |_| "".into())
}

/// Renders a function to a stream of HTML strings. After the `view` runs, the `prefix` will run with
/// the same scope. This can be used to generate additional HTML that has access to the same `Scope`.
///
/// This renders:
/// 1) the prefix
/// 2) the application shell
///   a) HTML for everything that is not under a `<Suspense/>`,
///   b) the `fallback` for any `<Suspense/>` component that is not already resolved, and
///   c) JavaScript necessary to receive streaming [Resource](leptos_reactive::Resource) data.
/// 3) streaming [Resource](leptos_reactive::Resource) data. Resources begin loading on the
///    server and are sent down to the browser to resolve. On the browser, if the app sees that
///    it is waiting for a resource to resolve from the server, it doesn't run it initially.
/// 4) HTML fragments to replace each `<Suspense/>` fallback with its actual data as the resources
///    read under that `<Suspense/>` resolve.
pub fn render_to_stream_with_prefix(
  view: impl FnOnce(Scope) -> View + 'static,
  prefix: impl FnOnce(Scope) -> Cow<'static, str> + 'static,
) -> impl Stream<Item = String> {
  let (stream, runtime, _) =
    render_to_stream_with_prefix_undisposed(view, prefix);
  runtime.dispose();
  stream
}

/// Renders a function to a stream of HTML strings and returns the [Scope] and [Runtime] that were created, so
/// they can be disposed when appropriate. After the `view` runs, the `prefix` will run with
/// the same scope. This can be used to generate additional HTML that has access to the same `Scope`.
///
/// This renders:
/// 1) the prefix
/// 2) the application shell
///   a) HTML for everything that is not under a `<Suspense/>`,
///   b) the `fallback` for any `<Suspense/>` component that is not already resolved, and
///   c) JavaScript necessary to receive streaming [Resource](leptos_reactive::Resource) data.
/// 3) streaming [Resource](leptos_reactive::Resource) data. Resources begin loading on the
///    server and are sent down to the browser to resolve. On the browser, if the app sees that
///    it is waiting for a resource to resolve from the server, it doesn't run it initially.
/// 4) HTML fragments to replace each `<Suspense/>` fallback with its actual data as the resources
///    read under that `<Suspense/>` resolve.
pub fn render_to_stream_with_prefix_undisposed(
  view: impl FnOnce(Scope) -> View + 'static,
  prefix: impl FnOnce(Scope) -> Cow<'static, str> + 'static,
) -> (impl Stream<Item = String>, RuntimeId, ScopeId) {
  HydrationCtx::reset_id();

  // create the runtime
  let runtime = create_runtime();

  let (
    (shell, prefix, pending_resources, pending_fragments, serializers),
    scope,
    _,
  ) = run_scope_undisposed(runtime, {
    move |cx| {
      // the actual app body/template code
      // this does NOT contain any of the data being loaded asynchronously in resources
      let shell = view(cx).render_to_string(cx);

      let resources = cx.pending_resources();
      let pending_resources = serde_json::to_string(&resources).unwrap();
      let prefix = prefix(cx);

      let shell = {
        #[cfg(debug_assertions)]
        {
          format!("<style>[leptos]{{display:none;}}</style>{shell}")
        }

        #[cfg(not(debug_assertions))]
        format!("<style>l-m{{display:none;}}</style>{shell}")
      };

      (
        shell,
        prefix,
        pending_resources,
        cx.pending_fragments(),
        cx.serialization_resolvers(),
      )
    }
  });

  let fragments = FuturesUnordered::new();
  for (fragment_id, (key_before, fut)) in pending_fragments {
    fragments.push(async move { (fragment_id, key_before, fut.await) })
  }

  // resources and fragments
  // stream HTML for each <Suspense/> as it resolves
  let fragments = fragments.map(|(fragment_id, id_before_suspense, html)| {
    cfg_if! {
      if #[cfg(debug_assertions)] {
        // Debug-mode <Suspense/>-replacement code
        format!(
          r#"
                  <template id="{fragment_id}f">{html}</template>
                  <script>
                      var start = document.getElementById("_{fragment_id}o");
                      var end = document.getElementById("_{fragment_id}c");
                      var range = new Range();
                      range.setStartBefore(start.nextSibling.nextSibling);
                      range.setEndAfter(end.previousSibling.previousSibling);
                      range.deleteContents();
                      var tpl = document.getElementById("{fragment_id}f");
                      end.parentNode.insertBefore(tpl.content.cloneNode(true), end.previousSibling);
                  </script>
                  "#
        )
      } else {
        // Release-mode <Suspense/>-replacement code
        format!(
          r#"
                  <template id="{fragment_id}f">{html}</template>
                  <script>
                      var start = document.getElementById("_{id_before_suspense}");
                      var end = document.getElementById("_{fragment_id}");
                      var range = new Range();
                      range.setStartAfter(start);
                      range.setEndBefore(end);
                      range.deleteContents();
                      var tpl = document.getElementById("{fragment_id}f");
                      end.parentNode.insertBefore(tpl.content.cloneNode(true), end.previousSibling);
                  </script>
                  "#
        )
      }
    }
  });
  // stream data for each Resource as it resolves
  let resources = serializers.map(|(id, json)| {
    let id = serde_json::to_string(&id).unwrap();
    format!(
      r#"<script>
                  if(__LEPTOS_RESOURCE_RESOLVERS.get({id})) {{
                      __LEPTOS_RESOURCE_RESOLVERS.get({id})({json:?})
                  }} else {{
                      __LEPTOS_RESOLVED_RESOURCES.set({id}, {json:?});
                  }}
              </script>"#,
    )
  });

  // HTML for the view function and script to store resources
  let stream = futures::stream::once(async move {
    format!(
      r#"
              {prefix}
              {shell}
              <script>
                  __LEPTOS_PENDING_RESOURCES = {pending_resources};
                  __LEPTOS_RESOLVED_RESOURCES = new Map();
                  __LEPTOS_RESOURCE_RESOLVERS = new Map();
              </script>
          "#
    )
  })
  // TODO these should be combined again in a way that chains them appropriately
  // such that individual resources can resolve before all fragments are done
  .chain(fragments)
  .chain(resources);

  (stream, runtime, scope)
}

impl View {
  /// Consumes the node and renders it into an HTML string.
  pub fn render_to_string(self, _cx: Scope) -> Cow<'static, str> {
    self.render_to_string_helper()
  }

  pub(crate) fn render_to_string_helper(self) -> Cow<'static, str> {
    match self {
      View::Text(node) => node.content,
      View::Component(node) => {
        let content = || {
          node
            .children
            .into_iter()
            .map(|node| node.render_to_string_helper())
            .join("")
        };
        cfg_if! {
          if #[cfg(debug_assertions)] {
            format!(r#"<leptos-{name}-start leptos id="{}"></leptos-{name}-start>{}<leptos-{name}-end leptos id="{}"></leptos-{name}-end>"#,
              HydrationCtx::to_string(&node.id, false),
              content(),
              HydrationCtx::to_string(&node.id, true),
              name = to_kebab_case(&node.name)
            ).into()
          } else {
            format!(
              r#"{}<l-m id="{}"></l-m>"#,
              content(),
              HydrationCtx::to_string(&node.id, true)
            ).into()
          }
        }
      }
      View::CoreComponent(node) => {
        let (id, name, wrap, content) = match node {
          CoreComponent::Unit(u) => (
            u.id.clone(),
            "",
            false,
            Box::new(move || {
              #[cfg(debug_assertions)]
              {
                format!(
                  "<leptos-unit leptos id={}></leptos-unit>",
                  HydrationCtx::to_string(&u.id, true)
                )
                .into()
              }

              #[cfg(not(debug_assertions))]
              format!("<l-m id={}></l-m>", HydrationCtx::to_string(&u.id, true))
                .into()
            }) as Box<dyn FnOnce() -> Cow<'static, str>>,
          ),
          CoreComponent::DynChild(node) => {
            let child = node.child.take();
            (
              node.id,
              "dyn-child",
              true,
              Box::new(move || {
                if let Some(child) = *child {
                  // On debug builds, `DynChild` has two marker nodes,
                  // so there is no way for the text to be merged with
                  // surrounding text when the browser parses the HTML,
                  // but in release, `DynChild` only has a trailing marker,
                  // and the browser automatically merges the dynamic text
                  // into one single node, so we need to artificially make the
                  // browser create the dynamic text as it's own text node
                  if let View::Text(t) = child {
                    if !cfg!(debug_assertions) {
                      format!("<!>{}", t.content).into()
                    } else {
                      t.content
                    }
                  } else {
                    child.render_to_string_helper()
                  }
                } else {
                  "".into()
                }
              }) as Box<dyn FnOnce() -> Cow<'static, str>>,
            )
          }
          CoreComponent::Each(node) => {
            let children = node.children.take();

            (
              node.id,
              "each",
              true,
              Box::new(move || {
                children
                  .into_iter()
                  .flatten()
                  .map(|node| {
                    let id = node.id;

                    let content = || node.child.render_to_string_helper();

                    #[cfg(debug_assertions)]
                    {
                      format!(
                        "<leptos-each-item-start leptos \
                         id=\"{}\"></\
                         leptos-each-item-start>{}<leptos-each-item-end \
                         leptos id=\"{}\"></leptos-each-item-end>",
                        HydrationCtx::to_string(&id, false),
                        content(),
                        HydrationCtx::to_string(&id, true),
                      )
                    }

                    #[cfg(not(debug_assertions))]
                    format!(
                      "{}<l-m id=\"{}\"></l-m>",
                      content(),
                      HydrationCtx::to_string(&id, true)
                    )
                  })
                  .join("")
                  .into()
              }) as Box<dyn FnOnce() -> Cow<'static, str>>,
            )
          }
        };

        if wrap {
          cfg_if! {
            if #[cfg(debug_assertions)] {
              format!(
                r#"<leptos-{name}-start leptos id="{}"></leptos-{name}-start>{}<leptos-{name}-end leptos id="{}"></leptos-{name}-end>"#,
                HydrationCtx::to_string(&id, false),
                content(),
                HydrationCtx::to_string(&id, true),
              ).into()
            } else {
              let _ = name;

              format!(
                r#"{}<l-m id="{}"></l-m>"#,
                content(),
                HydrationCtx::to_string(&id, true)
              ).into()
            }
          }
        } else {
          content()
        }
      }
      View::Element(el) => {
        if let Some(prerendered) = el.prerendered {
          prerendered
        } else {
          let tag_name = el.name;

          let attrs = el
            .attrs
            .into_iter()
            .map(|(name, value)| -> Cow<'static, str> {
              if value.is_empty() {
                format!(" {name}").into()
              } else {
                format!(
                  " {name}=\"{}\"",
                  html_escape::encode_double_quoted_attribute(&value)
                )
                .into()
              }
            })
            .join("");

          if el.is_void {
            format!("<{tag_name}{attrs}/>").into()
          } else {
            let children = el
              .children
              .into_iter()
              .map(|node| node.render_to_string_helper())
              .join("");

            format!("<{tag_name}{attrs}>{children}</{tag_name}>").into()
          }
        }
      }
      View::Transparent(_) => Default::default(),
    }
  }
}

fn to_kebab_case(name: &str) -> String {
  if name.is_empty() {
    return String::new();
  }

  let mut new_name = String::with_capacity(name.len() + 8);

  let mut chars = name.chars();

  new_name.push(
    chars
      .next()
      .map(|mut c| {
        if c.is_ascii() {
          c.make_ascii_lowercase();
        }

        c
      })
      .unwrap(),
  );

  for mut char in chars {
    if char.is_ascii_uppercase() {
      char.make_ascii_lowercase();

      new_name.push('-');
    }

    new_name.push(char);
  }

  new_name
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
