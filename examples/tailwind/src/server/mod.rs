mod generated;

use crate::app::*;
use actix_files::Files;
use actix_web::*;
use futures::StreamExt;
use generated::{HTML_END, HTML_MIDDLE, HTML_START};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[derive(Copy, Clone, Debug)]
struct ActixIntegration {
    path: ReadSignal<String>,
}

impl History for ActixIntegration {
    fn location(&self, cx: leptos::Scope) -> ReadSignal<LocationChange> {
        create_signal(
            cx,
            LocationChange {
                value: self.path.get(),
                replace: false,
                scroll: true,
                state: State(None),
            },
        )
        .0
    }

    fn navigate(&self, _loc: &LocationChange) {}
}

// match every path — our router will handle actual dispatch
#[get("{tail:.*}")]
async fn render_app(req: HttpRequest) -> impl Responder {
    let path = req.path();

    let query = req.query_string();
    let path = if query.is_empty() {
        "http://leptos".to_string() + path
    } else {
        "http://leptos".to_string() + path + "?" + query
    };

    let app = move |cx| {
        let integration = ActixIntegration {
            path: create_signal(cx, path.clone()).0,
        };
        provide_context(cx, RouterIntegrationContext(std::rc::Rc::new(integration)));

        view! { cx, <App /> }
    };

    HttpResponse::Ok().content_type("text/html").streaming(
        futures::stream::once(async { HTML_START.to_string() })
            .chain(render_to_stream(move |cx| {
                use_context::<MetaContext>(cx)
                    .map(|meta| meta.dehydrate())
                    .unwrap_or_default()
            }))
            .chain(futures::stream::once(async { HTML_MIDDLE.to_string() }))
            .chain(render_to_stream(move |cx| app(cx).to_string()))
            .chain(futures::stream::once(async { HTML_END.to_string() }))
            .map(|html| Ok(web::Bytes::from(html)) as Result<web::Bytes>),
    )
}

pub async fn run() -> std::io::Result<()> {
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap();

    simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

    log::info!("serving at {host}:{port}");

    HttpServer::new(|| {
        App::new()
            .service(
                web::scope("/pkg")
                    .service(Files::new("", "target/site/pkg"))
                    .wrap(middleware::Compress::default()),
            )
            .service(render_app)
    })
    .bind((host, port))?
    .run()
    .await
}
