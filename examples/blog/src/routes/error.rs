use leptos::*;
use leptos_router::*;

#[component]
#[allow(non_snake_case)]
pub fn PageNotFound(_cx: Scope) -> Element {
    view! {
        _cx,
        <div>
            <div class="data">
                <p>"hey it&apos;s 404"</p>
                <br/>
                <br/>

                <nav class="nav">
                    <div class="nav-text">
                       <A exact=true href="/"><p>"# home"</p></A>
                       <A exact=true href="/blog"><p>"# blog"</p></A>
                       <A href="/about"><p>"# more about me"</p></A>
                    </div>
                </nav>
            </div>
        </div>
    }
}
