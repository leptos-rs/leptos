mod route;
mod router;
mod routes;

use leptos_dom::IntoChild;
use leptos_reactive::Scope;

pub use route::*;
pub use router::*;
pub use routes::*;

use crate::use_route;

#[allow(non_snake_case)]
pub fn Outlet(cx: Scope) -> impl IntoChild {
    let route = use_route(cx);
    move || route.child().map(|child| child.outlet())
}
