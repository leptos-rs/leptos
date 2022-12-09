mod app;
#[cfg(feature = "ssr")]
mod server;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "ssr")] {
        #[actix_web::main]
        async fn main() -> std::io::Result<()> {
            server::run().await
        }
    }
    else {
        pub fn main() {}
    }
}
