use crate::{
    components::ToHref,
    hooks::{has_router, use_navigate, use_resolved_path},
    location::{BrowserUrl, LocationProvider},
    NavigateOptions,
};
use leptos::{ev, html::form, prelude::*, task::spawn_local};
use std::{error::Error, sync::Arc};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{FormData, RequestRedirect, Response};

type OnFormData = Arc<dyn Fn(&FormData)>;
type OnResponse = Arc<dyn Fn(&Response)>;
type OnError = Arc<dyn Fn(&gloo_net::Error)>;

/// An HTML [`form`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form) progressively
/// enhanced to use client-side routing.
#[component]
pub fn Form<A>(
    /// [`method`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form#attr-method)
    /// is the HTTP method to submit the form with (`get` or `post`).
    #[prop(optional)]
    method: Option<&'static str>,
    /// [`action`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form#attr-action)
    /// is the URL that processes the form submission. Takes a [`String`], [`&str`], or a reactive
    /// function that returns a [`String`].
    action: A,
    /// [`enctype`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form#attr-enctype)
    /// is the MIME type of the form submission if `method` is `post`.
    #[prop(optional)]
    enctype: Option<String>,
    /// A signal that will be incremented whenever the form is submitted with `post`. This can useful
    /// for reactively updating a [Resource] or another signal whenever the form has been submitted.
    #[prop(optional)]
    version: Option<RwSignal<usize>>,
    /// A signal that will be set if the form submission ends in an error.
    #[prop(optional)]
    error: Option<RwSignal<Option<Box<dyn Error + Send + Sync>>>>,
    /// A callback will be called with the [`FormData`](web_sys::FormData) when the form is submitted.
    #[prop(optional)]
    on_form_data: Option<OnFormData>,
    /// A callback will be called with the [`Response`](web_sys::Response) the server sends in response
    /// to a form submission.
    #[prop(optional)]
    on_response: Option<OnResponse>,
    /// A callback will be called if the attempt to submit the form results in an error.
    #[prop(optional)]
    on_error: Option<OnError>,
    /// Sets whether the page should be scrolled to the top when the form is submitted.
    #[prop(optional)]
    noscroll: bool,
    /// Sets whether the page should replace the current location in the history when the form is submitted.
    #[prop(optional)]
    replace: bool,
    /// Component children; should include the HTML of the form elements.
    children: Children,
) -> impl IntoView
where
    A: ToHref + Send + Sync + 'static,
{
    async fn post_form_data(
        action: &str,
        form_data: FormData,
    ) -> Result<gloo_net::http::Response, gloo_net::Error> {
        gloo_net::http::Request::post(action)
            .header("Accept", "application/json")
            .redirect(RequestRedirect::Follow)
            .body(form_data)?
            .send()
            .await
    }

    async fn post_params(
        action: &str,
        enctype: &str,
        params: web_sys::UrlSearchParams,
    ) -> Result<gloo_net::http::Response, gloo_net::Error> {
        gloo_net::http::Request::post(action)
            .header("Accept", "application/json")
            .header("Content-Type", enctype)
            .redirect(RequestRedirect::Follow)
            .body(params)?
            .send()
            .await
    }

    fn inner(
        has_router: bool,
        method: Option<&'static str>,
        action: ArcMemo<Option<String>>,
        enctype: Option<String>,
        version: Option<RwSignal<usize>>,
        error: Option<RwSignal<Option<Box<dyn Error + Send + Sync>>>>,
        on_form_data: Option<OnFormData>,
        on_response: Option<OnResponse>,
        on_error: Option<OnError>,
        children: Children,
        noscroll: bool,
        replace: bool,
    ) -> impl IntoView {
        let action_version = version;
        let navigate = has_router.then(use_navigate);
        let on_submit = {
            move |ev: web_sys::SubmitEvent| {
                let navigate = navigate.clone();
                if ev.default_prevented() {
                    return;
                }
                let navigate_options = NavigateOptions {
                    scroll: !noscroll,
                    replace,
                    ..Default::default()
                };

                let (form, method, action, enctype) =
                    extract_form_attributes(&ev);

                let form_data =
                    web_sys::FormData::new_with_form(&form).unwrap_throw();
                if let Some(on_form_data) = on_form_data.clone() {
                    on_form_data(&form_data);
                }
                let params =
                    web_sys::UrlSearchParams::new_with_str_sequence_sequence(
                        &form_data,
                    )
                    .unwrap_throw();
                // multipart POST (setting Context-Type breaks the request)
                if method == "post" && enctype == "multipart/form-data" {
                    ev.prevent_default();
                    ev.stop_propagation();

                    let on_response = on_response.clone();
                    let on_error = on_error.clone();
                    spawn_local(async move {
                        let res = post_form_data(&action, form_data).await;
                        match res {
                            Err(e) => {
                                error!("<Form/> error while POSTing: {e:#?}");
                                if let Some(on_error) = on_error {
                                    on_error(&e);
                                }
                                if let Some(error) = error {
                                    error.try_set(Some(Box::new(e)));
                                }
                            }
                            Ok(resp) => {
                                let resp = web_sys::Response::from(resp);
                                if let Some(version) = action_version {
                                    version.update(|n| *n += 1);
                                }
                                if let Some(error) = error {
                                    error.try_set(None);
                                }
                                if let Some(on_response) = on_response.clone() {
                                    on_response(&resp);
                                }
                                // Check all the logical 3xx responses that might
                                // get returned from a server function
                                if resp.redirected() {
                                    let resp_url = &resp.url();
                                    match BrowserUrl::parse(resp_url.as_str()) {
                                        Ok(url) => {
                                            if url.origin()
                                                != current_window_origin()
                                                || navigate.is_none()
                                            {
                                                _ = window()
                                                    .location()
                                                    .set_href(
                                                        resp_url.as_str(),
                                                    );
                                            } else {
                                                #[allow(
                                                    clippy::unnecessary_unwrap
                                                )]
                                                let navigate =
                                                    navigate.unwrap();
                                                navigate(
                                                    &format!(
                                                        "{}{}{}",
                                                        url.path(),
                                                        if url
                                                            .search()
                                                            .is_empty()
                                                        {
                                                            ""
                                                        } else {
                                                            "?"
                                                        },
                                                        url.search(),
                                                    ),
                                                    navigate_options,
                                                )
                                            }
                                        }
                                        Err(e) => warn!("{:?}", e),
                                    }
                                }
                            }
                        }
                    });
                }
                // POST
                else if method == "post" {
                    ev.prevent_default();
                    ev.stop_propagation();

                    let on_response = on_response.clone();
                    let on_error = on_error.clone();
                    spawn_local(async move {
                        let res = post_params(&action, &enctype, params).await;
                        match res {
                            Err(e) => {
                                error!("<Form/> error while POSTing: {e:#?}");
                                if let Some(on_error) = on_error {
                                    on_error(&e);
                                }
                                if let Some(error) = error {
                                    error.try_set(Some(Box::new(e)));
                                }
                            }
                            Ok(resp) => {
                                let resp = web_sys::Response::from(resp);
                                if let Some(version) = action_version {
                                    version.update(|n| *n += 1);
                                }
                                if let Some(error) = error {
                                    error.try_set(None);
                                }
                                if let Some(on_response) = on_response.clone() {
                                    on_response(&resp);
                                }
                                // Check all the logical 3xx responses that might
                                // get returned from a server function
                                if resp.redirected() {
                                    let resp_url = &resp.url();
                                    match BrowserUrl::parse(resp_url.as_str()) {
                                        Ok(url) => {
                                            if url.origin()
                                                != current_window_origin()
                                                || navigate.is_none()
                                            {
                                                _ = window()
                                                    .location()
                                                    .set_href(
                                                        resp_url.as_str(),
                                                    );
                                            } else {
                                                #[allow(
                                                    clippy::unnecessary_unwrap
                                                )]
                                                let navigate =
                                                    navigate.unwrap();
                                                navigate(
                                                    &format!(
                                                        "{}{}{}",
                                                        url.path(),
                                                        if url
                                                            .search()
                                                            .is_empty()
                                                        {
                                                            ""
                                                        } else {
                                                            "?"
                                                        },
                                                        url.search(),
                                                    ),
                                                    navigate_options,
                                                )
                                            }
                                        }
                                        Err(e) => warn!("{:?}", e),
                                    }
                                }
                            }
                        }
                    });
                }
                // otherwise, GET
                else {
                    let params =
                        params.to_string().as_string().unwrap_or_default();
                    if let Some(navigate) = navigate {
                        navigate(
                            &format!("{action}?{params}"),
                            navigate_options,
                        );
                    } else {
                        _ = window()
                            .location()
                            .set_href(&format!("{action}?{params}"));
                    }
                    ev.prevent_default();
                    ev.stop_propagation();
                }
            }
        };

        let method = method.unwrap_or("get");

        form()
            .attr("method", method)
            .attr("action", move || action.get())
            .attr("enctype", enctype)
            .on(ev::submit, on_submit)
            .child(children())
    }

    let has_router = has_router();
    let action = if has_router {
        use_resolved_path(move || action.to_href()())
    } else {
        ArcMemo::new(move |_| Some(action.to_href()()))
    };
    inner(
        has_router,
        method,
        action,
        enctype,
        version,
        error,
        on_form_data,
        on_response,
        on_error,
        children,
        noscroll,
        replace,
    )
}

fn current_window_origin() -> String {
    let location = window().location();
    let protocol = location.protocol().unwrap_or_default();
    let hostname = location.hostname().unwrap_or_default();
    let port = location.port().unwrap_or_default();
    format!(
        "{}//{}{}{}",
        protocol,
        hostname,
        if port.is_empty() { "" } else { ":" },
        port
    )
}

fn extract_form_attributes(
    ev: &web_sys::Event,
) -> (web_sys::HtmlFormElement, String, String, String) {
    let submitter = ev.unchecked_ref::<web_sys::SubmitEvent>().submitter();
    match &submitter {
        Some(el) => {
            if let Some(form) = el.dyn_ref::<web_sys::HtmlFormElement>() {
                (
                    form.clone(),
                    form.get_attribute("method")
                        .unwrap_or_else(|| "get".to_string())
                        .to_lowercase(),
                    form.get_attribute("action")
                        .unwrap_or_default()
                        .to_lowercase(),
                    form.get_attribute("enctype")
                        .unwrap_or_else(|| {
                            "application/x-www-form-urlencoded".to_string()
                        })
                        .to_lowercase(),
                )
            } else if let Some(input) =
                el.dyn_ref::<web_sys::HtmlInputElement>()
            {
                let form = ev
                    .target()
                    .unwrap()
                    .unchecked_into::<web_sys::HtmlFormElement>();
                (
                    form.clone(),
                    input.get_attribute("method").unwrap_or_else(|| {
                        form.get_attribute("method")
                            .unwrap_or_else(|| "get".to_string())
                            .to_lowercase()
                    }),
                    input.get_attribute("action").unwrap_or_else(|| {
                        form.get_attribute("action")
                            .unwrap_or_default()
                            .to_lowercase()
                    }),
                    input.get_attribute("enctype").unwrap_or_else(|| {
                        form.get_attribute("enctype")
                            .unwrap_or_else(|| {
                                "application/x-www-form-urlencoded".to_string()
                            })
                            .to_lowercase()
                    }),
                )
            } else if let Some(button) =
                el.dyn_ref::<web_sys::HtmlButtonElement>()
            {
                let form = ev
                    .target()
                    .unwrap()
                    .unchecked_into::<web_sys::HtmlFormElement>();
                (
                    form.clone(),
                    button.get_attribute("method").unwrap_or_else(|| {
                        form.get_attribute("method")
                            .unwrap_or_else(|| "get".to_string())
                            .to_lowercase()
                    }),
                    button.get_attribute("action").unwrap_or_else(|| {
                        form.get_attribute("action")
                            .unwrap_or_default()
                            .to_lowercase()
                    }),
                    button.get_attribute("enctype").unwrap_or_else(|| {
                        form.get_attribute("enctype")
                            .unwrap_or_else(|| {
                                "application/x-www-form-urlencoded".to_string()
                            })
                            .to_lowercase()
                    }),
                )
            } else {
                leptos::logging::debug_warn!(
                    "<Form/> cannot be submitted from a tag other than \
                     <form>, <input>, or <button>"
                );
                panic!()
            }
        }
        None => match ev.target() {
            None => {
                leptos::logging::debug_warn!(
                    "<Form/> SubmitEvent fired without a target."
                );
                panic!()
            }
            Some(form) => {
                let form = form.unchecked_into::<web_sys::HtmlFormElement>();
                (
                    form.clone(),
                    form.get_attribute("method")
                        .unwrap_or_else(|| "get".to_string()),
                    form.get_attribute("action").unwrap_or_default(),
                    form.get_attribute("enctype").unwrap_or_else(|| {
                        "application/x-www-form-urlencoded".to_string()
                    }),
                )
            }
        },
    }
}
