use leptos::*;
use web_sys::Event;

// This highlights four different ways that child components can communicate
// with their parent:
// 1) <ButtonA/>: passing a WriteSignal as one of the child component props,
//    for the child component to write into and the parent to read
// 2) <ButtonB/>: passing a closure as one of the child component props, for
//    the child component to call
// 3) <ButtonC/>: adding a simple event listener on the child component itself
// 4) <ButtonD/>: providing a context that is used in the component (rather than prop drilling)

#[derive(Copy, Clone)]
struct SmallcapsContext(WriteSignal<bool>);

#[component]
pub fn App(cx: Scope) -> Element {
    // just some signals to toggle three classes on our <p>
    let (red, set_red) = create_signal(cx, false);
    let (right, set_right) = create_signal(cx, false);
    let (italics, set_italics) = create_signal(cx, false);
    let (smallcaps, set_smallcaps) = create_signal(cx, false);

    // the newtype pattern isn't *necessary* here but is a good practice
    // it avoids confusion with other possible future `WriteSignal<bool>` contexts
    // and makes it easier to refer to it in ButtonD
    provide_context(cx, SmallcapsContext(set_smallcaps));

    view! {
        cx,
        <main>
            <p
                // class: attributes take F: Fn() => bool, and these signals all implement Fn()
                class:red=red
                class:right=right
                class:italics=italics
                class:smallcaps=smallcaps
            >
                "Lorem ipsum sit dolor amet."
            </p>

            // Button A: pass the signal setter
            <ButtonA setter=set_red/>

            // Button B: pass a closure
            <ButtonB on_click=move |_| set_right.update(|value| *value = !*value)/>

            // Button C: components that return an Element, like elements, can take on: event handler attributes
            <ButtonC on:click=move |_| set_italics.update(|value| *value = !*value)/>

            // Button D gets its setter from context rather than props
            <ButtonD/>
        </main>
    }
}

// Button A receives a signal setter and updates the signal itself
#[component]
pub fn ButtonA(cx: Scope, setter: WriteSignal<bool>) -> Element {
    view! {
        cx,
        <button
            on:click=move |_| setter.update(|value| *value = !*value)
        >
            "Toggle Red"
        </button>
    }
}

// Button B receives a closure
#[component]
pub fn ButtonB<F>(cx: Scope, on_click: F) -> Element
where
    F: Fn(Event) + 'static,
{
    view! {
        cx,
        <button
            on:click=on_click
        >
            "Toggle Right"
        </button>
    }

    // just a note: in an ordinary function ButtonB could take on_click: impl Fn(Event) + 'static
    // and save you from typing out the generic
    // the component macro actually expands to define a
    //
    // struct ButtonBProps<F> where F: Fn(Event) + 'static {
    //   on_click: F
    // }
    //
    // this is what allows us to have named props in our component invocation,
    // instead of an ordered list of function arguments
    // if Rust ever had named function arguments we could drop this requirement
}

// Button C will have its event listener added by the parent
// This is just a way of encapsulating whatever markup you need for the button
#[component]
pub fn ButtonC(cx: Scope) -> Element {
    view! {
        cx,
        <button>
            "Toggle Italics"
        </button>
    }
}

// Button D is very similar to Button A, but instead of passing the setter as a prop
// we get it from the context
#[component]
pub fn ButtonD(cx: Scope) -> Element {
    let setter = use_context::<SmallcapsContext>(cx).unwrap().0;

    view! {
        cx,
        <button
            on:click=move |_| setter.update(|value| *value = !*value)
        >
            "Toggle Small Caps"
        </button>
    }
}
