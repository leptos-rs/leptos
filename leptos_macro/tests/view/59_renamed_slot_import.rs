// A `#[slot]` and its parent `#[component]` imported under renamed aliases.
// The view macro should resolve the slot through the struct's inherent methods,
// so renamed imports work correctly.
//
// Uses `slot:my_slot` syntax to specify the original prop name on the parent,
// since the tag name `RenamedSlot` converts to `renamed_slot` which doesn't
// match the parent's `my_slot` prop.

use leptos::prelude::*;

mod inner {
    use leptos::prelude::*;

    #[slot]
    pub struct MySlot {
        #[prop(into)]
        label: String,
    }

    #[component]
    pub fn Parent(my_slot: MySlot) -> impl IntoView {
        my_slot.label
    }
}

use inner::MySlot as RenamedSlot;
use inner::Parent as RenamedParent;

#[component]
fn App() -> impl IntoView {
    view! {
        <RenamedParent>
            <RenamedSlot slot:my_slot label="hello" />
        </RenamedParent>
    }
}

fn main() {}
