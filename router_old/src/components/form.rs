use crate::{
    hooks::has_router, resolve_redirect_url, use_navigate, use_resolved_path,
    NavigateOptions, ToHref, Url,
};
use leptos::{
    html::form,
    logging::*,
    server_fn::{client::Client, codec::PostUrl, request::ClientReq, ServerFn},
    *,
};
use serde::de::DeserializeOwned;
use std::{error::Error, fmt::Debug, rc::Rc};
use thiserror::Error;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::{
    Event, FormData, HtmlButtonElement, HtmlFormElement, HtmlInputElement,
    RequestRedirect, SubmitEvent,
};

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
    error: Option<RwSignal<Option<Box<dyn Error>>>>,
    /// A callback will be called with the [`FormData`](web_sys::FormData) when the form is submitted.
    #[prop(optional)]
    on_form_data: Option<OnFormData>,
    /// Sets the `class` attribute on the underlying `<form>` tag, making it easier to style.
    #[prop(optional, into)]
    class: Option<AttributeValue>,
    /// A callback will be called with the [`Response`](web_sys::Response) the server sends in response
    /// to a form submission.
    #[prop(optional)]
    on_response: Option<OnResponse>,
    /// A callback will be called if the attempt to submit the form results in an error.
    #[prop(optional)]
    on_error: Option<OnError>,
    /// A [`NodeRef`] in which the `<form>` element should be stored.
    #[prop(optional)]
    node_ref: Option<NodeRef<html::Form>>,
    /// Sets whether the page should be scrolled to the top when the form is submitted.
    #[prop(optional)]
    noscroll: bool,
    /// Sets whether the page should replace the current location in the history when the form is submitted.
    #[prop(optional)]
    replace: bool,
    /// Arbitrary attributes to add to the `<form>`. Attributes can be added with the
    /// `attr:` syntax in the `view` macro.
    #[prop(attrs)]
    attributes: Vec<(&'static str, Attribute)>,
    /// Component children; should include the HTML of the form elements.
    children: Children,
) -> impl IntoView
where
    A: ToHref + 'static,
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
        noscroll: bool,
        replace: bool,
        attributes: Vec<(&'static str, Attribute)>,
    ) -> HtmlElement<html::Form> {
        let action_version = version;
        let on_submit = {
            move |ev: web_sys::SubmitEvent| {
                if ev.default_prevented() {
                    return;
                }
                let navigate = has_router.then(use_navigate);
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
                let action = if has_router {
                    use_resolved_path(move || action.clone())
                        .get_untracked()
                        .unwrap_or_default()
                } else {
                    action
                };
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
                                    match Url::try_from(resp_url.as_str()) {
                                        Ok(url) => {
                                            if url.origin
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
                                                        url.pathname,
                                                        if url.search.is_empty()
                                                        {
                                                            ""
                                                        } else {
                                                            "?"
                                                        },
                                                        url.search,
                                                    ),
                                                    navigate_options,
                                                )
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
                                    match Url::try_from(resp_url.as_str()) {
                                        Ok(url) => {
                                            if url.origin
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
                                                        url.pathname,
                                                        if url.search.is_empty()
                                                        {
                                                            ""
                                                        } else {
                                                            "?"
                                                        },
                                                        url.search,
                                                    ),
                                                    navigate_options,
                                                )
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

        let mut form = form()
            .attr("method", method)
            .attr("action", move || action.get())
            .attr("enctype", enctype)
            .on(ev::submit, on_submit)
            .attr("class", class)
            .child(children());
        if let Some(node_ref) = node_ref {
            form = form.node_ref(node_ref)
        };
        for (attr_name, attr_value) in attributes {
            form = form.attr(attr_name, attr_value);
        }
        form
    }

    let has_router = has_router();
    let action = if has_router {
        use_resolved_path(move || action.to_href()())
    } else {
        create_memo(move |_| Some(action.to_href()()))
    };
    let class = class.map(|bx| bx.into_attribute_boxed());
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
        class,
        children,
        node_ref,
        noscroll,
        replace,
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
///
/// ## Complex Inputs
/// Server function arguments that are structs with nested serializable fields
/// should make use of indexing notation of `serde_qs`.
///
/// ```rust
/// # use leptos::*;
/// # use leptos_router::*;
///
/// #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// struct HeftyData {
///     first_name: String,
///     last_name: String,
/// }
///
/// #[component]
/// fn ComplexInput() -> impl IntoView {
///     let submit = Action::<VeryImportantFn, _>::server();
///
///     view! {
///       <ActionForm action=submit>
///         <input type="text" name="hefty_arg[first_name]" value="leptos"/>
///         <input
///           type="text"
///           name="hefty_arg[last_name]"
///           value="closures-everywhere"
///         />
///         <input type="submit"/>
///       </ActionForm>
///     }
/// }
///
/// #[server]
/// async fn very_important_fn(
///     hefty_arg: HeftyData,
/// ) -> Result<(), ServerFnError> {
///     assert_eq!(hefty_arg.first_name.as_str(), "leptos");
///     assert_eq!(hefty_arg.last_name.as_str(), "closures-everywhere");
///     Ok(())
/// }
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
#[component]
pub fn ActionForm<ServFn>(
    /// The action from which to build the form. This should include a URL, which can be generated
    /// by default using [`create_server_action`](leptos_server::create_server_action) or added
    /// manually using [`using_server_fn`](leptos_server::Action::using_server_fn).
    action: Action<
        ServFn,
        Result<ServFn::Output, ServerFnError<ServFn::Error>>,
    >,
    /// Sets the `id` attribute on the underlying `<form>` tag
    #[prop(optional, into)]
    id: Option<AttributeValue>,
    /// Sets the `class` attribute on the underlying `<form>` tag, making it easier to style.
    #[prop(optional, into)]
    class: Option<AttributeValue>,

    /// A [`NodeRef`] in which the `<form>` element should be stored.
    #[prop(optional)]
    node_ref: Option<NodeRef<html::Form>>,
    /// Arbitrary attributes to add to the `<form>`
    #[prop(attrs, optional)]
    attributes: Vec<(&'static str, Attribute)>,
    /// Component children; should include the HTML of the form elements.
    children: Children,
) -> impl IntoView
where
    ServFn: DeserializeOwned + ServerFn<InputEncoding = PostUrl> + 'static,
    <<ServFn::Client as Client<ServFn::Error>>::Request as ClientReq<
        ServFn::Error,
    >>::FormData: From<FormData>,
{
    let has_router = has_router();
    if !has_router {
        _ = server_fn::redirect::set_redirect_hook(|loc: &str| {
            if let Some(url) = resolve_redirect_url(loc) {
                _ = window().location().set_href(&url.href());
            }
        });
    }
    let action_url = action.url().unwrap_or_else(|| {
        debug_warn!(
            "<ActionForm/> action needs a URL. Either use \
             create_server_action() or Action::using_server_fn()."
        );
        String::new()
    });
    let version = action.version();
    let value = action.value();

    let class = class.map(|bx| bx.into_attribute_boxed());
    let id = id.map(|bx| bx.into_attribute_boxed());

    let on_submit = {
        move |ev: SubmitEvent| {
            if ev.default_prevented() {
                return;
            }

            // <button formmethod="dialog"> should *not* dispatch the action, but should be allowed to
            // just bubble up and close the <dialog> naturally
            let is_dialog = ev
                .submitter()
                .and_then(|el| el.get_attribute("formmethod"))
                .as_deref()
                == Some("dialog");
            if is_dialog {
                return;
            }

            ev.prevent_default();

            match ServFn::from_event(&ev) {
                Ok(new_input) => {
                    action.dispatch(new_input);
                }
                Err(err) => {
                    error!(
                        "Error converting form field into server function \
                         arguments: {err:?}"
                    );
                    batch(move || {
                        value.set(Some(Err(ServerFnError::Serialization(
                            err.to_string(),
                        ))));
                        version.update(|n| *n += 1);
                    });
                }
            }
        }
    };

    let mut action_form = form()
        .attr("action", action_url)
        .attr("method", "post")
        .attr("id", id)
        .attr("class", class)
        .on(ev::submit, on_submit)
        .child(children());
    if let Some(node_ref) = node_ref {
        action_form = action_form.node_ref(node_ref)
    };
    for (attr_name, attr_value) in attributes {
        action_form = action_form.attr(attr_name, attr_value);
    }
    action_form
}

/// Automatically turns a server [MultiAction](leptos_server::MultiAction) into an HTML
/// [`form`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
/// progressively enhanced to use client-side routing.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
#[component]
pub fn MultiActionForm<ServFn>(
    /// The action from which to build the form. This should include a URL, which can be generated
    /// by default using [create_server_action](leptos_server::create_server_action) or added
    /// manually using [leptos_server::Action::using_server_fn].
    action: MultiAction<ServFn, Result<ServFn::Output, ServerFnError>>,
    /// Sets the `id` attribute on the underlying `<form>` tag
    #[prop(optional, into)]
    id: Option<AttributeValue>,
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
    #[prop(attrs, optional)]
    attributes: Vec<(&'static str, Attribute)>,
    /// Component children; should include the HTML of the form elements.
    children: Children,
) -> impl IntoView
where
    ServFn:
        Clone + DeserializeOwned + ServerFn<InputEncoding = PostUrl> + 'static,
    <<ServFn::Client as Client<ServFn::Error>>::Request as ClientReq<
        ServFn::Error,
    >>::FormData: From<FormData>,
{
    let has_router = has_router();
    if !has_router {
        _ = server_fn::redirect::set_redirect_hook(|loc: &str| {
            if let Some(url) = resolve_redirect_url(loc) {
                _ = window().location().set_href(&url.href());
            }
        });
    }
    let action_url = action.url().unwrap_or_else(|| {
        debug_warn!(
            "<MultiActionForm/> action needs a URL. Either use \
             create_server_action() or Action::using_server_fn()."
        );
        String::new()
    });

    let on_submit = move |ev: SubmitEvent| {
        if ev.default_prevented() {
            return;
        }

        ev.prevent_default();

        match ServFn::from_event(&ev) {
            Err(e) => {
                if let Some(error) = error {
                    error.try_set(Some(Box::new(e)));
                }
            }
            Ok(input) => {
                action.dispatch(input);
                if let Some(error) = error {
                    error.try_set(None);
                }
            }
        }
    };

    let class = class.map(|bx| bx.into_attribute_boxed());

    let id = id.map(|bx| bx.into_attribute_boxed());
    let mut action_form = form()
        .attr("action", action_url)
        .attr("method", "post")
        .attr("id", id)
        .attr("class", class)
        .on(ev::submit, on_submit)
        .child(children());
    if let Some(node_ref) = node_ref {
        action_form = action_form.node_ref(node_ref)
    };
    for (attr_name, attr_value) in attributes {
        action_form = action_form.attr(attr_name, attr_value);
    }
    action_form
}

fn form_data_from_event(
    ev: &SubmitEvent,
) -> Result<FormData, FromFormDataError> {
    let submitter = ev.submitter();
    let mut submitter_name_value = None;
    let opt_form = match &submitter {
        Some(el) => {
            if let Some(form) = el.dyn_ref::<HtmlFormElement>() {
                Some(form.clone())
            } else if let Some(input) = el.dyn_ref::<HtmlInputElement>() {
                submitter_name_value = Some((input.name(), input.value()));
                Some(ev.target().unwrap().unchecked_into())
            } else if let Some(button) = el.dyn_ref::<HtmlButtonElement>() {
                submitter_name_value = Some((button.name(), button.value()));
                Some(ev.target().unwrap().unchecked_into())
            } else {
                None
            }
        }
        None => ev.target().map(|form| form.unchecked_into()),
    };
    match opt_form.as_ref().map(FormData::new_with_form) {
        None => Err(FromFormDataError::MissingForm(ev.clone().into())),
        Some(Err(e)) => Err(FromFormDataError::FormData(e)),
        Some(Ok(form_data)) => {
            if let Some((name, value)) = submitter_name_value {
                form_data
                    .append_with_str(&name, &value)
                    .map_err(FromFormDataError::FormData)?;
            }
            Ok(form_data)
        }
    }
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
    fn from_event(ev: &web_sys::Event) -> Result<Self, FromFormDataError>;

    /// Tries to deserialize the data, given the actual form data.
    fn from_form_data(
        form_data: &web_sys::FormData,
    ) -> Result<Self, serde_qs::Error>;
}

#[derive(Error, Debug)]
pub enum FromFormDataError {
    #[error("Could not find <form> connected to event.")]
    MissingForm(Event),
    #[error("Could not create FormData from <form>: {0:?}")]
    FormData(JsValue),
    #[error("Deserialization error: {0:?}")]
    Deserialization(serde_qs::Error),
}

impl<T> FromFormData for T
where
    T: serde::de::DeserializeOwned,
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    fn from_event(ev: &Event) -> Result<Self, FromFormDataError> {
        let submit_ev = ev.unchecked_ref();
        let form_data = form_data_from_event(submit_ev)?;
        Self::from_form_data(&form_data)
            .map_err(FromFormDataError::Deserialization)
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
