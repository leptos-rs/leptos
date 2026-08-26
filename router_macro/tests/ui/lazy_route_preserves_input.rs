use leptos_router_macro::lazy_route;

struct Route;

#[lazy_route]
impl Route {
    const PRESERVED: () = ();
}

fn main() {
    let _ = Route::PRESERVED;
}
