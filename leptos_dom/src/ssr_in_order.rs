use crate::{CoreComponent, HydrationCtx, View};
use cfg_if::cfg_if;
use futures::Stream;
use itertools::Itertools;
use leptos_reactive::{create_runtime, run_scope_undisposed, Scope};
use std::{borrow::Cow, future::Future, pin::Pin};

/// Renders an in-order HTML stream, pausing at `<Suspense/>` components.
pub async fn render_to_stream_in_order(
  view: impl FnOnce(Scope) -> View + 'static,
) -> impl Stream<Item = String> {
  HydrationCtx::reset_id();

  // create the runtime
  let runtime = create_runtime();

  let (chunks, y, z) = run_scope_undisposed(runtime, |cx| {
    let view = view(cx);
    view.into_stream_chunks(cx)
  });

  eprintln!("{chunks:#?}");

  let (mut tx, rx) = futures::channel::mpsc::channel(1);
  leptos_reactive::spawn_local(async move {
    let mut buffer = String::new();
    for chunk in chunks {
      match chunk {
        StreamChunk::Sync(sync) => buffer.push_str(&sync),
        StreamChunk::Async(suspended) => {
          // add static HTML before the Suspense and stream it down
          _ = tx.try_send(std::mem::take(&mut buffer));

          // send the inner stream
          let suspended = suspended.await;
          _ = tx.try_send(suspended.to_string());
        }
      }
    }
    // send final sync chunk
    _ = tx.try_send(std::mem::take(&mut buffer));
  });

  rx
}

enum StreamChunk {
  Sync(Cow<'static, str>),
  Async(Pin<Box<dyn Future<Output = String>>>),
}

impl std::fmt::Debug for StreamChunk {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      StreamChunk::Sync(data) => write!(f, "StreamChunk::Sync({data:?})"),
      StreamChunk::Async(_) => write!(f, "StreamChunk::Async(_)"),
    }
  }
}

impl View {
  fn into_stream_chunks(self, cx: Scope) -> Vec<StreamChunk> {
    let mut chunks = Vec::new();
    self.into_stream_chunks_helper(cx, &mut chunks);
    chunks
  }

  fn into_stream_chunks_helper(self, cx: Scope, chunks: &mut Vec<StreamChunk>) {
    match self {
      // TODO this doesn't handle nested <Suspense/>, but would stream out of order
      View::Suspense(id, node) => {
        eprintln!("rendering Suspense");
        let id = id.to_string();
        chunks.push(StreamChunk::Sync(
          format!("<!--suspense-open-{id}-->").into(),
        ));
        if let Some((_, fragment)) = cx.take_pending_fragment(&id) {
          chunks.push(StreamChunk::Async(fragment));
        }
        chunks.push(StreamChunk::Sync(
          format!("<!--suspense-close-{id}-->").into(),
        ));
      }
      View::Text(node) => chunks.push(StreamChunk::Sync(node.content)),
      View::Component(node) => {
        eprintln!("\nrendering Component");
        cfg_if! {
          if #[cfg(debug_assertions)] {
            let name = crate::to_kebab_case(&node.name);
            chunks.push(StreamChunk::Sync(format!(r#"<!--hk={}|leptos-{name}-start-->"#, HydrationCtx::to_string(&node.id, false)).into()));
            for child in node.children {
                child.into_stream_chunks_helper(cx, chunks);
            }
            chunks.push(StreamChunk::Sync(format!(r#"<!--hk={}|leptos-{name}-end-->"#, HydrationCtx::to_string(&node.id, true)).into()));
          } else {
            for child in node.children {
                child.into_stream_chunks_helper(cx, chunks);
            }
            chunks.push(StreamChunk::Sync(format!(r#"<!--hk={}-->"#, HydrationCtx::to_string(&node.id, true))))
          }
        }
      }
      View::Element(el) => {
        eprintln!("rendering Element");
        if let Some(prerendered) = el.prerendered {
          chunks.push(StreamChunk::Sync(prerendered))
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
            chunks
              .push(StreamChunk::Sync(format!("<{tag_name}{attrs}/>").into()));
          } else if let Some(inner_html) = inner_html {
            chunks.push(StreamChunk::Sync(
              format!("<{tag_name}{attrs}>{inner_html}</{tag_name}>").into(),
            ));
          } else {
            chunks
              .push(StreamChunk::Sync(format!("<{tag_name}{attrs}>").into()));
            for child in el.children {
              child.into_stream_chunks_helper(cx, chunks);
            }

            chunks.push(StreamChunk::Sync(format!("</{tag_name}>").into()));
          }
        }
      }
      View::Transparent(_) => {}
      View::CoreComponent(node) => {
        let (id, name, wrap, content) = match node {
          CoreComponent::Unit(u) => (
            u.id.clone(),
            "",
            false,
            Box::new(move |chunks: &mut Vec<StreamChunk>| {
              #[cfg(debug_assertions)]
              {
                chunks.push(StreamChunk::Sync(
                  format!(
                    "<!--hk={}|leptos-unit-->",
                    HydrationCtx::to_string(&u.id, true)
                  )
                  .into(),
                ));
              }

              #[cfg(not(debug_assertions))]
              chunks.push(StreamChunk::Sync(
                format!("<!--hk={}-->", HydrationCtx::to_string(&u.id, true))
                  .into(),
              ));
            }) as Box<dyn FnOnce(&mut Vec<StreamChunk>)>,
          ),
          CoreComponent::DynChild(node) => {
            let child = node.child.take();
            (
              node.id,
              "dyn-child",
              true,
              Box::new(move |chunks: &mut Vec<StreamChunk>| {
                if let Some(child) = *child {
                  // On debug builds, `DynChild` has two marker nodes,
                  // so there is no way for the text to be merged with
                  // surrounding text when the browser parses the HTML,
                  // but in release, `DynChild` only has a trailing marker,
                  // and the browser automatically merges the dynamic text
                  // into one single node, so we need to artificially make the
                  // browser create the dynamic text as it's own text node
                  if let View::Text(t) = child {
                    chunks.push(if !cfg!(debug_assertions) {
                      StreamChunk::Sync(format!("<!>{}", t.content).into())
                    } else {
                      StreamChunk::Sync(t.content)
                    });
                  } else {
                    child.into_stream_chunks_helper(cx, chunks);
                  }
                }
              }) as Box<dyn FnOnce(&mut Vec<StreamChunk>)>,
            )
          }
          CoreComponent::Each(node) => {
            let children = node.children.take();
            (
              node.id,
              "each",
              true,
              Box::new(move |chunks: &mut Vec<StreamChunk>| {
                for node in children.into_iter().flatten() {
                  let id = node.id;

                  #[cfg(debug_assertions)]
                  {
                    chunks.push(StreamChunk::Sync(
                      format!(
                        "<!--hk={}|leptos-each-item-start-->",
                        HydrationCtx::to_string(&id, false)
                      )
                      .into(),
                    ));
                    node.child.into_stream_chunks_helper(cx, chunks);
                    chunks.push(StreamChunk::Sync(
                      format!(
                        "<!--hk={}|leptos-each-item-end-->",
                        HydrationCtx::to_string(&id, true)
                      )
                      .into(),
                    ));
                  }
                }
              }) as Box<dyn FnOnce(&mut Vec<StreamChunk>)>,
            )
          }
        };

        if wrap {
          cfg_if! {
            if #[cfg(debug_assertions)] {
              chunks.push(StreamChunk::Sync(format!("<!--hk={}|leptos-{name}-start-->", HydrationCtx::to_string(&id, false)).into()));
              content(chunks);
              chunks.push(StreamChunk::Sync(format!("<!--hk={}|leptos-{name}-end-->", HydrationCtx::to_string(&id, true)).into()));
            } else {
              let _ = name;
              content(chunks);
              chunks.push(StreamChunk::Sync(format!("<!--hk={}-->", HydrationCtx::to_string(&id, true)).into()))
            }
          }
        } else {
          content(chunks);
        }
      }
    }
  }
}
