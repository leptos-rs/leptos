use crate::{use_navigate, use_resolved_path, ToHref, Url};
use leptos::{html::form, *};
use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error, rc::Rc};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use wasm_bindgen_futures::JsFuture;
use web_sys::RequestRedirect;

type OnFormData = Rc<dyn Fn(&web_sys::FormData)>;
type OnResponse = Rc<dyn Fn(&web_sys::Response)>;
type OnError = Rc<dyn Fn(&gloo_net::Error)>;

/// An HTML [`form`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form) progressively
/// enhanced to use client-side routing.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
#[component]
pub fn Form<A>(
    cx: Scope,
    /// [`method`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form#attr-method)
    /// is the HTTP method to submit the form with (`get` or `post`).
    #[prop(optional)]
    method: Option<&'static str>,
    /// [`action`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form#attr-action)
    /// is the URL that processes the form submission. Takes a [String], [&str], or a reactive
    /// function that returns a [String].
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
    error: Option<RwSignal<Option<Box<dyn Error>>>>,
    /// A callback will be called with the [FormData](web_sys::FormData) when the form is submitted.
    #[prop(optional)]
    on_form_data: Option<OnFormData>,
    /// Sets the `class` attribute on the underlying `<form>` tag, making it easier to style.
    #[prop(optional, into)]
    class: Option<AttributeValue>,
    /// A callback will be called with the [Response](web_sys::Response) the server sends in response
    /// to a form submission.
    #[prop(optional)]
    on_response: Option<OnResponse>,
    /// A callback will be called if the attempt to submit the form results in an error.
    #[prop(optional)]
    on_error: Option<OnError>,
    /// A [`NodeRef`] in which the `<form>` element should be stored.
    #[prop(optional)]
    node_ref: Option<NodeRef<html::Form>>,
    /// Arbitrary attributes to add to the `<form>`
    #[prop(optional, into)]
    attributes: Option<MaybeSignal<AdditionalAttributes>>,
    /// Component children; should include the HTML of the form elements.
    children: Children,
) -> impl IntoView
where
    A: ToHref + 'static,
{
    fn inner(
        cx: Scope,
        method: Option<&'static str>,
        action: Memo<Option<String>>,
        enctype: Option<String>,
        version: Option<RwSignal<usize>>,
        error: Option<RwSignal<Option<Box<dyn Error>>>>,
        on_form_data: Option<OnFormData>,
        on_response: Option<OnResponse>,
        on_error: Option<OnError>,
        class: Option<Attribute>,
        children: Children,
        node_ref: Option<NodeRef<html::Form>>,
        attributes: Option<MaybeSignal<AdditionalAttributes>>,
    ) -> HtmlElement<html::Form> {
        let action_version = version;
        let on_submit = {
            move |ev: web_sys::SubmitEvent| {
                if ev.default_prevented() {
                    return;
                }
                let navigate = use_navigate(cx);

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
                let action = use_resolved_path(cx, move || action.clone())
                    .get_untracked()
                    .unwrap_or_default();
                // multipart POST (setting Context-Type breaks the request)
                if method == "post" && enctype == "multipart/form-data" {
                    ev.prevent_default();
                    ev.stop_propagation();

                    let on_response = on_response.clone();
                    let on_error = on_error.clone();
                    spawn_local(async move {
                        let res = gloo_net::http::Request::post(&action)
                            .header("Accept", "application/json")
                            .redirect(RequestRedirect::Follow)
                            .body(form_data)
                            .send()
                            .await;
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
                                if let Some(version) = action_version {
                                    version.update(|n| *n += 1);
                                }
                                if let Some(error) = error {
                                    error.try_set(None);
                                }
                                if let Some(on_response) = on_response.clone() {
                                    on_response(resp.as_raw());
                                }
                                // Check all the logical 3xx responses that might
                                // get returned from a server function
                                if resp.redirected() {
                                    let resp_url = &resp.url();
                                    match Url::try_from(resp_url.as_str()) {
                                        Ok(url) => {
                                            if url.origin
                                                != current_window_origin()
                                            {
                                                _ = window()
                                                    .location()
                                                    .set_href(
                                                        resp_url.as_str(),
                                                    );
                                            } else {
                                                request_animation_frame(
                                                    move || {
                                                        if let Err(e) = navigate(
                                                            &format!(
                                                                "{}{}{}",
                                                                url.pathname,
                                                                if url
                                                                    .search
                                                                    .is_empty()
                                                                {
                                                                    ""
                                                                } else {
                                                                    "?"
                                                                },
                                                                url.search,
                                                            ),
                                                            Default::default(),
                                                        ) {
                                                            warn!("{}", e);
                                                        }
                                                    },
                                                );
                                            }
                                        }
                                        Err(e) => warn!("{}", e),
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
                        let res = gloo_net::http::Request::post(&action)
                            .header("Accept", "application/json")
                            .header("Content-Type", &enctype)
                            .redirect(RequestRedirect::Follow)
                            .body(params)
                            .send()
                            .await;
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
                                if let Some(version) = action_version {
                                    version.update(|n| *n += 1);
                                }
                                if let Some(error) = error {
                                    error.try_set(None);
                                }
                                if let Some(on_response) = on_response.clone() {
                                    on_response(resp.as_raw());
                                }
                                // Check all the logical 3xx responses that might
                                // get returned from a server function
                                if resp.redirected() {
                                    let resp_url = &resp.url();
                                    match Url::try_from(resp_url.as_str()) {
                                        Ok(url) => {
                                            if url.origin
                                                != current_window_origin()
                                            {
                                                _ = window()
                                                    .location()
                                                    .set_href(
                                                        resp_url.as_str(),
                                                    );
                                            } else {
                                                request_animation_frame(
                                                    move || {
                                                        if let Err(e) = navigate(
                                                            &format!(
                                                                "{}{}{}",
                                                                url.pathname,
                                                                if url
                                                                    .search
                                                                    .is_empty()
                                                                {
                                                                    ""
                                                                } else {
                                                                    "?"
                                                                },
                                                                url.search,
                                                            ),
                                                            Default::default(),
                                                        ) {
                                                            warn!("{}", e);
                                                        }
                                                    },
                                                );
                                            }
                                        }
                                        Err(e) => warn!("{}", e),
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
                    if navigate(
                        &format!("{action}?{params}"),
                        Default::default(),
                    )
                    .is_ok()
                    {
                        ev.prevent_default();
                        ev.stop_propagation();
                    }
                }
            }
        };

        let method = method.unwrap_or("get");

        let mut form = form(cx)
            .attr("method", method)
            .attr("action", move || action.get())
            .attr("enctype", enctype)
            .on(ev::submit, on_submit)
            .attr("class", class)
            .child(children(cx));
        if let Some(node_ref) = node_ref {
            form = form.node_ref(node_ref)
        };
        if let Some(attributes) = attributes {
            let attributes = attributes.get();
            for (attr_name, attr_value) in attributes.into_iter() {
                let attr_name = attr_name.to_owned();
                let attr_value = attr_value.to_owned();
                form = form.attr(attr_name, move || attr_value.get());
            }
        }
        form
    }

    let action = use_resolved_path(cx, move || action.to_href()());
    let class = class.map(|bx| bx.into_attribute_boxed(cx));
    inner(
        cx,
        method,
        action,
        enctype,
        version,
        error,
        on_form_data,
        on_response,
        on_error,
        class,
        children,
        node_ref,
        attributes,
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

/// Automatically turns a server [Action](leptos_server::Action) into an HTML
/// [`form`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
/// progressively enhanced to use client-side routing.
///
/// ## Encoding
/// **Note:** `<ActionForm/>` only works with server functions that use the
/// default `Url` encoding. This is to ensure that `<ActionForm/>` works correctly
/// both before and after WASM has loaded.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
#[component]
pub fn ActionForm<I, O>(
    cx: Scope,
    /// The action from which to build the form. This should include a URL, which can be generated
    /// by default using [create_server_action](leptos_server::create_server_action) or added
    /// manually using [leptos_server::Action::using_server_fn].
    action: Action<I, Result<O, ServerFnError>>,
    /// Sets the `class` attribute on the underlying `<form>` tag, making it easier to style.
    #[prop(optional, into)]
    class: Option<AttributeValue>,
    /// A signal that will be set if the form submission ends in an error.
    #[prop(optional)]
    error: Option<RwSignal<Option<Box<dyn Error>>>>,
    /// A [`NodeRef`] in which the `<form>` element should be stored.
    #[prop(optional)]
    node_ref: Option<NodeRef<html::Form>>,
    /// Arbitrary attributes to add to the `<form>`
    #[prop(optional, into)]
    attributes: Option<MaybeSignal<AdditionalAttributes>>,
    /// Component children; should include the HTML of the form elements.
    children: Children,
) -> impl IntoView
where
    I: Clone + ServerFn + 'static,
    O: Clone + Serialize + DeserializeOwned + 'static,
{
    let action_url = if let Some(url) = action.url() {
        url
    } else {
        debug_warn!(
            "<ActionForm/> action needs a URL. Either use \
             create_server_action() or Action::using_server_fn()."
        );
        String::new()
    };
    let version = action.version();
    let value = action.value();
    let input = action.input();

    let on_error = Rc::new(move |e: &gloo_net::Error| {
        cx.batch(move || {
            action.set_pending(false);
            let e = ServerFnError::Request(e.to_string());
            value.try_set(Some(Err(e.clone())));
            if let Some(error) = error {
                error.try_set(Some(Box::new(ServerFnErrorErr::from(e))));
            }
        });
    });

    let on_form_data = Rc::new(move |form_data: &web_sys::FormData| {
        let data = I::from_form_data(form_data);
        match data {
            Ok(data) => {
                cx.batch(move || {
                    input.try_set(Some(data));
                    action.set_pending(true);
                });
            }
            Err(e) => {
                error!("{e}");
                let e = ServerFnError::Serialization(e.to_string());
                cx.batch(move || {
                    value.try_set(Some(Err(e.clone())));
                    if let Some(error) = error {
                        error
                            .try_set(Some(Box::new(ServerFnErrorErr::from(e))));
                    }
                });
            }
        }
    });

    let on_response = Rc::new(move |resp: &web_sys::Response| {
        let resp = resp.clone().expect("couldn't get Response");
        spawn_local(async move {
            let body = JsFuture::from(
                resp.text().expect("couldn't get .text() from Response"),
            )
            .await;
            let status = resp.status();
            match body {
                Ok(json) => {
                    let json = json
                        .as_string()
                        .expect("couldn't get String from JsString");
                    if (500..=599).contains(&status) {
                        match serde_json::from_str::<ServerFnError>(&json) {
                            Ok(res) => {
                                value.try_set(Some(Err(res)));
                                if let Some(error) = error {
                                    error.try_set(None);
                                }
                            }
                            Err(e) => {
                                value.try_set(Some(Err(
                                    ServerFnError::Deserialization(
                                        e.to_string(),
                                    ),
                                )));
                                if let Some(error) = error {
                                    error.try_set(Some(Box::new(e)));
                                }
                            }
                        }
                    } else {
                        match serde_json::from_str::<O>(&json) {
                            Ok(res) => {
                                value.try_set(Some(Ok(res)));
                                if let Some(error) = error {
                                    error.try_set(None);
                                }
                            }
                            Err(e) => {
                                value.try_set(Some(Err(
                                    ServerFnError::Deserialization(
                                        e.to_string(),
                                    ),
                                )));
                                if let Some(error) = error {
                                    error.try_set(Some(Box::new(e)));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("{e:?}");
                    if let Some(error) = error {
                        error.try_set(Some(Box::new(
                            ServerFnErrorErr::Request(
                                e.as_string().unwrap_or_default(),
                            ),
                        )));
                    }
                }
            };
            cx.batch(move || {
                input.try_set(None);
                action.set_pending(false);
            });
        });
    });
    let class = class.map(|bx| bx.into_attribute_boxed(cx));

    #[cfg(debug_assertions)]
    {
        if I::encoding() != server_fn::Encoding::Url {
            leptos::warn!(
                "<ActionForm/> only supports the `Url` encoding for server \
                 functions, but {} uses {:?}.",
                std::any::type_name::<I>(),
                I::encoding()
            );
        }
    }

    let mut props = FormProps::builder()
        .action(action_url)
        .version(version)
        .on_form_data(on_form_data)
        .on_response(on_response)
        .on_error(on_error)
        .method("post")
        .class(class)
        .children(children)
        .build();
    props.error = error;
    props.node_ref = node_ref;
    props.attributes = attributes;
    Form(cx, props)
}

/// Automatically turns a server [MultiAction](leptos_server::MultiAction) into an HTML
/// [`form`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
/// progressively enhanced to use client-side routing.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
#[component]
pub fn MultiActionForm<I, O>(
    cx: Scope,
    /// The action from which to build the form. This should include a URL, which can be generated
    /// by default using [create_server_action](leptos_server::create_server_action) or added
    /// manually using [leptos_server::Action::using_server_fn].
    action: MultiAction<I, Result<O, ServerFnError>>,
    /// Sets the `class` attribute on the underlying `<form>` tag, making it easier to style.
    #[prop(optional, into)]
    class: Option<AttributeValue>,
    /// A signal that will be set if the form submission ends in an error.
    #[prop(optional)]
    error: Option<RwSignal<Option<Box<dyn Error>>>>,
    /// A [`NodeRef`] in which the `<form>` element should be stored.
    #[prop(optional)]
    node_ref: Option<NodeRef<html::Form>>,
    /// Arbitrary attributes to add to the `<form>`
    #[prop(optional, into)]
    attributes: Option<MaybeSignal<AdditionalAttributes>>,
    /// Component children; should include the HTML of the form elements.
    children: Children,
) -> impl IntoView
where
    I: Clone + ServerFn + 'static,
    O: Clone + Serializable + 'static,
{
    let multi_action = action;
    let action = if let Some(url) = multi_action.url() {
        url
    } else {
        debug_warn!(
            "<MultiActionForm/> action needs a URL. Either use \
             create_server_action() or Action::using_server_fn()."
        );
        String::new()
    };

    let on_submit = move |ev: web_sys::SubmitEvent| {
        if ev.default_prevented() {
            return;
        }

        match I::from_event(&ev) {
            Err(e) => {
                error!("{e}");
                if let Some(error) = error {
                    error.try_set(Some(Box::new(e)));
                }
            }
            Ok(input) => {
                ev.prevent_default();
                ev.stop_propagation();
                multi_action.dispatch(input);
                if let Some(error) = error {
                    error.try_set(None);
                }
            }
        }
    };

    let class = class.map(|bx| bx.into_attribute_boxed(cx));
    let mut form = form(cx)
        .attr("method", "POST")
        .attr("action", action)
        .on(ev::submit, on_submit)
        .attr("class", class)
        .child(children(cx));
    if let Some(node_ref) = node_ref {
        form = form.node_ref(node_ref)
    };
    if let Some(attributes) = attributes {
        let attributes = attributes.get();
        for (attr_name, attr_value) in attributes.into_iter() {
            let attr_name = attr_name.to_owned();
            let attr_value = attr_value.to_owned();
            form = form.attr(attr_name, move || attr_value.get());
        }
    }
    form
}
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
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
                leptos_dom::debug_warn!(
                    "<Form/> cannot be submitted from a tag other than \
                     <form>, <input>, or <button>"
                );
                panic!()
            }
        }
        None => match ev.target() {
            None => {
                leptos_dom::debug_warn!(
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

/// Tries to deserialize a type from form data. This can be used for client-side
/// validation during form submission.
pub trait FromFormData
where
    Self: Sized + serde::de::DeserializeOwned,
{
    /// Tries to deserialize the data, given only the `submit` event.
    fn from_event(ev: &web_sys::Event) -> Result<Self, serde_qs::Error>;

    /// Tries to deserialize the data, given the actual form data.
    fn from_form_data(
        form_data: &web_sys::FormData,
    ) -> Result<Self, serde_qs::Error>;
}

impl<T> FromFormData for T
where
    T: serde::de::DeserializeOwned,
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    fn from_event(ev: &web_sys::Event) -> Result<Self, serde_qs::Error> {
        let (form, _, _, _) = extract_form_attributes(ev);

        let form_data = web_sys::FormData::new_with_form(&form).unwrap_throw();

        Self::from_form_data(&form_data)
    }
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    fn from_form_data(
        form_data: &web_sys::FormData,
    ) -> Result<Self, serde_qs::Error> {
        let data =
            web_sys::UrlSearchParams::new_with_str_sequence_sequence(form_data)
                .unwrap_throw();
        let data = data.to_string().as_string().unwrap_or_default();
        serde_qs::Config::new(5, false).deserialize_str::<Self>(&data)
    }
}
