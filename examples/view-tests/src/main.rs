use leptos::*;

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(|cx| view! { cx, <TemplateConsumer/> })
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
            <h1>"Template Consumer"</h1>
            {cloned_tpl}
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
