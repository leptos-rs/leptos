#[cfg(any(feature = "ssr", doc))]
use crate::ServerFnTraitObj;
pub use server_fn_macro_default::server;
#[cfg(any(feature = "ssr", doc))]
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[cfg(any(feature = "ssr", doc))]
lazy_static::lazy_static! {
    static ref REGISTERED_SERVER_FUNCTIONS: Arc<RwLock<HashMap<&'static str, &'static DefaultServerFnTraitObj>>> = {
        let mut map = HashMap::new();
        for server_fn in inventory::iter::<DefaultServerFnTraitObj> {
            map.insert(server_fn.0.url(), server_fn);
        }
        Arc::new(RwLock::new(map))
    };
}

#[cfg(feature = "ssr")]
inventory::collect!(DefaultServerFnTraitObj);

/// Attempts to find a server function registered at the given path.
///
/// This can be used by a server to handle the requests, as in the following example (using `actix-web`)
///
/// ```rust, ignore
/// #[post("{tail:.*}")]
/// async fn handle_server_fns(
///     req: HttpRequest,
///     params: web::Path<String>,
///     body: web::Bytes,
/// ) -> impl Responder {
///     let path = params.into_inner();
///     let accept_header = req
///         .headers()
///         .get("Accept")
///         .and_then(|value| value.to_str().ok());
///
///     if let Some(server_fn) = server_fn_by_path(path.as_str()) {
///         let body: &[u8] = &body;
///         match server_fn(&body).await {
///             Ok(serialized) => {
///                 // if this is Accept: application/json then send a serialized JSON response
///                 if let Some("application/json") = accept_header {
///                     HttpResponse::Ok().body(serialized)
///                 }
///                 // otherwise, it's probably a <form> submit or something: redirect back to the referrer
///                 else {
///                     HttpResponse::SeeOther()
///                         .insert_header(("Location", "/"))
///                         .content_type("application/json")
///                         .body(serialized)
///                 }
///             }
///             Err(e) => {
///                 eprintln!("server function error: {e:#?}");
///                 HttpResponse::InternalServerError().body(e.to_string())
///             }
///         }
///     } else {
///         HttpResponse::BadRequest().body(format!("Could not find a server function at that route."))
///     }
/// }
/// ```
#[cfg(any(feature = "ssr", doc))]
pub fn server_fn_by_path(
    path: &str,
) -> Option<&'static DefaultServerFnTraitObj> {
    REGISTERED_SERVER_FUNCTIONS
        .read()
        .expect("Server function registry is poisoned")
        .get(path)
        .copied()
}

/// Returns the set of currently-registered server function paths, for debugging purposes.
#[cfg(any(feature = "ssr", doc))]
pub fn server_fns_by_path() -> Vec<&'static str> {
    REGISTERED_SERVER_FUNCTIONS
        .read()
        .expect("Server function registry is poisoned")
        .keys()
        .copied()
        .collect()
}

#[cfg(any(feature = "ssr", doc))]
/// A server function that can be called from the client without any context from the server.
pub struct DefaultServerFnTraitObj(ServerFnTraitObj<()>);

#[cfg(any(feature = "ssr", doc))]
impl DefaultServerFnTraitObj {
    /// Creates a new server function with the given prefix, URL, encoding, and function.
    pub const fn from_generic_server_fn(f: ServerFnTraitObj<()>) -> Self {
        Self(f)
    }
}

#[cfg(any(feature = "ssr", doc))]
impl std::ops::Deref for DefaultServerFnTraitObj {
    type Target = ServerFnTraitObj<()>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(any(feature = "ssr", doc))]
impl std::ops::DerefMut for DefaultServerFnTraitObj {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
