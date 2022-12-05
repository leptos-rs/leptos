use crate::{use_navigate, use_resolved_path, TextProp};
use cfg_if::cfg_if;
use leptos::*;
use std::{error::Error, rc::Rc};
use typed_builder::TypedBuilder;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Properties that can be passed to the [Form] component, which is an HTML
/// [`form`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
/// progressively enhanced to use client-side routing.
#[derive(TypedBuilder)]
pub struct FormProps<A>
where
    A: TextProp + 'static,
{
    /// [`method`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form#attr-method)
    /// is the HTTP method to submit the form with (`get` or `post`).
    #[builder(default, setter(strip_option))]
    pub method: Option<&'static str>,
    /// [`action`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form#attr-action)
    /// is the URL that processes the form submission. Takes a [String], [&str], or a reactive
    /// function that returns a [String].
    pub action: A,
    /// [`enctype`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form#attr-enctype)
    /// is the MIME type of the form submission if `method` is `post`.
    #[builder(default, setter(strip_option))]
    pub enctype: Option<String>,
    /// A signal that will be incremented whenever the form is submitted with `post`. This can useful
    /// for reactively updating a [Resource] or another signal whenever the form has been submitted.
    #[builder(default, setter(strip_option))]
    pub version: Option<RwSignal<usize>>,
    /// A signal that will be set if the form submission ends in an error.
    #[builder(default, setter(strip_option))]
    pub error: Option<RwSignal<Option<Box<dyn Error>>>>,
    /// A callback will be called with the [FormData](web_sys::FormData) when the form is submitted.
    #[builder(default, setter(strip_option))]
    pub on_form_data: Option<Rc<dyn Fn(&web_sys::FormData)>>,
    /// A callback will be called with the [Response](web_sys::Response) the server sends in response
    /// to a form submission.
    #[builder(default, setter(strip_option))]
    pub on_response: Option<Rc<dyn Fn(&web_sys::Response)>>,
    /// Component children; should include the HTML of the form elements.
    pub children: Box<dyn Fn() -> Vec<Element>>,
}

/// An HTML [`form`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form) progressively
/// enhanced to use client-side routing.
#[allow(non_snake_case)]
pub fn Form<A>(cx: Scope, props: FormProps<A>) -> Element
where
    A: TextProp + 'static,
{
    let FormProps {
        method,
        action,
        enctype,
        children,
        version,
        error,
        on_form_data,
        on_response,
    } = props;

    let action_version = version;
    let action = use_resolved_path(cx, move || action.to_value()());

    let on_submit = move |ev: web_sys::SubmitEvent| {
        if ev.default_prevented() {
            return;
        }
        let navigate = use_navigate(cx);

        let (form, method, action, enctype) = extract_form_attributes(&ev);

        let form_data = web_sys::FormData::new_with_form(&form).unwrap_throw();
        if let Some(on_form_data) = on_form_data.clone() {
            on_form_data(&form_data);
        }
        let params =
            web_sys::UrlSearchParams::new_with_str_sequence_sequence(&form_data).unwrap_throw();
        let action = use_resolved_path(cx, move || action.clone())
            .get()
            .unwrap_or_default();
        // POST
        if method == "post" {
            ev.prevent_default();

            let on_response = on_response.clone();
            spawn_local(async move {
                let res = gloo_net::http::Request::post(&action)
                    .header("Accept", "application/json")
                    .header("Content-Type", &enctype)
                    .body(params)
                    .send()
                    .await;
                match res {
                    Err(e) => {
                        log::error!("<Form/> error while POSTing: {e:#?}");
                        if let Some(error) = error {
                            error.set(Some(Box::new(e)));
                        }
                    }
                    Ok(resp) => {
                        if let Some(version) = action_version {
                            version.update(|n| *n += 1);
                        }
                        if let Some(error) = error {
                            error.set(None);
                        }
                        if let Some(on_response) = on_response.clone() {
                            on_response(&resp.as_raw());
                        }

                        if resp.status() == 303 {
                            if let Some(redirect_url) = resp.headers().get("Location") {
                                _ = navigate(&redirect_url, Default::default());
                            }
                        }
                    }
                }
            });
        }
        // otherwise, GET
        else {
            let params = params.to_string().as_string().unwrap_or_default();
            if navigate(&format!("{action}?{params}"), Default::default()).is_ok() {
                ev.prevent_default();
            }
        }
    };

    let children = children();

    cfg_if! {
        if #[cfg(feature = "stable")] {
            let on_submit = move |ev: web_sys::Event| on_submit(ev.unchecked_into());
        }
    };

    cfg_if! {
        if #[cfg(not(feature = "stable"))] {
            view! { cx,
                <form
                    method=method
                    action=action
                    enctype=enctype
                    on:submit=on_submit
                >
                    {children}
                </form>
            }
        }
        else {
            view! { cx,
                <form
                    method=method
                    action=move || action.get()
                    enctype=enctype
                    on:submit=on_submit
                >
                    {children}
                </form>
            }
        }
    }
}

/// Properties that can be passed to the [ActionForm] component, which
/// automatically turns a server [Action](leptos_server::Action) into an HTML
/// [`form`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
/// progressively enhanced to use client-side routing.
#[derive(TypedBuilder)]
pub struct ActionFormProps<I, O>
where
    I: 'static,
    O: 'static,
{
    /// The action from which to build the form. This should include a URL, which can be generated
    /// by default using [create_server_action](leptos_server::create_server_action) or added
    /// manually using [leptos_server::Action::using_server_fn].
    pub action: Action<I, Result<O, ServerFnError>>,
    /// Component children; should include the HTML of the form elements.
    pub children: Box<dyn Fn() -> Vec<Element>>,
}

/// Automatically turns a server [Action](leptos_server::Action) into an HTML
/// [`form`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
/// progressively enhanced to use client-side routing.
#[allow(non_snake_case)]
pub fn ActionForm<I, O>(cx: Scope, props: ActionFormProps<I, O>) -> Element
where
    I: Clone + ServerFn + 'static,
    O: Clone + Serializable + 'static,
{
    let action = if let Some(url) = props.action.url() {
        url
    } else {
        debug_warn!("<ActionForm/> action needs a URL. Either use create_server_action() or Action::using_server_fn().");
        ""
    }.to_string();
    let version = props.action.version;
    let value = props.action.value;
    let input = props.action.input;

    let on_form_data = Rc::new(move |form_data: &web_sys::FormData| {
        let data = action_input_from_form_data(form_data);
        match data {
            Ok(data) => input.set(Some(data)),
            Err(e) => log::error!("{e}"),
        }
    });

    let on_response = Rc::new(move |resp: &web_sys::Response| {
        let resp = resp.clone().expect("couldn't get Response");
        spawn_local(async move {
            let body =
                JsFuture::from(resp.text().expect("couldn't get .text() from Response")).await;
            match body {
                Ok(json) => {
                    log::debug!(
                        "body is {:?}\nO is {:?}",
                        json.as_string().unwrap(),
                        std::any::type_name::<O>()
                    );
                    match O::from_json(
                        &json.as_string().expect("couldn't get String from JsString"),
                    ) {
                        Ok(res) => value.set(Some(Ok(res))),
                        Err(e) => {
                            value.set(Some(Err(ServerFnError::Deserialization(e.to_string()))))
                        }
                    }
                }
                Err(e) => log::error!("{e:?}"),
            }
        });
    });

    Form(
        cx,
        FormProps::builder()
            .action(action)
            .version(version)
            .on_form_data(on_form_data)
            .on_response(on_response)
            .method("post")
            .children(props.children)
            .build(),
    )
}

/// Properties that can be passed to the [MultiActionForm] component, which
/// automatically turns a server [MultiAction](leptos_server::MultiAction) into an HTML
/// [`form`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
/// progressively enhanced to use client-side routing.
#[derive(TypedBuilder)]
pub struct MultiActionFormProps<I, O>
where
    I: 'static,
    O: 'static,
{
    /// The action from which to build the form. This should include a URL, which can be generated
    /// by default using [create_server_action](leptos_server::create_server_action) or added
    /// manually using [leptos_server::Action::using_server_fn].
    pub action: MultiAction<I, Result<O, ServerFnError>>,
    /// Component children; should include the HTML of the form elements.
    pub children: Box<dyn Fn() -> Vec<Element>>,
}

/// Automatically turns a server [MultiAction](leptos_server::MultiAction) into an HTML
/// [`form`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
/// progressively enhanced to use client-side routing.
#[allow(non_snake_case)]
pub fn MultiActionForm<I, O>(cx: Scope, props: MultiActionFormProps<I, O>) -> Element
where
    I: Clone + ServerFn + 'static,
    O: Clone + Serializable + 'static,
{
    let multi_action = props.action;
    let action = if let Some(url) = multi_action.url() {
        url
    } else {
        debug_warn!("<MultiActionForm/> action needs a URL. Either use create_server_action() or Action::using_server_fn().");
        ""
    }.to_string();

    let on_submit = move |ev: web_sys::SubmitEvent| {
        if ev.default_prevented() {
            return;
        }

        let (form, _, _, _) = extract_form_attributes(&ev);

        let form_data = web_sys::FormData::new_with_form(&form).unwrap_throw();
        let data = action_input_from_form_data(&form_data);
        match data {
            Err(e) => log::error!("{e}"),
            Ok(input) => {
                ev.prevent_default();
                multi_action.dispatch(input);
            }
        }
    };

    let children = (props.children)();

    cfg_if! {
        if #[cfg(feature = "stable")] {
            let on_submit = move |ev: web_sys::Event| on_submit(ev.unchecked_into());
        }
    };

    view! { cx,
        <form
            method="POST"
            action=action
            on:submit=on_submit
        >
            {children}
        </form>
    }
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
                        .unwrap_or_else(|| "".to_string())
                        .to_lowercase(),
                    form.get_attribute("enctype")
                        .unwrap_or_else(|| "application/x-www-form-urlencoded".to_string())
                        .to_lowercase(),
                )
            } else if let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>() {
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
                            .unwrap_or_else(|| "".to_string())
                            .to_lowercase()
                    }),
                    input.get_attribute("enctype").unwrap_or_else(|| {
                        form.get_attribute("enctype")
                            .unwrap_or_else(|| "application/x-www-form-urlencoded".to_string())
                            .to_lowercase()
                    }),
                )
            } else if let Some(button) = el.dyn_ref::<web_sys::HtmlButtonElement>() {
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
                            .unwrap_or_else(|| "".to_string())
                            .to_lowercase()
                    }),
                    button.get_attribute("enctype").unwrap_or_else(|| {
                        form.get_attribute("enctype")
                            .unwrap_or_else(|| "application/x-www-form-urlencoded".to_string())
                            .to_lowercase()
                    }),
                )
            } else {
                leptos_dom::debug_warn!("<Form/> cannot be submitted from a tag other than <form>, <input>, or <button>");
                panic!()
            }
        }
        None => match ev.target() {
            None => {
                leptos_dom::debug_warn!("<Form/> SubmitEvent fired without a target.");
                panic!()
            }
            Some(form) => {
                let form = form.unchecked_into::<web_sys::HtmlFormElement>();
                (
                    form.clone(),
                    form.get_attribute("method")
                        .unwrap_or_else(|| "get".to_string()),
                    form.get_attribute("action").unwrap_or_default(),
                    form.get_attribute("enctype")
                        .unwrap_or_else(|| "application/x-www-form-urlencoded".to_string()),
                )
            }
        },
    }
}

fn action_input_from_form_data<I: serde::de::DeserializeOwned>(
    form_data: &web_sys::FormData,
) -> Result<I, serde_urlencoded::de::Error> {
    let data = web_sys::UrlSearchParams::new_with_str_sequence_sequence(&form_data).unwrap_throw();
    let data = data.to_string().as_string().unwrap_or_default();
    serde_urlencoded::from_str::<I>(&data)
}
