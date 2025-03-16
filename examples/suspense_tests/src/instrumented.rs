use leptos::prelude::*;
use leptos_router::{
    components::{ParentRoute, Route, A},
    hooks::use_params,
    nested_router::Outlet,
    params::Params,
    ParamSegment, SsrMode, StaticSegment, WildcardSegment,
};

#[cfg(feature = "ssr")]
pub(super) mod counter {
    use std::{
        collections::HashMap,
        sync::{
            atomic::{AtomicU32, Ordering},
            LazyLock, Mutex,
        },
    };

    #[derive(Default)]
    pub struct Counter(AtomicU32);

    impl Counter {
        #[allow(dead_code)]
        pub const fn new() -> Self {
            Self(AtomicU32::new(0))
        }

        pub fn get(&self) -> u32 {
            self.0.load(Ordering::SeqCst)
        }

        pub fn inc(&self) -> u32 {
            self.0.fetch_add(1, Ordering::SeqCst)
        }

        pub fn reset(&self) {
            self.0.store(0, Ordering::SeqCst);
        }
    }

    #[derive(Default)]
    pub struct Counters {
        pub list_items: Counter,
        pub get_item: Counter,
        pub inspect_item_root: Counter,
        pub inspect_item_field: Counter,
    }

    impl From<&mut Counters> for super::Counters {
        fn from(counter: &mut Counters) -> Self {
            Self {
                get_item: counter.get_item.get(),
                inspect_item_root: counter.inspect_item_root.get(),
                inspect_item_field: counter.inspect_item_field.get(),
                list_items: counter.list_items.get(),
            }
        }
    }

    impl Counters {
        pub fn reset(&self) {
            self.get_item.reset();
            self.inspect_item_root.reset();
            self.inspect_item_field.reset();
            self.list_items.reset();
        }
    }

    pub static COUNTERS: LazyLock<Mutex<HashMap<u64, Counters>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Item {
    id: i64,
    name: Option<String>,
    field: Option<String>,
}

#[server]
async fn list_items(ticket: u64) -> Result<Vec<i64>, ServerFnError> {
    // emulate database query overhead
    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    (*counter::COUNTERS)
        .lock()
        .expect("somehow panicked elsewhere")
        .entry(ticket)
        .or_default()
        .list_items
        .inc();
    Ok(vec![1, 2, 3, 4])
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct GetItemResult(pub Item, pub Vec<String>);

#[server]
async fn get_item(
    ticket: u64,
    id: i64,
) -> Result<GetItemResult, ServerFnError> {
    // emulate database query overhead
    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    (*counter::COUNTERS)
        .lock()
        .expect("somehow panicked elsewhere")
        .entry(ticket)
        .or_default()
        .get_item
        .inc();
    let name = None::<String>;
    let field = None::<String>;
    Ok(GetItemResult(
        Item { id, name, field },
        ["path1", "path2", "path3"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
    ))
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct InspectItemResult(pub Item, pub String, pub Vec<String>);

#[server]
async fn inspect_item(
    ticket: u64,
    id: i64,
    path: String,
) -> Result<InspectItemResult, ServerFnError> {
    // emulate database query overhead
    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    let mut split = path.split('/');
    let name = split.next().map(str::to_string);
    let path = name
        .clone()
        .expect("name should have been defined at this point");
    let field = split
        .next()
        .and_then(|s| (!s.is_empty()).then(|| s.to_string()));
    if field.is_none() {
        (*counter::COUNTERS)
            .lock()
            .expect("somehow panicked elsewhere")
            .entry(ticket)
            .or_default()
            .inspect_item_root
            .inc();
    } else {
        (*counter::COUNTERS)
            .lock()
            .expect("somehow panicked elsewhere")
            .entry(ticket)
            .or_default()
            .inspect_item_field
            .inc();
    }
    Ok(InspectItemResult(
        Item { id, name, field },
        path,
        ["field1", "field2", "field3"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
    ))
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Counters {
    pub get_item: u32,
    pub inspect_item_root: u32,
    pub inspect_item_field: u32,
    pub list_items: u32,
}

#[server]
async fn get_counters(ticket: u64) -> Result<Counters, ServerFnError> {
    Ok((*counter::COUNTERS)
        .lock()
        .expect("somehow panicked elsewhere")
        .entry(ticket)
        .or_default()
        .into())
}

#[server(ResetCounters)]
async fn reset_counters(ticket: u64) -> Result<(), ServerFnError> {
    (*counter::COUNTERS)
        .lock()
        .expect("somehow panicked elsewhere")
        .entry(ticket)
        .or_default()
        .reset();
    // leptos::logging::log!("counters for ticket {ticket} have been reset");
    Ok(())
}

#[derive(Clone, Default)]
pub struct SuspenseCounters {
    item_overview: u32,
    item_inspect: u32,
    item_listing: u32,
}

#[component]
pub fn InstrumentedRoutes() -> impl leptos_router::MatchNestedRoutes + Clone {
    // TODO should make this mode configurable via feature flag?
    let ssr = SsrMode::Async;
    view! {
        <ParentRoute path=StaticSegment("instrumented") view=InstrumentedRoot ssr>
            <Route path=StaticSegment("/") view=InstrumentedTop />
            <ParentRoute path=StaticSegment("item") view=ItemRoot>
                <Route path=StaticSegment("/") view=ItemListing />
                <ParentRoute path=ParamSegment("id") view=ItemTop>
                    <Route path=StaticSegment("/") view=ItemOverview />
                    <Route path=WildcardSegment("path") view=ItemInspect />
                </ParentRoute>
            </ParentRoute>
            <Route path=StaticSegment("counters") view=ShowCounters />
        </ParentRoute>
    }
    .into_inner()
}

#[derive(Copy, Clone)]
pub struct Ticket(pub u64);

#[derive(Copy, Clone)]
pub struct CSRTicket(pub u64);

#[cfg(feature = "ssr")]
fn inst_ticket() -> u64 {
    // SSR will always use 0 for the ticket
    0
}

#[cfg(not(feature = "ssr"))]
fn inst_ticket() -> u64 {
    // CSR will use a random number for the ticket
    (js_sys::Math::random() * ((u64::MAX - 1) as f64) + 1f64) as u64
}

#[component]
fn InstrumentedRoot() -> impl IntoView {
    let counters = RwSignal::new(SuspenseCounters::default());
    provide_context(counters);
    provide_field_nav_portlet_context();

    // Generate a ID directly on this component.  Rather than relying on
    // additional server functions, doing it this way emulates more
    // standard workflows better and to avoid having to add another
    // thing to instrument/interfere with the typical use case.
    // Downside is that randomness has a chance to conflict.
    //
    // Furthermore, this approach **will** result in unintuitive
    // behavior when it isn't accounted for - specifically, the reason
    // for this design is that when SSR it will guarantee usage of `0`
    // as the ticket, while CSR it will be of some other value as the
    // version it uses will be random.  However, when trying to get back
    // the counters associated with the ticket, rendering using SSR will
    // always produce the SSR version and this quirk will need to be
    // accounted for.
    let ticket = inst_ticket();
    // leptos::logging::log!(
    //     "Ticket for this InstrumentedRoot instance: {ticket}"
    // );
    provide_context(Ticket(ticket));

    let csr_ticket = RwSignal::<Option<CSRTicket>>::new(None);

    let reset_counters = ServerAction::<ResetCounters>::new();

    Effect::new(move |_| {
        let ticket = expect_context::<Ticket>().0;
        csr_ticket.set(Some(CSRTicket(ticket)));
    });

    view! {
        <section id="instrumented">
            <nav>
                <a href="/">"Site Root"</a>
                <A href="./" exact=true>
                    "Instrumented Root"
                </A>
                <A href="item/" strict_trailing_slash=true>
                    "Item Listing"
                </A>
                <A href="counters" strict_trailing_slash=true>
                    "Counters"
                </A>
            </nav>
            <FieldNavPortlet />
            <Outlet />
            <Suspense>
                {move || Suspend::new(async move {
                    let clear_suspense_counters = move |_| {
                        counters.update(|c| *c = SuspenseCounters::default());
                    };
                    csr_ticket
                        .get()
                        .map(|ticket| {
                            let ticket = ticket.0;
                            view! {
                                <ActionForm action=reset_counters>
                                    <input type="hidden" name="ticket" value=format!("{ticket}") />
                                    <input
                                        id="reset-csr-counters"
                                        type="submit"
                                        value="Reset CSR Counters"
                                        on:click=clear_suspense_counters
                                    />
                                </ActionForm>
                            }
                        })
                })}
            </Suspense>
            <footer>
                <nav>
                    <A href="item/3/">"Target 3##"</A>
                    <A href="item/4/">"Target 4##"</A>
                    <A href="item/4/path1/">"Target 41#"</A>
                    <A href="item/4/path2/">"Target 42#"</A>
                    <A href="item/4/path2/field1">"Target 421"</A>
                    <A href="item/1/path2/field3">"Target 123"</A>
                </nav>
            </footer>
        </section>
    }
}

#[component]
fn InstrumentedTop() -> impl IntoView {
    view! {
        <h1>"Instrumented Tests"</h1>
        <p>
            "These tests validates the number of invocations of server functions and suspenses per access."
        </p>
        <ul>
            // not using `A` because currently some bugs with artix
            <li>
                <a href="item/">"Item Listing"</a>
            </li>
            <li>
                <a href="item/4/path1/">"Target 41#"</a>
            </li>
        </ul>
    }
}

#[component]
fn ItemRoot() -> impl IntoView {
    let ticket = expect_context::<Ticket>().0;
    provide_context(Resource::new_blocking(
        move || (),
        move |_| async move { list_items(ticket).await },
    ));

    view! {
        <h2>"<ItemRoot/>"</h2>
        <Outlet />
    }
}

#[component]
fn ItemListing() -> impl IntoView {
    let suspense_counters = expect_context::<RwSignal<SuspenseCounters>>();
    let resource =
        expect_context::<Resource<Result<Vec<i64>, ServerFnError>>>();
    let item_listing = move || {
        Suspend::new(async move {
            let result = resource.await.map(|items| items
            .into_iter()
            .map(move |item|
                // FIXME seems like relative link isn't working, it is currently
                // adding an extra `/` in artix; manually construct `a` instead.
                // <li><A href=format!("./{item}/")>"Item "{item}</A></li>
                view! {
                    <li>
                        <a href=format!("/instrumented/item/{item}/")>"Item "{item}</a>
                    </li>
                }
            )
            .collect_view()
        );
            suspense_counters.update_untracked(|c| c.item_listing += 1);
            result
        })
    };

    view! {
        <h3>"<ItemListing/>"</h3>
        <ul>
            <Suspense>{item_listing}</Suspense>
        </ul>
    }
}

#[derive(Params, PartialEq, Clone, Debug)]
struct ItemTopParams {
    id: Option<i64>,
}

#[component]
fn ItemTop() -> impl IntoView {
    let ticket = expect_context::<Ticket>().0;
    let params = use_params::<ItemTopParams>();
    // map result to an option as the focus isn't error rendering
    provide_context(Resource::new_blocking(
        move || params.get().map(|p| p.id),
        move |id| async move {
            match id {
                Err(_) => None,
                Ok(Some(id)) => get_item(ticket, id).await.ok(),
                _ => None,
            }
        },
    ));
    view! {
        <h4>"<ItemTop/>"</h4>
        <Outlet />
    }
}

#[component]
fn ItemOverview() -> impl IntoView {
    let suspense_counters = expect_context::<RwSignal<SuspenseCounters>>();
    let resource = expect_context::<Resource<Option<GetItemResult>>>();
    let item_view = move || {
        Suspend::new(async move {
            let result = resource.await.map(|GetItemResult(item, names)| {
                view! {
                    <p>{format!("Viewing {item:?}")}</p>
                    <ul>
                        {names
                            .into_iter()
                            .map(|name| {
                                let id = item.id;
                                // FIXME seems like relative link isn't working, it is currently
                                // adding an extra `/` in artix; manually construct `a` instead.
                                // <li><A href=format!("./{name}/")>{format!("Inspect {name}")}</A></li>
                                view! {
                                    <li>
                                        <a href=format!(
                                            "/instrumented/item/{id}/{name}/",
                                        )>"Inspect "{name.clone()}</a>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                }
            });
            suspense_counters.update_untracked(|c| c.item_overview += 1);
            result
        })
    };

    view! {
        <h5>"<ItemOverview/>"</h5>
        <Suspense>{item_view}</Suspense>
    }
}

#[derive(Params, PartialEq, Clone, Debug)]
struct ItemInspectParams {
    path: Option<String>,
}

#[component]
fn ItemInspect() -> impl IntoView {
    let ticket = expect_context::<Ticket>().0;
    let suspense_counters = expect_context::<RwSignal<SuspenseCounters>>();
    let params = use_params::<ItemInspectParams>();
    let res_overview = expect_context::<Resource<Option<GetItemResult>>>();
    let res_inspect = Resource::new_blocking(
        move || params.get().map(|p| p.path),
        move |p| async move {
            // leptos::logging::log!("res_inspect: res_overview.await");
            let overview = res_overview.await;
            // leptos::logging::log!("res_inspect: resolved res_overview.await");
            // let result =
            match (overview, p) {
                (Some(item), Ok(Some(path))) => {
                    // leptos::logging::log!("res_inspect: inspect_item().await");
                    inspect_item(ticket, item.0.id, path.clone()).await.ok()
                }
                _ => None,
            }
            // ;
            // leptos::logging::log!("res_inspect: resolved inspect_item().await");
            // result
        },
    );
    let ws = use_context::<WriteSignal<Option<FieldNavCtx>>>();
    on_cleanup(move || {
        if let Some(c) = ws {
            c.set(None);
        }
    });
    let inspect_view = move || {
        // leptos::logging::log!("inspect_view closure invoked");
        Suspend::new(async move {
            // leptos::logging::log!("inspect_view Suspend::new() called");
            let result = res_inspect.await.map(|InspectItemResult(item, name, fields)| {
                // leptos::logging::log!("inspect_view res_inspect awaited");
                let id = item.id;
                expect_context::<WriteSignal<Option<FieldNavCtx>>>().set(Some(
                    fields.iter()
                        .map(|field| FieldNavItem {
                            href: format!("/instrumented/item/{id}/{name}/{field}"),
                            text: field.to_string(),
                        })
                        .collect::<Vec<_>>()
                        .into()
                ));
                view! {
                    <p>{format!("Inspecting {item:?}")}</p>
                    <ul>
                        {fields
                            .iter()
                            .map(|field| {
                                // FIXME seems like relative link to root for a wildcard isn't
                                // working as expected, so manually construct `a` instead.
                                // let text = format!("Inspect {name}/{field}");
                                // view! {
                                // <li><A href=format!("{field}")>{text}</A></li>
                                // }
                                view! {
                                    <li>
                                        <a href=format!(
                                            "/instrumented/item/{id}/{name}/{field}",
                                        )>{format!("Inspect {name}/{field}")}</a>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                }
            });
            suspense_counters.update_untracked(|c| c.item_inspect += 1);
            // leptos::logging::log!(
            //     "returning result, result.is_some() = {}, count = {}",
            //     result.is_some(),
            //     suspense_counters.get().item_inspect,
            // );
            result
        })
    };

    view! {
        <h5>"<ItemInspect/>"</h5>
        <Suspense>{inspect_view}</Suspense>
    }
}

#[component]
fn ShowCounters() -> impl IntoView {
    // There is _weirdness_ in this view.  The `Server Calls` counters
    // will be acquired via the expected mode and be rendered as such.
    //
    // However, upon `Reset Counters`, the mode from which the reset
    // was issued will result in the rendering be reflected as such, so
    // if the intial state was SSR, resetting under CSR will result in
    // the CSR counters be rendered after.  However for the intents and
    // purpose for the testing only the CSR is cared for.
    //
    // At the end of the day, it is possible to have both these be
    // separated out, but for the purpose of this test the focus is not
    // on the SSR side of things (at least until further regression is
    // discovered that affects SSR directly).
    let ticket = expect_context::<Ticket>().0;
    let suspense_counters = expect_context::<RwSignal<SuspenseCounters>>();
    let reset_counters = ServerAction::<ResetCounters>::new();
    let res_counter = Resource::new(
        move || reset_counters.version().get(),
        move |_| async move {
            (
                get_counters(ticket).await,
                if ticket == 0 { "SSR" } else { "CSR" }.to_string(),
                ticket,
            )
        },
    );
    let counter_view = move || {
        Suspend::new(async move {
            // ensure current mode and ticket are both updated
            let (counters, mode, ticket) = res_counter.await;
            counters.map(|counters| {
                let clear_suspense_counters = move |_| {
                    suspense_counters.update(|c| {
                        // leptos::logging::log!("resetting suspense counters");
                        *c = SuspenseCounters::default();
                    });
                };
                view! {
                    <h3 id="server-calls">"Server Calls ("{mode}")"</h3>
                    <dl>
                        <dt>"list_items"</dt>
                        <dd id="list_items">{counters.list_items}</dd>
                        <dt>"get_item"</dt>
                        <dd id="get_item">{counters.get_item}</dd>
                        <dt>"inspect_item_root"</dt>
                        <dd id="inspect_item_root">{counters.inspect_item_root}</dd>
                        <dt>"inspect_item_field"</dt>
                        <dd id="inspect_item_field">{counters.inspect_item_field}</dd>
                    </dl>
                    <ActionForm action=reset_counters>
                        <input type="hidden" name="ticket" value=format!("{ticket}") />
                        <input
                            id="reset-counters"
                            type="submit"
                            value="Reset Counters"
                            on:click=clear_suspense_counters
                        />
                    </ActionForm>
                }
            })
        })
    };

    view! {
        <h2>"Counters"</h2>

        <h3 id="suspend-calls">"Suspend Calls"</h3>
        {move || {
            suspense_counters
                .with(|c| {
                    view! {
                        <dl>
                            <dt>"item_listing"</dt>
                            <dd id="item_listing">{c.item_listing}</dd>
                            <dt>"item_overview"</dt>
                            <dd id="item_overview">{c.item_overview}</dd>
                            <dt>"item_inspect"</dt>
                            <dd id="item_inspect">{c.item_inspect}</dd>
                        </dl>
                    }
                })
        }}

        <Suspense>{counter_view}</Suspense>
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct FieldNavItem {
    pub href: String,
    pub text: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct FieldNavCtx(pub Option<Vec<FieldNavItem>>);

impl From<Vec<FieldNavItem>> for FieldNavCtx {
    fn from(item: Vec<FieldNavItem>) -> Self {
        Self(Some(item))
    }
}

#[component]
pub fn FieldNavPortlet() -> impl IntoView {
    let ctx = expect_context::<ReadSignal<Option<FieldNavCtx>>>();
    move || {
        let ctx = ctx.get();
        ctx.map(|ctx| {
            view! {
                <div id="FieldNavPortlet">
                    <span>"FieldNavPortlet:"</span>
                    <nav>
                        {ctx
                            .0
                            .map(|ctx| {
                                ctx.into_iter()
                                    .map(|FieldNavItem { href, text }| {
                                        view! { <A href=href>{text}</A> }
                                    })
                                    .collect_view()
                            })}
                    </nav>
                </div>
            }
        })
    }
}

pub fn provide_field_nav_portlet_context() {
    // wrapping the Ctx in an Option allows better ergonomics whenever it isn't needed
    let (ctx, set_ctx) = signal(None::<FieldNavCtx>);
    provide_context(ctx);
    provide_context(set_ctx);
}
