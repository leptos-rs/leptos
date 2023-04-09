use leptos::*;
use leptos_router::*;

#[component]
#[allow(non_snake_case)]
pub fn About(_cx: Scope) -> Element {
    view! {
        _cx,
        <div>
            <div class="data">
                <div class="intro">
                    <p>"hi there again"</p>
                    <br/>
                    <p>"people call me ted, long story"</p>
                    <p>"i currently am a fullstack developer, mostly working with nextjs, nodejs and devops, data stuffs, just like any other fullstack swe in the world. i&apos;m also using arch linux btw, coding rust in neovim blabla."</p>
                    <p>"more than that, i love watching tv series, youtube videos, twitch streams, writing things, sometimes write songs, running, swimming, badminton, chess ..."</p>
                    <br />
                    <p>"so, click if you"</p>
                    <ul>
                        <li><a href="https://www.linkedin.com/in/huy-le-vu-minh/" target="_blank">"think there is a job that suitable for me"</a></li>
                        <li><a href="https://github.com/LeVuMinhHuy" target="_blank">"wanna check out some of my projects"</a></li>
                        <li><a href="https://www.facebook.com/moreromem" target="_blank">"comfortable to drop some messages"</a></li>
                        <li><a href="/">"or just want to read"</a></li>

                    </ul>

                    <br/>
                    <p>"i&apos;m also proud to say that this site is built with zero line of js. it's all about rust and wasm, based on leptos"</p>
                </div>

                <nav class="nav">
                    <div class="nav-text">
                       <A exact=true href="/blog"><p>"# blog"</p></A>
                       <A exact=true href="/"><p>"# home"</p></A>
                    </div>
                </nav>
            </div>
        </div>
    }
}
