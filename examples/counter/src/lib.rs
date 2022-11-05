use leptos::*;

pub fn simple_counter(cx: Scope) -> web_sys::Element {
    let (value, set_value) = create_signal(cx, 0);

    view! { cx,
        <div>
            <MyComponent><p>"Here's the child"</p></MyComponent>
            <button on:click=move |_| set_value(0)>"Clear"</button>
            <button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
            <span>"Value: " {move || value().to_string()} "!"</span>
            <button on:click=move |_| set_value.update(|value| *value += 1)>"+1"</button>
        </div>
    }
}

#[component]
fn MyComponent(cx: Scope, children: Option<Box<dyn Fn() -> Vec<Element>>>) -> Element {
    view! {
        cx,
       <my-component>
          <p>"Here's the child you passed in: "</p>
          <slot></slot>
       </my-component>
    }
}
