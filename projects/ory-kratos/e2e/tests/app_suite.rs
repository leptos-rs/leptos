#![feature(never_type)]
mod fixtures;

use anyhow::anyhow;
use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::log::EventEntryAdded;
use chromiumoxide::cdp::js_protocol::runtime::EventConsoleApiCalled;
use chromiumoxide::{
    browser::{Browser, BrowserConfig},
    cdp::browser_protocol::{
        network::{EventRequestWillBeSent, EventResponseReceived, Request, Response},
        page::NavigateParams,
    },
    element::Element,
    page::ScreenshotParams,
    Page,
};
use cucumber::World;
use futures::channel::mpsc::Sender;
use futures_util::stream::StreamExt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tokio_tungstenite::connect_async;
use uuid::Uuid;
static EMAIL_ID_MAP: Lazy<RwLock<HashMap<String, String>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Clone, Debug, PartialEq)]
pub struct RequestPair {
    req: Option<Request>,
    redirect_resp: Option<Response>,
    resp: Option<Response>,
    cookies_before_request: String,
    cookies_after_response: String,
    ts: std::time::Instant,
}

/*
let screenshot =   world
  .page
  .screenshot(
      ScreenshotParams::builder()
          .capture_beyond_viewport(true)
          .full_page(true)
          .build(),
  )
  .await
  .unwrap();
  world.screenshots.push(screenshot);
   */
#[derive(Clone, Debug)]
pub enum CookieEnum {
    BeforeReq(String),
    AfterResp(String),
}
impl RequestPair {
    pub fn to_string(&self) -> String {
        let (top_req, req_headers) = if let Some(req) = &self.req {
            (
                format!("{} : {} \n", req.method, req.url,),
                format!("{} :\n{:#?} \n", req.url, req.headers),
            )
        } else {
            ("NO REQ".to_string(), "NO REQ".to_string())
        };
        let (top_redirect_resp, _redirect_resp_headers) = if let Some(resp) = &self.redirect_resp {
            (
                format!("{} : {}", resp.status, resp.url),
                format!("{} :\n {:#?}", resp.url, resp.headers),
            )
        } else {
            ("".to_string(), "".to_string())
        };
        let (top_resp, resp_headers) = if let Some(resp) = &self.resp {
            (
                format!("{} : {}", resp.status, resp.url),
                format!("{} :\n {:#?}", resp.url, resp.headers),
            )
        } else {
            ("NO RESP".to_string(), "NO RESP".to_string())
        };

        format!(
            "REQ: {}\n RESP: {}\n \n REDIRECT {} \n REQ_HEADERS: {} \n REQ_COOKIES: \n{}\n RESP_HEADERS:{} \n RESP_COOKIES: \n{}\n ",
            top_req, top_resp,top_redirect_resp,  req_headers,  self.cookies_before_request,resp_headers,self.cookies_after_response
        )
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // create a thread and store a
    //  tokio-tungstenite client that connectsto http://127.0.0.1:1080/ws
    // and then stores the recieved messages in a once_cell::Lazy<RwLock<Vec<MailCrabMsg>>>
    // or a custom struct that matches the body or has specific impls for verify codes, links etc.
    let _ = tokio::spawn(async move {
        let (mut socket, _) = connect_async(
            url::Url::parse("ws://127.0.0.1:1080/ws").expect("Can't connect to case count URL"),
        )
        .await
        .unwrap();
        while let Some(msg) = socket.next().await {
            if let Ok(tokio_tungstenite::tungstenite::Message::Text(text)) = msg {
                let Email { id, to } = serde_json::from_str::<Email>(&text).unwrap();
                let email = to[0].email.clone();
                EMAIL_ID_MAP.write().await.insert(email, id.to_string());
            }
        }
    });

    AppWorld::cucumber()
        .init_tracing()
        .fail_on_skipped()
        .max_concurrent_scenarios(1)
        .fail_fast()
        .before(|_feature, _rule, scenario, world| {
            Box::pin(async move {
                let screenshot_directory_name = format!("./screenshots/{}", scenario.name);
                if let Ok(sc_dir) = std::fs::read_dir(&screenshot_directory_name) {
                    for file in sc_dir {
                        if let Ok(file) = file {
                            std::fs::remove_file(file.path()).unwrap();
                        }
                    }
                } else {
                    std::fs::create_dir(&screenshot_directory_name).unwrap();
                }
                // take the page from world
                // add network event listener, tracking requests and pairing them with responses
                // store them somewhere inside of the world.
                let page = world.page.clone();
                let mut req_events = page
                    .event_listener::<EventRequestWillBeSent>()
                    .await
                    .unwrap();
                let mut resp_events = page
                    .event_listener::<EventResponseReceived>()
                    .await
                    .unwrap();
                world.page.enable_log().await.unwrap();
                // get log events generated by the browser
                let mut log_events = page.event_listener::<EventEntryAdded>().await.unwrap();
                // get log events generated by leptos or other console.log() calls..
                let mut runtime_events = page
                    .event_listener::<EventConsoleApiCalled>()
                    .await
                    .unwrap();
                let console_logs = world.console_logs.clone();
                let console_logs_2 = world.console_logs.clone();

                tokio::task::spawn(async move {
                    while let Some(event) = log_events.next().await {
                        if let Some(EventEntryAdded { entry }) = 
                        Arc::<EventEntryAdded>::into_inner(event) {
                            console_logs.write().await.push(format!(" {entry:#?} "));
                        } else {
                            tracing::error!("tried to into inner but none")
                        }
                    }
                });

                tokio::task::spawn(async move {
                    while let Some(event) = runtime_events.next().await {
                        if let Some(event) =Arc::<EventConsoleApiCalled>::into_inner(event) {
                            console_logs_2
                            .write()
                            .await
                            .push(format!(" CONSOLE_LOG: {:#?}", event.args));
                        } else {
                            tracing::error!("tried to into inner but none")
                        }
                       
                    }
                });

                let (tx, mut rx) = futures::channel::mpsc::channel::<Option<CookieEnum>>(1000);
                let mut tx_c = tx.clone();
                let mut tx_c_2 = tx.clone();

                world.cookie_sender = Some(tx);
                let req_resp = world.req_resp.clone();
                // Ideally you'd send the message for the Page to get the cookies from inside of the event stream loop,
                // but for some reason that doesn't always work (but sometimes it does),
                // but putting it in it's own thread makes it always work. Not sure why at the moment... ,
                // something about async, about senders, about trying to close the browser but keeping senders around.
                // we need to close the loop and drop the task to close the browser (I think)...
                tokio::task::spawn(async move {
                    while let Some(some_request_id) = rx.next().await {
                        if let Some(cookie_enum) = some_request_id {
                            match cookie_enum {
                                CookieEnum::BeforeReq(req_id) => {
                                    let cookies = page
                                        .get_cookies()
                                        .await
                                        .unwrap_or_default()
                                        .iter()
                                        .map(|cookie| {
                                            format!("name={}\n value={}", cookie.name, cookie.value)
                                        })
                                        .collect::<Vec<String>>()
                                        .join("\n");
                                    if let Some(thing) = req_resp
                                    .write()
                                    .await
                                    .get_mut(&req_id) {
                                        thing.cookies_before_request = cookies;

                                    }
                                    
                                }
                                CookieEnum::AfterResp(req_id) => {
                                    let cookies = page
                                        .get_cookies()
                                        .await
                                        .unwrap_or_default()
                                        .iter()
                                        .map(|cookie| {
                                            format!("name={}\n value={}", cookie.name, cookie.value)
                                        })
                                        .collect::<Vec<String>>()
                                        .join("\n");
                                    if let Some(thing) = req_resp
                                    .write()
                                    .await
                                    .get_mut(&req_id) {
                                        thing.cookies_after_response = cookies;
                                    }
                                   }
                            }
                        } else {
                            break;
                        }
                    }
                });

                let req_resp = world.req_resp.clone();
                tokio::task::spawn(async move {
                    while let Some(event) = req_events.next().await {
                        if let Some(event) = Arc::<EventRequestWillBeSent>::into_inner(event) {
                            if event.request.url.contains("/pkg/") {
                                continue;
                            }
                            let req_id = event.request_id.inner().clone();
                            req_resp.write().await.insert(
                                req_id.clone(),
                                RequestPair {
                                    req: Some(event.request),
                                    redirect_resp: event.redirect_response,
                                    resp: None,
                                    cookies_before_request: "".to_string(),
                                    cookies_after_response: "".to_string(),
                                    ts: std::time::Instant::now(),
                                },
                            );
                            if let Err(msg) = tx_c.try_send(Some(CookieEnum::BeforeReq(req_id.clone()))) {
                                tracing::error!(" oopsies on the {msg:#?}");
                            }
                        } else {
                            tracing::error!("into inner err")
                        }
                    }
                });

                let req_resp = world.req_resp.clone();
                tokio::task::spawn(async move {
                    while let Some(event) = resp_events.next().await {
                        if let Some(event) = Arc::<EventResponseReceived>::into_inner(event){
                            if event.response.url.contains("/pkg/") {
                                continue;
                            }
                            let req_id = event.request_id.inner().clone();
                            if let Err(msg) = tx_c_2
                                .try_send(Some(CookieEnum::AfterResp(req_id.clone()))) {
                                tracing::error!("err sending {msg:#?}");
                            }
                            if let Some(request_pair) = req_resp.write().await.get_mut(&req_id) {
                                request_pair.resp = Some(event.response);
                            } else {
                                req_resp.write().await.insert(
                                    req_id.clone(),
                                    RequestPair {
                                        req: None,
                                        redirect_resp: None,
                                        resp: Some(event.response),
                                        cookies_before_request: "No cookie?".to_string(),
                                        cookies_after_response: "No cookie?".to_string(),
                                        ts: std::time::Instant::now(),
                                    },
                                );
                            }
                        } else {
                            tracing::error!(" uhh err here")
                        }
                     
                        
                    }
                });
                // We don't need to join on our join handles, they will run detached and clean up whenever.
            })
        })
        .after(|_feature, _rule, scenario, ev, world| {
            Box::pin(async move {
                let screenshot_directory_name = format!("./screenshots/{}", scenario.name);

                let world = world.unwrap();
                // screenshot the last step
                if let Ok(screenshot) = world
                .page
                .screenshot(
                    ScreenshotParams::builder()
                        .capture_beyond_viewport(true)
                        .full_page(true)
                        .build(),
                )
                .await {
                    world.screenshots.push(screenshot);
                }

                if let cucumber::event::ScenarioFinished::StepFailed(_, _, _) = ev {
                    // close the cookie task.
                    if world
                        .cookie_sender
                        .as_mut()
                        .unwrap()
                        .try_send(None).is_err() {
                            tracing::error!("can't close cookie sender");
                        }
                    // print any applicable screenshots (just the last one of the failed step if there was none taken during the scenario)
                    for (i, screenshot) in world.screenshots.iter().enumerate() {
                        // i.e ./screenshots/login/1.png
                        _ =std::fs::write(
                            screenshot_directory_name.clone()
                                + "/"
                                + i.to_string().as_str()
                                + ".png",
                            screenshot,
                        );
                    }
                    // print network
                    let mut network_output = world
                        .req_resp
                        .read()
                        .await
                        .values()
                        .map(|val| val.clone())
                        .collect::<Vec<RequestPair>>();

                    network_output.sort_by(|a, b| a.ts.cmp(&b.ts));

                    let network_output = network_output
                        .into_iter()
                        .map(|val| val.to_string())
                        .collect::<Vec<String>>()
                        .join("\n");

                    _ = std::fs::write("./network_output", network_output.as_bytes());

                    let console_logs = world.console_logs.read().await.join("\n");

                    _ =std::fs::write("./console_logs", console_logs.as_bytes());

                    // print html
                    if let Ok(html) = world.page.content().await {
                        _ = std::fs::write("./html", html.as_bytes());
                    }
                }
                if let Err(err) = world.browser.close().await {
                    tracing::error!("{err:#?}");
                }
                if let Err(err) =  world.browser.wait().await {
                    tracing::error!("{err:#?}");
                }
            })
        })
        .run_and_exit("./features")
        .await;
    Ok(())
}

#[tracing::instrument]
async fn build_browser() -> Result<Browser, Box<dyn std::error::Error>> {
    let (browser, mut handler) = Browser::launch(
        BrowserConfig::builder()
            //.enable_request_intercept()
            .disable_cache()
            .request_timeout(Duration::from_secs(1))
            //.with_head()
            //.arg("--remote-debugging-port=9222")
            .build()?,
    )
    .await?;

    tokio::task::spawn(async move {
        while let Some(h) = handler.next().await {
            if h.is_err() {
                tracing::info!("{h:?}");
                break;
            }
        }
    });

    Ok(browser)
}

pub const HOST: &str = "https://127.0.0.1:3000";

#[derive(World)]
#[world(init = Self::new)]
pub struct AppWorld {
    pub browser: Browser,
    pub page: Page,
    pub req_resp: Arc<RwLock<HashMap<String, RequestPair>>>,
    pub clipboard: HashMap<&'static str, String>,
    pub cookie_sender: Option<Sender<Option<CookieEnum>>>,
    pub screenshots: Vec<Vec<u8>>,
    pub console_logs: Arc<RwLock<Vec<String>>>,
}

impl std::fmt::Debug for AppWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppWorld").finish()
    }
}

impl AppWorld {
    async fn new() -> Result<Self, anyhow::Error> {
        let browser = build_browser().await.unwrap();

        let page = browser.new_page("about:blank").await?;

        Ok(Self {
            browser,
            page,
            req_resp: Arc::new(RwLock::new(HashMap::new())),
            clipboard: HashMap::new(),
            cookie_sender: None,
            screenshots: Vec::new(),
            console_logs: Arc::new(RwLock::new(Vec::new())),
        })
    }

    pub async fn errors(&mut self) -> Result<()> {
        if let Ok(error) = self.find(ids::ERROR_ERROR_ID).await {
            Err(anyhow!("{}", error.inner_text().await?.unwrap_or(String::from("no error in inner template?"))))
        } else {
            Ok(())
        }
    }

    pub async fn find(&self, id: &'static str) -> Result<Element> {
        for _ in 0..4 {
            if let Ok(el) = self.page.find_element(format!("#{id}")).await {
                return Ok(el);
            }
            crate::fixtures::wait().await;
        }
        Err(anyhow!("Can't find {id}"))
    }

    pub async fn find_submit(&mut self) -> Result<Element> {
        for _ in 0..4 {
            if let Ok(el) = self.page.find_element(format!("input[type=submit]")).await {
                return Ok(el);
            }
            crate::fixtures::wait().await;
        }
        Err(anyhow!("Can't find input type=submit"))
    }

    /*pub async fn find_all(&mut self, id: &'static str) -> Result<ElementList> {
        Ok(ElementList(
            self.page.find_elements(format!("#{id}")).await?,
        ))
    }*/

    pub async fn goto_url(&mut self, url: &str) -> Result<()> {
        self.page
            .goto(
                NavigateParams::builder()
                    .url(url)
                    .build()
                    .map_err(|err| anyhow!(err))?,
            )
            .await?
            .wait_for_navigation()
            .await?;
        self.screenshot().await?;
        Ok(())
    }

    pub async fn goto_path(&mut self, path: &str) -> Result<()> {
        let url = format!("{}{}", HOST, path);
        self.page
            .goto(
                NavigateParams::builder()
                    .url(url)
                    .build()
                    .map_err(|err| anyhow!(err))?,
            )
            .await?;
        self.screenshot().await?;
        Ok(())
    }
    pub async fn screenshot(&mut self) -> Result<()> {
        let sc = self.page.screenshot(ScreenshotParams::default()).await?;
        self.screenshots.push(sc);
        Ok(())
    }
    pub async fn set_field<S: AsRef<str> + std::fmt::Display>(
        &mut self,
        id: &'static str,
        value: S,
    ) -> Result<()> {
        let element = self.find(id).await?;
        element.focus().await?.type_str(value).await?;
        self.screenshot().await?;
        Ok(())
    }

    pub async fn click(&mut self, id: &'static str) -> Result<()> {
        self.find(id).await?.click().await?;
        Ok(())
    }
    #[tracing::instrument(err)]
    pub async fn submit(&mut self) -> Result<()> {
        self.screenshot().await?;
        self.find_submit().await?.click().await?;
        Ok(())
    }
    pub async fn find_text(&self, text: String) -> Result<Element> {
        let selector: String = format!("//*[contains(text(), '{text}') or @*='{text}']");
        let mut count = 0;
        loop {
            let result = self.page.find_xpath(&selector).await;
            if result.is_err() && count < 4 {
                count += 1;
                crate::fixtures::wait().await;
            } else {
                let result = result?;
                return Ok(result);
            }
        }
    }
    pub async fn url_contains(&self, s: &'static str) -> Result<()> {
        if let Some(current) = self.page.url().await? {
            if !current.contains(s) {
                return Err(anyhow!("{current} does not contains {s}"));
            }
        } else {
            return Err(anyhow!("NO CURRENT URL FOUND"));
        }
        Ok(())
    }
    pub async fn verify_route(&self, path: &'static str) -> Result<()> {
        let url = format!("{}{}", HOST, path);
        if let Some(current) = self.page.url().await? {
            if current != url {
                return Err(anyhow!(
                    "EXPECTING ROUTE: {path}\n but FOUND:\n {current:#?}"
                ));
            }
        } else {
            return Err(anyhow!(
                "EXPECTING ROUTE: {path}\n but NO CURRENT URL FOUND"
            ));
        }
        Ok(())
    }
}

/*
#[derive(Debug)]
pub struct ElementList(Vec<Element>);
impl ElementList {
    /// iterates over elements, finds first element whose text (as rendered) contains text given as function's argument.
    pub async fn find_by_text(&self,text:&'static str) -> Result<Element> {
        for element in self.0.iter() {
            if let Ok(Some(inner_text)) = element.inner_text().await {
                if inner_text.contains(text) {
                    return Ok(element);
                }
            }
        }
        Err(anyhow!(format!("given text {} no element found",text)))
    }

}*/

#[derive(Serialize, Deserialize, Debug)]
struct Email {
    id: Uuid,
    to: Vec<Recipient>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Recipient {
    name: Option<String>,
    email: String,
}
