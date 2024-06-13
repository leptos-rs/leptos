use api_boundary::*;
use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use leptos_router::*;

mod api;
mod components;
mod pages;

use self::{components::*, pages::*};

const DEFAULT_API_URL: &str = "/api";
const API_TOKEN_STORAGE_KEY: &str = "api-token";

#[component]
pub fn App() -> impl IntoView {
    // -- signals -- //

    let authorized_api = RwSignal::new(None::<api::AuthorizedApi>);
    let user_info = RwSignal::new(None::<UserInfo>);
    let logged_in = Signal::derive(move || authorized_api.get().is_some());

    // -- actions -- //

    let fetch_user_info = create_action(move |_| async move {
        match authorized_api.get() {
            Some(api) => match api.user_info().await {
                Ok(info) => {
                    user_info.update(|i| *i = Some(info));
                }
                Err(err) => {
                    log::error!("Unable to fetch user info: {err}")
                }
            },
            None => {
                log::error!("Unable to fetch user info: not logged in")
            }
        }
    });

    let logout = create_action(move |_| async move {
        match authorized_api.get() {
            Some(api) => match api.logout().await {
                Ok(_) => {
                    authorized_api.update(|a| *a = None);
                    user_info.update(|i| *i = None);
                }
                Err(err) => {
                    log::error!("Unable to logout: {err}")
                }
            },
            None => {
                log::error!("Unable to logout user: not logged in")
            }
        }
    });

    // -- callbacks -- //

    let on_logout = move |_| {
        logout.dispatch(());
    };

    // -- init API -- //

    let unauthorized_api = api::UnauthorizedApi::new(DEFAULT_API_URL);
    if let Ok(token) = LocalStorage::get(API_TOKEN_STORAGE_KEY) {
        let api = api::AuthorizedApi::new(DEFAULT_API_URL, token);
        authorized_api.update(|a| *a = Some(api));
        fetch_user_info.dispatch(());
    }

    log::debug!("User is logged in: {}", logged_in.get_untracked());

    // -- effects -- //

    create_effect(move |_| {
        log::debug!("API authorization state changed");
        match authorized_api.get() {
            Some(api) => {
                log::debug!(
                    "API is now authorized: save token in LocalStorage"
                );
                LocalStorage::set(API_TOKEN_STORAGE_KEY, api.token())
                    .expect("LocalStorage::set");
            }
            None => {
                log::debug!(
                    "API is no longer authorized: delete token from \
                     LocalStorage"
                );
                LocalStorage::delete(API_TOKEN_STORAGE_KEY);
            }
        }
    });

    view! {
        <Router>
            <NavBar logged_in on_logout/>
            <main>
                <Routes>
                    <Route
                        path=Page::Home.path()
                        view=move || {
                            view! { <Home user_info=user_info.into()/> }
                        }
                    />
                    <Route
                        path=Page::Login.path()
                        view=move || {
                            view! {
                                <Login
                                    api=unauthorized_api
                                    on_success=move |api| {
                                        log::info!("Successfully logged in");
                                        authorized_api.update(|v| *v = Some(api));
                                        let navigate = use_navigate();
                                        navigate(Page::Home.path(), Default::default());
                                        fetch_user_info.dispatch(());
                                    }
                                />
                            }
                        }
                    />
                    <Route
                        path=Page::Register.path()
                        view=move || {
                            view! { <Register api=unauthorized_api/> }
                        }
                    />
                </Routes>
            </main>
        </Router>
    }
}
