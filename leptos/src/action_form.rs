use crate::{
    children::Children, component, from_form_data::FromFormData, prelude::*,
    IntoView,
};
use leptos_dom::{events::submit, helpers::window};
use leptos_server::ServerAction;
use serde::de::DeserializeOwned;
use server_fn::{
    client::Client, codec::PostUrl, request::ClientReq, ServerFn, ServerFnError,
};
use tachys::{
    either::Either,
    html::{
        attribute::any_attribute::AnyAttribute,
        element::{form, Form},
    },
    reactive_graph::node_ref::NodeRef,
    renderer::dom::Dom,
};
use web_sys::{FormData, SubmitEvent};

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
    action: ServerAction<ServFn>,
    /// A [`NodeRef`] in which the `<form>` element should be stored.
    #[prop(optional)]
    node_ref: Option<NodeRef<Form>>,
    /// Arbitrary attributes to add to the `<form>`
    #[prop(attrs, optional)]
    attributes: Vec<AnyAttribute<Dom>>,
    /// Component children; should include the HTML of the form elements.
    children: Children,
) -> impl IntoView
where
    ServFn: DeserializeOwned
        + ServerFn<InputEncoding = PostUrl>
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
{
    // if redirect hook has not yet been set (by a router), defaults to a browser redirect
    _ = server_fn::redirect::set_redirect_hook(|loc: &str| {
        if let Some(url) = resolve_redirect_url(loc) {
            _ = window().location().set_href(&url.href());
        }
    });

    let action_url = ServFn::url();
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
                    value.set(Some(Err(ServerFnError::Serialization(
                        err.to_string(),
                    ))));
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
    // TODO add other attributes
    /*for (attr_name, attr_value) in attributes {
        action_form = action_form.attr(attr_name, attr_value);
    }*/
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
