use leptos_pavex::{LeptosOptions, RouteListing};
use pavex::{
    blueprint::{
        constructor::{CloningStrategy, Lifecycle},
        router::{ANY, GET},
        Blueprint,
    },
    f,
};
/// The main blueprint, containing all the routes, constructors and error handlers
/// required by our API.
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    register_common_constructors(&mut bp);

    bp.constructor(
        f!(crate::user_agent::UserAgent::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(crate::user_agent::invalid_user_agent));

    add_telemetry_middleware(&mut bp);

    bp.route(GET, "/test/ping", f!(crate::routes::status::ping));
    bp.route(GET, "/test/greet/:name", f!(crate::routes::greet::greet));
    // Handle all /api requests as those are Leptos server fns
    bp.route(ANY, "/api/*fn_name", f!(leptos_pavex::handle_server_fns));
    bp.route(ANY, "/");
    bp.fallback(f!(file_handler));
    bp
}

/// Common constructors used by all routes.
fn register_common_constructors(bp: &mut Blueprint) {
    // Configuration Options
    bp.constructor(
        f!(crate::leptos::get_cargo_leptos_conf(), Lifecycle::Singleton),
        Lifecycle::Singleton,
    );
    // List of Routes
    bp.constructor(
        f!(crate::leptos::get_app_route_listing(), Lifecycle::Singleton),
        Lifecycle::Singleton,
    );

    bp.constructor(
        f!(leptos_pavex::PavexRequest::extract),
        LifeCycle::RequestScoped,
    );
    // Query parameters
    bp.constructor(
        f!(pavex::request::query::QueryParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::request::query::errors::ExtractQueryParamsError::into_response
    ));

    // Route parameters
    bp.constructor(
        f!(pavex::request::route::RouteParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::request::route::errors::ExtractRouteParamsError::into_response
    ));

    // Json body
    bp.constructor(
        f!(pavex::request::body::JsonBody::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::request::body::errors::ExtractJsonBodyError::into_response
    ));
    bp.constructor(
        f!(pavex::request::body::BufferedBody::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::request::body::errors::ExtractBufferedBodyError::into_response
    ));
    bp.constructor(
        f!(<pavex::request::body::BodySizeLimit as std::default::Default>::default),
        Lifecycle::RequestScoped,
    );
}

/// Add the telemetry middleware, as well as the constructors of its dependencies.
fn add_telemetry_middleware(bp: &mut Blueprint) {
    bp.constructor(
        f!(crate::telemetry::RootSpan::new),
        Lifecycle::RequestScoped,
    )
    .cloning(CloningStrategy::CloneIfNecessary);

    bp.wrap(f!(crate::telemetry::logger));
}
