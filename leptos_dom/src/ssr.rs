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
/// # use leptos::*;
/// let html = render_to_string(|cx| view! { cx,
///   <p>"Hello, world!"</p>
/// });
/// // static HTML includes some hydration info
/// assert_eq!(html, "<p id=\"_0-1\">Hello, world!</p>");
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

  html.into()
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
  // TODO can remove id_before_suspense entirely now
  let fragments = fragments.map(|(fragment_id, _, html)| {
    format!(
      r#"
              <template id="{fragment_id}f">{html}</template>
              <script>
                  var placeholder = document.getElementById("_{fragment_id}");
                  var tpl = document.getElementById("{fragment_id}f");
                  placeholder.textContent = "";
                  placeholder.append(tpl.content.cloneNode(true));
              </script>
              "#
    )
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
            format!(r#"<!--hk={}|leptos-{name}-start-->{}<!--hk={}|leptos-{name}-end-->"#,
              HydrationCtx::to_string(&node.id, false),
              content(),
              HydrationCtx::to_string(&node.id, true),
              name = to_kebab_case(&node.name)
            ).into()
          } else {
            format!(
              r#"{}<!--hk={}-->"#,
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
                  "<!--hk={}|leptos-unit-->",
                  HydrationCtx::to_string(&u.id, true)
                )
                .into()
              }

              #[cfg(not(debug_assertions))]
              format!("<!--hk={}-->", HydrationCtx::to_string(&u.id, true))
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
                        "<!--hk={}|leptos-each-item-start-->{}<!\
                         --hk={}|leptos-each-item-end-->",
                        HydrationCtx::to_string(&id, false),
                        content(),
                        HydrationCtx::to_string(&id, true),
                      )
                    }

                    #[cfg(not(debug_assertions))]
                    format!(
                      "{}<!--hk={}-->",
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
                r#"<!--hk={}|leptos-{name}-start-->{}<!--hk={}|leptos-{name}-end-->"#,
                HydrationCtx::to_string(&id, false),
                content(),
                HydrationCtx::to_string(&id, true),
              ).into()
            } else {
              let _ = name;

              format!(
                r#"{}<!--hk={}-->"#,
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

          let mut inner_html = None;

          let attrs = el
            .attrs
            .into_iter()
            .filter_map(|(name, value)| -> Option<Cow<'static, str>> {
              if value.is_empty() {
                Some(format!(" {name}").into())
              } else if name == "inner_html" {
                inner_html = Some(value);
                None
              } else {
                Some(
                  format!(
                    " {name}=\"{}\"",
                    html_escape::encode_double_quoted_attribute(&value)
                  )
                  .into(),
                )
              }
            })
            .join("");

          if el.is_void {
            format!("<{tag_name}{attrs}/>").into()
          } else if let Some(inner_html) = inner_html {
            format!("<{tag_name}{attrs}>{inner_html}</{tag_name}>").into()
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

#[cfg(debug_assertions)]
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
