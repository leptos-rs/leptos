use leptos::*;
use leptos_router::*;

#[component]
#[allow(non_snake_case)]
pub fn Home(_cx: Scope) -> Element {
    //log!("rendering homepage");

    view! {
        _cx,
        <div>
            <div class="data">
                <div class="intro">
                    <p>"hi there"</p>
                    <br/>

                    <p>"i'm ted"</p>
                    <p>"i do code. using js for work and working with rust for fun"</p>
                    <p>"sometimes i also do write, about tech and thoughts, here"</p>
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
