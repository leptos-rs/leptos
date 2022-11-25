use cfg_if::cfg_if;
use std::borrow::Cow;

/// Encodes strings to be used as text in HTML by escaping `&`, `<`, and `>`.
pub fn escape_text(text: &str) -> Cow<'_, str> {
    html_escape::encode_text(text)
}

/// Encodes strings to be used as attribute values in HTML by escaping `&`, `<`, `>`, and `"`.
pub fn escape_attr(text: &str) -> Cow<'_, str> {
    html_escape::encode_double_quoted_attribute(text)
}

cfg_if! {
    if #[cfg(not(any(feature = "csr", feature = "hydrate")))] {
        use leptos_reactive::*;

        use crate::Element;
        use futures::{stream::FuturesUnordered, Stream, StreamExt};

        /// Renders a component to a static HTML string.
        ///
        /// ```
        /// # cfg_if::cfg_if! { if #[cfg(not(any(feature = "csr", feature = "hydrate")))] {
        /// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view;
        /// let html = render_to_string(|cx| view! { cx,
        ///   <p>"Hello, world!"</p>
        /// });
        /// assert_eq!(html, r#"<p data-hk="0-0">Hello, world!</p>"#);
        /// # }}
        /// ```
        pub fn render_to_string(view: impl FnOnce(Scope) -> Element + 'static) -> String {
            let runtime = create_runtime();
            let html = run_scope(runtime, move |cx| view(cx));
            runtime.dispose();
            html
        }

        /// Renders a component to a stream of HTML strings.
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
        pub fn render_to_stream(view: impl FnOnce(Scope) -> Element + 'static) -> impl Stream<Item = String> {
            // create the runtime
            let runtime = create_runtime();

            let ((shell, pending_resources, pending_fragments, serializers), _, disposer) =
                run_scope_undisposed(runtime, {
                    move |cx| {
                        // the actual app body/template code
                        // this does NOT contain any of the data being loaded asynchronously in resources
                        let shell = view(cx);

                        let resources = cx.all_resources();
                        let pending_resources = serde_json::to_string(&resources).unwrap();

                        (
                            shell,
                            pending_resources,
                            cx.pending_fragments(),
                            cx.serialization_resolvers(),
                        )
                    }
                });

            let fragments = FuturesUnordered::new();
            for (fragment_id, fut) in pending_fragments {
                fragments.push(async move { (fragment_id, fut.await) })
            }

            // resources and fragments
            let resources_and_fragments = futures::stream::select(
                // stream data for each Resource as it resolves
                serializers.map(|(id, json)| {
                    let id = serde_json::to_string(&id).unwrap();
                    format!(
                        r#"<script>
                                if(__LEPTOS_RESOURCE_RESOLVERS.get({id})) {{
                                    console.log("(create_resource) calling resolver");
                                    __LEPTOS_RESOURCE_RESOLVERS.get({id})({json:?})
                                }} else {{
                                    console.log("(create_resource) saving data for resource creation");
                                    __LEPTOS_RESOLVED_RESOURCES.set({id}, {json:?});
                                }}
                            </script>"#,
                    )
                }),
                // stream HTML for each <Suspense/> as it resolves
                fragments.map(|(fragment_id, html)| {
                    format!(
                        r#"
                            <template id="{fragment_id}">{html}</template>
                            <script>
                                var frag = document.querySelector(`[data-fragment-id="{fragment_id}"]`);
                                var tpl = document.getElementById("{fragment_id}");
                                console.log("replace", frag, "with", tpl.content.cloneNode(true));
                                frag.replaceWith(tpl.content.cloneNode(true));
                            </script>
                            "#
                    )
                })
            );

            // HTML for the view function and script to store resources
            futures::stream::once(async move {
                format!(
                    r#"
                        {shell}
                        <script>
                            __LEPTOS_PENDING_RESOURCES = {pending_resources};
                            __LEPTOS_RESOLVED_RESOURCES = new Map();
                            __LEPTOS_RESOURCE_RESOLVERS = new Map();
                        </script>
                    "#
                )
            })
            .chain(resources_and_fragments)
            // dispose of Scope and Runtime
            .chain(futures::stream::once(async move {
                disposer.dispose();
                runtime.dispose();
                Default::default()
            }))
        }
    }
}
