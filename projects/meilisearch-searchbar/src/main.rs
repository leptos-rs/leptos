#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{routing::get, Extension, Router};
    use leptos::get_configuration;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use meilisearch_searchbar::StockRow;
    use meilisearch_searchbar::{fallback::file_and_error_handler, *};

    // simple_logger is a lightweight alternative to tracing, when you absolutely have to trace, use tracing.
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();

    let mut rdr = csv::Reader::from_path("data_set.csv").unwrap();

    // Our data set doesn't have a good id for the purposes of meilisearch, Name is unique but it's not formatted correctly because it may have spaces.
    let documents: Vec<StockRow> = rdr
        .records()
        .enumerate()
        .map(|(i, rec)| {
            // There's probably a better way to do this.
            let mut record = csv::StringRecord::new();
            record.push_field(&i.to_string());
            for field in rec.unwrap().iter() {
                record.push_field(field);
            }
            record
                .deserialize::<StockRow>(None)
                .expect(&format!("{:?}", record))
        })
        .collect();

    // My own check. I know how long I expect it to be, if it's not this length something is wrong.
    assert_eq!(documents.len(), 503);

    let client = meilisearch_sdk::Client::new(
        std::env::var("MEILISEARCH_URL").unwrap(),
        std::env::var("MEILISEARCH_API_KEY").ok(),
    );
    // An index is where the documents are stored.
    let task = client
        .create_index("stock_prices", Some("id"))
        .await
        .unwrap();

    // Meilisearch may take some time to execute the request so we are going to wait till it's completed
    client.wait_for_task(task, None, None).await.unwrap();

    let task_2 = client
        .get_index("stock_prices")
        .await
        .unwrap()
        .add_documents(&documents, Some("id"))
        .await
        .unwrap();

    client.wait_for_task(task_2, None, None).await.unwrap();
    
    drop(documents);

    let conf = get_configuration(Some("Cargo.toml")).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    // build our application with a route
    let app = Router::new()
        .route("/favicon.ico", get(file_and_error_handler))
        .leptos_routes(&leptos_options, routes, App)
        .fallback(file_and_error_handler)
        .layer(Extension(client))
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    println!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
