use crate::error_template::{AppError, ErrorTemplate};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {


        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/openapi-swagger-ui.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! {
                <ErrorTemplate outside_errors/>
            }
            .into_view()
        }>
            <main>
                <Routes>
                    <Route path="" view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let hello = Action::<HelloWorld,_>::server();
    view! {
        <button on:click = move |_| hello.dispatch(HelloWorld{say_whut:SayHello{say:true}})>
            "hello world"
        </button>


        <ErrorBoundary
            fallback=|err| view! { <p>{format!("{err:#?}")}</p>}>
            {
                move || hello.value().get().map(|h|match h {
                    Ok(h) => h.into_view(),
                    err => err.into_view()
                })
            }
        </ErrorBoundary>

        <AiSayHello/>
    }
}

#[cfg_attr(feature="ssr",derive(utoipa::ToSchema))]
#[derive(Debug,Copy,Clone,serde::Serialize,serde::Deserialize)]
pub struct SayHello {
   say:bool,
}

// the following function comment is what our GPT will get
/// Call to say hello world, or call to not say hello world.
#[cfg_attr(feature="ssr",utoipa::path(
    post,
    path = "/api/hello_world",
    responses(
        (status = 200, description = "Hello world from server or maybe not?", body = String),
    ),
    params(
        ("say_whut" = SayHello, description = "If true then say hello, if false then don't."),
    )
))]
#[server(
    // we need to encoude our server functions as json because that's what openai generates
    input=server_fn::codec::Json,
    endpoint="hello_world"
)]
pub async fn hello_world(say_whut:SayHello) -> Result<String,ServerFnError> {
    if say_whut.say {
        Ok("hello world".to_string())
    } else {
        Ok("not hello".to_string())
    }
}


/// Takes a list of names
#[cfg_attr(feature="ssr",utoipa::path(
    post,
    path = "/api/name_list",
    responses(
        (status = 200, description = "The same list you got back", body = String),
    ),
    params(
        ("list" = Vec<String>, description = "A list of names"),
    )
))]
#[server(
    input=server_fn::codec::Json,
    endpoint="name_list"
)]
pub async fn name_list(list:Vec<String>) -> Result<Vec<String>,ServerFnError> {
    Ok(list)
}



#[derive(Clone,Debug,PartialEq,serde::Serialize,serde::Deserialize)]
pub struct AiServerCall{
    pub path:String,
    pub args:String,
}


// Don't include our AI function in the OpenAPI
#[server]
pub async fn ai_msg(msg:String) -> Result<AiServerCall,ServerFnError> {
    crate::open_ai::call_gpt_with_api(msg).await.get(0).cloned().ok_or(ServerFnError::new("No first message"))
}

#[component]
pub fn AiSayHello() -> impl IntoView {
    let ai_msg = Action::<AiMsg, _>::server();
    let result = create_rw_signal(Vec::new());
    view!{
        <ActionForm action=ai_msg>
        <label> "Tell the AI what function to call."
        <input name="msg"/>
        </label>
        <input type="submit"/>
        </ActionForm>
        <div>
        {
            move || if let Some(Ok(AiServerCall{path,args})) = ai_msg.value().get() {
                spawn_local(async move {
                    let text = 
                    reqwest::Client::new()
                    .post(format!("http://127.0.0.1:3000/api/{}",path))
                    .header("content-type","application/json")
                    .body(args)
                    .send()
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap();
                    result.update(|list|
                            list.push(
                                text
                            )
                        );
                });
            }
        }
        <For
        each=move || result.get()
        key=|_| uuid::Uuid::new_v4()
        children=move |s:String| {
          view! {
            <p>{s}</p>
          }
        }
      />
        </div>
    }
}