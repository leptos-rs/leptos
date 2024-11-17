#[test]
fn generic_component_signal_inference() {
    use leptos::prelude::*;

    #[component]
    pub fn SimpleCounter(#[prop(into)] step: Signal<i32>) -> impl IntoView {
        _ = step;
        view! {
            <div>
            </div>
        }
    }

    let a = RwSignal::new(1);
    let (b, _) = signal(1);

    view! {
        <SimpleCounter step=a/>
        <SimpleCounter step=b/>
        <SimpleCounter step=Signal::stored(1)/>
    };
}
