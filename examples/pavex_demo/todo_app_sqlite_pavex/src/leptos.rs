use leptos::{get_configuration, leptos_config::ConfFile};
use leptos_pavex::generate_route_list;
use leptos_router::RouteListing;
use pavex::{
    http::header::{ToStrError, USER_AGENT},
    request::RequestHead,
    response::Response,
};

/// Easier to do this to avoid having to register things with Blueprints
/// Provide LeptosOptions via env vars provided by cargo-leptos or the user
pub fn get_cargo_leptos_conf() -> ConfFile {
    get_configuration(None)
}

/// Generate all possible non server fn routes for our app
pub fn get_app_route_listing() -> Vec<RouteListing> {
    generate_route_list(TodoApp)
}
