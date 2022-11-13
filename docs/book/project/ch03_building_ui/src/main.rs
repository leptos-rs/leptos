use leptos::*;

fn main() {
    mount_to_body(|cx| {
        let name = "gbj";
        let userid = 0;
        let _input_element: Element;

        view! {
            cx,
            <main>
                <h1>"My Tasks"</h1>     // text nodes are wrapped in quotation marks
                <h2>"by " {name}</h2>
                <input
                    type="text"         // attributes work just like they do in HTML
                    name="new-todo"
                    prop:value="todo"   // `prop:` lets you set a property on a DOM node
                    value="initial"     // side note: the DOM `value` attribute only sets *initial* value
                                        // this is very important when working with forms!
                    _ref=_input_element // `_ref` stores tis element in a variable
                />
                <ul data-user=userid>   // attributes can take expressions as values
                    <li class="todo my-todo" // here we set the `class` attribute
                        class:completed=true // `class:` also lets you toggle individual classes
                        on:click=|_| todo!() // `on:` adds an event listener
                    >
                        "Buy milk."
                    </li>
                    <li class="todo my-todo" class:completed=false>
                        "???"
                    </li>
                    <li class="todo my-todo" class:completed=false>
                        "Profit!!!"
                    </li>
                </ul>
            </main>
        }
    })
}
