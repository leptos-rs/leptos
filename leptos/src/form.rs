use crate::{IntoView, children::Children, component, prelude::*};
use leptos_dom::helpers::window;
use leptos_server::{ServerAction, ServerMultiAction};
use serde::de::DeserializeOwned;
use server_fn::{
    Http, ServerFn,
    client::Client,
    codec::PostUrl,
    error::{IntoAppError, ServerFnErrorErr},
    request::ClientReq,
};
use tachys::{
    either::Either,
    html::{
        element::{Form, form},
        event::submit,
    },
    reactive_graph::node_ref::NodeRef,
};
use thiserror::Error;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt, prelude::wasm_bindgen};
use web_sys::{Event, FormData, HtmlElement, HtmlFormElement, SubmitEvent};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = FormData, js_name = FormData)]
    type FormDataWithSubmitter;

    #[wasm_bindgen(constructor, catch, js_class = "FormData")]
    fn new_with_form_and_submitter(
        form: &HtmlFormElement,
        submitter: &HtmlElement,
    ) -> Result<FormDataWithSubmitter, JsValue>;
}

/// Builds the [`FormData`] for `form` as a native submission would, including
/// the entry contributed by `submitter` (the button that triggered the
/// submission). `new FormData(form)` alone leaves that entry out, so servers
/// that branch on the button's `name`/`value` cannot tell which was activated.
pub fn form_data_with_submitter(
    form: &HtmlFormElement,
    submitter: Option<&HtmlElement>,
) -> Result<FormData, JsValue> {
    match submitter {
        Some(submitter) => {
            FormDataWithSubmitter::new_with_form_and_submitter(form, submitter)
                .map(|data| data.unchecked_into())
        }
        None => FormData::new_with_form(form),
    }
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
/// # use leptos::prelude::*;
/// use leptos::form::ActionForm;
///
/// #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// struct HeftyData {
///     first_name: String,
///     last_name: String,
/// }
///
/// #[component]
/// fn ComplexInput() -> impl IntoView {
///     let submit = ServerAction::<VeryImportantFn>::new();
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
#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip_all))]
#[component]
pub fn ActionForm<ServFn, OutputProtocol>(
    /// The action from which to build the form.
    action: ServerAction<ServFn>,
    /// A [`NodeRef`] in which the `<form>` element should be stored.
    #[prop(optional)]
    node_ref: Option<NodeRef<Form>>,
    /// Component children; should include the HTML of the form elements.
    children: Children,
) -> impl IntoView
where
    ServFn: DeserializeOwned
        + ServerFn<Protocol = Http<PostUrl, OutputProtocol>>
        + Clone
        + Send
        + Sync
        + 'static,
    <<ServFn::Client as Client<ServFn::Error>>::Request as ClientReq<
        ServFn::Error,
    >>::FormData: From<FormData>,
    ServFn: Send + Sync + 'static,
    ServFn::Output: Send + Sync + 'static,
    ServFn::Error: Send + Sync + 'static,
    <ServFn as ServerFn>::Client: Client<<ServFn as ServerFn>::Error>,
{
    // if redirect hook has not yet been set (by a router), defaults to a browser redirect
    _ = server_fn::redirect::set_redirect_hook(|loc: &str| {
        if let Some(url) = resolve_redirect_url(loc) {
            _ = window().location().set_href(&url.href());
        }
    });

    let version = action.version();
    let value = action.value();

    let on_submit = {
        move |ev: SubmitEvent| {
            if ev.default_prevented() {
                return;
            }

            ev.prevent_default();

            match ServFn::from_event(&ev) {
                Ok(new_input) => {
                    action.dispatch(new_input);
                }
                Err(err) => {
                    crate::logging::error!(
                        "Error converting form field into server function \
                         arguments: {err:?}"
                    );
                    value.set(Some(Err(ServerFnErrorErr::Serialization(
                        err.to_string(),
                    )
                    .into_app_error())));
                    version.update(|n| *n += 1);
                }
            }
        }
    };

    let action_form = form()
        .action(ServFn::url())
        .method("post")
        .on(submit, on_submit)
        .child(children());
    if let Some(node_ref) = node_ref {
        Either::Left(action_form.node_ref(node_ref))
    } else {
        Either::Right(action_form)
    }
}

/// Automatically turns a server [MultiAction](MultiAction) into an HTML
/// [`form`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
/// progressively enhanced to use client-side routing.
#[component]
pub fn MultiActionForm<ServFn, OutputProtocol>(
    /// The action from which to build the form.
    action: ServerMultiAction<ServFn>,
    /// A [`NodeRef`] in which the `<form>` element should be stored.
    #[prop(optional)]
    node_ref: Option<NodeRef<Form>>,
    /// Component children; should include the HTML of the form elements.
    children: Children,
) -> impl IntoView
where
    ServFn: Send
        + Sync
        + Clone
        + DeserializeOwned
        + ServerFn<Protocol = Http<PostUrl, OutputProtocol>>
        + 'static,
    ServFn::Output: Send + Sync + 'static,
    <<ServFn::Client as Client<ServFn::Error>>::Request as ClientReq<
        ServFn::Error,
    >>::FormData: From<FormData>,
    ServFn::Error: Send + Sync + 'static,
    <ServFn as ServerFn>::Client: Client<<ServFn as ServerFn>::Error>,
{
    // if redirect hook has not yet been set (by a router), defaults to a browser redirect
    _ = server_fn::redirect::set_redirect_hook(|loc: &str| {
        if let Some(url) = resolve_redirect_url(loc) {
            _ = window().location().set_href(&url.href());
        }
    });

    let on_submit = move |ev: SubmitEvent| {
        if ev.default_prevented() {
            return;
        }

        ev.prevent_default();

        match ServFn::from_event(&ev) {
            Ok(new_input) => {
                action.dispatch(new_input);
            }
            Err(err) => {
                action.dispatch_sync(Err(ServerFnErrorErr::Serialization(
                    err.to_string(),
                )
                .into_app_error()));
            }
        }
    };

    let action_form = form()
        .action(ServFn::url())
        .method("post")
        .attr("method", "post")
        .on(submit, on_submit)
        .child(children());
    if let Some(node_ref) = node_ref {
        Either::Left(action_form.node_ref(node_ref))
    } else {
        Either::Right(action_form)
    }
}

/// Resolves a redirect location to an (absolute) URL.
pub(crate) fn resolve_redirect_url(loc: &str) -> Option<web_sys::Url> {
    let origin = match window().location().origin() {
        Ok(origin) => origin,
        Err(e) => {
            leptos::logging::error!("Failed to get origin: {:#?}", e);
            return None;
        }
    };

    // TODO: Use server function's URL as base instead.
    let base = origin;

    match web_sys::Url::new_with_base(loc, &base) {
        Ok(url) => Some(url),
        Err(e) => {
            leptos::logging::error!(
                "Invalid redirect location: {}",
                e.as_string().unwrap_or_default(),
            );
            None
        }
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

/// Errors that can arise when converting from an HTML event or form into a Rust data type.
#[derive(Error, Debug)]
pub enum FromFormDataError {
    /// Could not find a `<form>` connected to the event.
    #[error("Could not find <form> connected to event.")]
    MissingForm(Event),
    /// Could not create `FormData` from the form.
    #[error("Could not create FormData from <form>: {0:?}")]
    FormData(JsValue),
    /// Failed to deserialize this Rust type from the form data.
    #[error("Deserialization error: {0:?}")]
    Deserialization(serde_qs::Error),
}

impl<T> FromFormData for T
where
    T: serde::de::DeserializeOwned,
{
    fn from_event(ev: &Event) -> Result<Self, FromFormDataError> {
        let submit_ev = ev.unchecked_ref();
        let form_data = form_data_from_event(submit_ev)?;
        Self::from_form_data(&form_data)
            .map_err(FromFormDataError::Deserialization)
    }

    fn from_form_data(
        form_data: &web_sys::FormData,
    ) -> Result<Self, serde_qs::Error> {
        let data =
            web_sys::UrlSearchParams::new_with_str_sequence_sequence(form_data)
                .unwrap_throw();
        let data = data.to_string().as_string().unwrap_or_default();
        serde_qs::Config::new()
            .use_form_encoding(true)
            .deserialize_str::<Self>(&data)
    }
}

fn form_data_from_event(
    ev: &SubmitEvent,
) -> Result<FormData, FromFormDataError> {
    let form = ev
        .target()
        .and_then(|target| target.dyn_into::<HtmlFormElement>().ok())
        .ok_or_else(|| FromFormDataError::MissingForm(ev.clone().into()))?;
    form_data_with_submitter(&form, ev.submitter().as_ref())
        .map_err(FromFormDataError::FormData)
}
