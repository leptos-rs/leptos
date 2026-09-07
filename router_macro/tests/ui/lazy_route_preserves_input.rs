use leptos_router_macro::lazy_route;

struct Route;

trait LazyRoute {
    const PRESERVED: ();
    async fn view(this: Self);
}

#[lazy_route]
impl LazyRoute for Route {
    const PRESERVED: () = ();

    async fn view(_this: Self) {}
}

fn main() {
    let _ = Route::PRESERVED;
}
