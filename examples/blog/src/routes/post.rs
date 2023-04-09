use leptos::*;
use leptos_router::*;

#[component]
#[allow(non_snake_case)]
pub fn Post(cx: Scope) -> Element {
    let params = use_params_map(cx);
    let id = params().get("id").cloned().unwrap_or_default();

    log!("{id}");

    // TODO:
    // 1. markdown to html
    // 2. get id and link it to the post

    view! {
        cx,
        <div>
            <div class="data">
                <div class="intro">
                    <p>"hi there"</p>
                    <br/>

                </div>

                <nav class="nav">
                    <div class="nav-text">
                       <A exact=true href="/blog"><p>"# blog"</p></A>
                       <A href="/about"><p>"# more about me"</p></A>
                    </div>
                </nav>
            </div>
        </div>
    }
}
