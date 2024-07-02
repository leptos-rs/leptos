use crate::{
    api::{self, UnauthorizedApi},
    components::credentials::*,
    Page,
};
use api_boundary::*;
use leptos::{logging::log, *};
use leptos_router::*;

#[component]
pub fn Register(api: UnauthorizedApi) -> impl IntoView {
    let (register_response, set_register_response) = create_signal(None::<()>);
    let (register_error, set_register_error) = create_signal(None::<String>);
    let (wait_for_response, set_wait_for_response) = create_signal(false);

    let register_action =
        create_action(move |(email, password): &(String, String)| {
            let email = email.to_string();
            let password = password.to_string();
            let credentials = Credentials { email, password };
            log!("Try to register new account for {}", credentials.email);
            async move {
                set_wait_for_response.update(|w| *w = true);
                let result = api.register(&credentials).await;
                set_wait_for_response.update(|w| *w = false);
                match result {
                    Ok(res) => {
                        set_register_response.update(|v| *v = Some(res));
                        set_register_error.update(|e| *e = None);
                    }
                    Err(err) => {
                        let msg = match err {
                            api::Error::Fetch(js_err) => {
                                format!("{js_err:?}")
                            }
                            api::Error::Api(err) => err.message,
                        };
                        log::warn!(
                            "Unable to register new account for {}: {msg}",
                            credentials.email
                        );
                        set_register_error.update(|e| *e = Some(msg));
                    }
                }
            }
        });

    let disabled = Signal::derive(move || wait_for_response.get());

    view! {
        <Show
            when=move || register_response.get().is_some()
            fallback=move || {
                view! {
                    <CredentialsForm
                        title="Please enter the desired credentials"
                        action_label="Register"
                        action=register_action
                        error=register_error.into()
                        disabled
                    />
                    <p>"Your already have an account?"</p>
                    <A href=Page::Login.path()>"Login"</A>
                }
            }
        >
            <p>"You have successfully registered."</p>
            <p>"You can now " <A href=Page::Login.path()>"login"</A> " with your new account."</p>
        </Show>
    }
}
