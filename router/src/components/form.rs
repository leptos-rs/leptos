use std::error::Error;

use leptos::*;
use typed_builder::TypedBuilder;
use wasm_bindgen::JsCast;

use crate::{use_navigate, use_resolved_path, ToHref};

/// Properties that can be passed to the [Form] component, which is an HTML
/// [`form`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
/// progressively enhanced to use client-side routing.
#[derive(TypedBuilder)]
pub struct FormProps<A>
where
    A: ToHref + 'static,
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
    /// Component children; should include the HTML of the form elements.
    pub children: Box<dyn Fn() -> Vec<Element>>,
}

/// An HTML [`form`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form) progressively
/// enhanced to use client-side routing.
#[allow(non_snake_case)]
pub fn Form<A>(cx: Scope, props: FormProps<A>) -> Element
where
    A: ToHref + 'static,
{
    let FormProps {
        method,
        action,
        enctype,
        children,
        version,
        error,
    } = props;

    let action_version = version;
    let action = use_resolved_path(cx, move || action.to_href()());

    let on_submit = move |ev: web_sys::Event| {
        if ev.default_prevented() {
            return;
        }
        ev.prevent_default();
        let submitter = ev.unchecked_ref::<web_sys::SubmitEvent>().submitter();
        let navigate = use_navigate(cx);

        let (form, method, action, enctype) = match &submitter {
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
        };

        let form_data = web_sys::FormData::new_with_form(&form).unwrap_throw();
        let params =
            web_sys::UrlSearchParams::new_with_str_sequence_sequence(&form_data).unwrap_throw();
        let action = use_resolved_path(cx, move || action.clone())
            .get()
            .unwrap_or_default();
        // POST
        if method == "post" {
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

                        if resp.status() == 303 {
                            if let Some(redirect_url) = resp.headers().get("Location") {
                                navigate(&redirect_url, Default::default());
                            }
                        }
                    }
                }
            });
        }
        // otherwise, GET
        else {
            let params = params.to_string().as_string().unwrap_or_default();
            navigate(&format!("{action}?{params}"), Default::default());
        }
    };

    let children = children();

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
    pub action: Action<I, O>,
    /// Component children; should include the HTML of the form elements.
    pub children: Box<dyn Fn() -> Vec<Element>>,
}

/// Automatically turns a server [Action](leptos_server::Action) into an HTML
/// [`form`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
/// progressively enhanced to use client-side routing.
#[allow(non_snake_case)]
pub fn ActionForm<I, O>(cx: Scope, props: ActionFormProps<I, O>) -> Element
where
    I: 'static,
    O: 'static,
{
    let action = if let Some(url) = props.action.url() {
        format!("/{url}")
    } else {
        debug_warn!("<ActionForm/> action needs a URL. Either use create_server_action() or Action::using_server_fn().");
        "".to_string()
    };
    let version = props.action.version;

    Form(
        cx,
        FormProps::builder()
            .action(action)
            .version(version)
            .method("post")
            .children(props.children)
            .build(),
    )
}
