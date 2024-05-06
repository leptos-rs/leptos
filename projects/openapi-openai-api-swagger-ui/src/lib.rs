pub mod app;
pub mod error_template;
#[cfg(feature = "ssr")]
pub mod fileserv;
#[cfg(feature="ssr")]
pub mod open_ai;
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}


#[cfg(feature="ssr")]
pub mod api_doc {
    use crate::app::__path_hello_world;
    use crate::app::SayHello;
    use crate::app::__path_name_list;
    #[derive(utoipa::OpenApi)]
    #[openapi(
        info(description = "My Api description"),
        paths(hello_world,name_list), components(schemas(SayHello)),
    )]
    pub struct ApiDoc;
}