use cfg_if::cfg_if;

cfg_if! {
if #[cfg(feature = "ssr")] {
    use crate::{
        error_template::ErrorTemplate,
        errors::TodoAppError,
    };
    use http::Uri;
    use leptos::{view, Errors, LeptosOptions};
    use std::sync::Arc;
    use viz::{
        handlers::serve, header::HeaderMap, types::RouteInfo, Body, Error, Handler,
        Request, RequestExt, Response, ResponseExt, Result,
    };

    pub async fn file_and_error_handler(req: Request<Body>) -> Result<Response> {
        let uri = req.uri().clone();
        let headers = req.headers().clone();
        let route_info = req.route_info().clone();
        let options = &*req.state::<Arc<LeptosOptions>>().ok_or(
            Error::Responder(Response::text("missing state type LeptosOptions")),
        )?;
        let root = &options.site_root;
        let resp = get_static_file(uri, root, headers, route_info).await?;
        let status = resp.status();

        if status.is_success() || status.is_redirection() {
            Ok(resp)
        } else {
            let mut errors = Errors::default();
            errors.insert_with_default_key(TodoAppError::NotFound);
            let handler = leptos_viz::render_app_to_stream(
                options.to_owned(),
                move |cx| view! {cx, <ErrorTemplate outside_errors=errors.clone()/>},
            );
            handler(req).await
        }
    }

    async fn get_static_file(
        uri: Uri,
        root: &str,
        headers: HeaderMap,
        route_info: Arc<RouteInfo>,
    ) -> Result<Response> {
        let mut req = Request::builder()
            .uri(uri.clone())
            .extension(route_info)
            .body(Body::empty())
            .unwrap();
        *req.headers_mut() = headers;
        // This path is relative to the cargo root
        serve::Dir::new(root).call(req).await
    }

}
}
