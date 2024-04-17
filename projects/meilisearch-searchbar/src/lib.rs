use leptos::*;
use leptos_meta::*;
use leptos_router::*;
#[cfg(feature = "ssr")]
pub mod fallback;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    // initializes logging using the `log` crate
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount_to_body(App);
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    // Provide this two our search components, they'll share a read and write handle to a Vec<StockRow>.
    let search_results = create_rw_signal(Vec::<StockRow>::new());
    provide_context(search_results);
    view! {
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Meta name="description" content="Leptos implementation of a Meilisearch backed Searchbar."/>
        <Router>
            <main>
                <Routes>
                    <Route path="/" view=||view!{
                        <SearchBar/>
                        <SearchResults/>
                    }/>
                </Routes>
            </main>
        </Router>
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize, Debug)]
pub struct StockRow {
    id: u32,
    name: String,
    last: String,
    high: String,
    low: String,
    absolute_change: f32,
    percentage_change: f32,
    volume: u64,
}

#[leptos::server]
pub async fn search_query(query: String) -> Result<Vec<StockRow>, ServerFnError> {
    use leptos_axum::extract;
    // Wow, so ergonomic!
    let axum::Extension::<meilisearch_sdk::Client>(client) = extract().await?;
    // Meilisearch has great defaults, lots of things are thought of for out of the box utility.
    // They limit the result length automatically (to 20), and have user friendly typo corrections and return similar words.
    let hits = client
        .get_index("stock_prices")
        .await
        .unwrap()
        .search()
        .with_query(query.as_str())
        .execute::<StockRow>()
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))?
        .hits;
    
    Ok(hits
        .into_iter()
        .map(|search_result| search_result.result)
        .collect())
}

#[component]
pub fn SearchBar() -> impl IntoView {
    let write_search_results = expect_context::<RwSignal<Vec<StockRow>>>().write_only();
    let search_query = create_server_action::<SearchQuery>();
    create_effect(move |_| {
        if let Some(value) = search_query.value()() {
            match value {
                Ok(search_results) => {
                    write_search_results.set(search_results);
                }
                Err(err) => {
                    leptos::logging::log!("{err}")
                }
            }
        }
    });

    view! {
        <div>
            <label for="search">Search</label>
            <input id="search" on:input=move|e|{
                let query = event_target_value(&e);
                search_query.dispatch(SearchQuery{query});
            }/>
        </div>
    }
}

#[component]
pub fn SearchResults() -> impl IntoView {
    let read_search_results = expect_context::<RwSignal<Vec<StockRow>>>().read_only();
    view! {
        <ul>
               <For
                    each=read_search_results
                    key=|row| row.name.clone()
                    children=move |StockRow{name,last,high,low,absolute_change,percentage_change,volume,..}: StockRow| {
          view! {
                <li>
                    {format!("{name}; last: {last}; high: {high}; low: {low}; chg.: {absolute_change}; chg...:{percentage_change}; volume:{volume}")}
                </li>
          }
        }
      />
        </ul>
    }
}
