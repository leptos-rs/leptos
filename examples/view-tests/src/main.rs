use leptos::*;

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(|cx| view! { cx, <Tests/> })
}

#[component]
fn SelfUpdatingEffect(cx: Scope) -> Element {
    let (a, set_a) = create_signal(cx, false);

    create_effect(cx, move |_| {
        if !a() {
            set_a(true);
        }
    });

    view! { cx,
      <h1>"Hello " {move || a().to_string()}</h1>
    }
}

#[component]
fn Tests(cx: Scope) -> Element {
    view! {
        cx,
        <div>
            //<div><SelfUpdatingEffect/></div>
            <div><BlockOrders/></div>
            //<div><TemplateConsumer/></div>
        </div>
    }
}

#[component]
fn BlockOrders(cx: Scope) -> Element {
    let a = "A";
    let b = "B";
    let c = "C";

    view! {
        cx,
        <div>
            <div>"A"</div>
            <div>{a}</div>
            <div><span>"A"</span></div>
            <div><span>{a}</span></div>
            <hr/>
            <div>"A" {b}</div>
            <div>{a} "B"</div>
            <div>{a} {b}</div>
            <div>{"A"} {"B"}</div>
            <div><span style="color: red">{a}</span> {b}</div>
            <hr/>
            <div>{a} "B" {c}</div>
            <div>"A" {b} "C"</div>
            <div>{a} {b} "C"</div>
            <div>{a} {b} {c}</div>
            <div>"A" {b} {c}</div>
            <hr/>
            <div>"A" {b} <span style="color: red">"C"</span></div>
            <div>"A" {b} <span style="color: red">{c}</span></div>
            <div>"A" <span style="color: red">"B"</span> "C"</div>
            <div>"A" <span style="color: red">"B"</span> {c}</div>
            <div>{a} <span style="color: red">{b}</span> {c}</div>
            <div>"A" {b} <span style="color: red">{c}</span></div>
            <div><span style="color: red">"A"</span> {b} {c}</div>
            <div><span style="color: red">{a}</span> "B" {c}</div>
            <div><span style="color: red">"A"</span> {b} "C"</div>
            <hr/>
            <div><span style="color: red">"A"</span> <span style="color: blue">{b}</span> {c}</div>
            <div><span style="color: red">{a}</span> "B" <span style="color: blue">{c}</span></div>
            <div><span style="color: red">"A"</span> {b} <span style="color: blue">"C"</span></div>
            <hr/>
            <div><A/></div>
            <div>"A" <B/></div>
            <div>{a} <B/></div>
            <div><A/> "B"</div>
            <div><A/> {b}</div>
            <div><A/><B/></div>
            <hr/>
            <div><A/> "B" <C/></div>
            <div><A/> {b} <C/></div>
            <div><A/> {b} "C"</div>
        </div>
    }
}

#[component]
fn A(cx: Scope) -> Element {
    view! { cx, <span style="color: red">"A"</span> }
}

#[component]
fn B(cx: Scope) -> Element {
    view! { cx, <span style="color: red">"B"</span> }
}

#[component]
fn C(cx: Scope) -> Element {
    view! { cx, <span style="color: red">"C"</span> }
}

#[component]
fn TemplateConsumer(cx: Scope) -> Element {
    let tpl = view! { cx, <TemplateExample/> };
    let cloned_tpl = tpl
        .unchecked_ref::<web_sys::HtmlTemplateElement>()
        .content()
        .clone_node_with_deep(true)
        .expect("couldn't clone template node");

    view! {
        cx,
        <div id="template">
            /* <h1>"Template Consumer"</h1>
            {cloned_tpl} */
        </div>
    }
}

#[component]
fn TemplateExample(cx: Scope) -> Element {
    view! {
        cx,
        <template>
            <div>"Template contents"</div>
        </template>
    }
}
