#[test]
fn generic_component_signal_inference() {
    use leptos::prelude::*;

    #[component]
    pub fn SimpleCounter<T>(#[prop(into)] step: Signal<T>) -> impl IntoView
    where
        T: Send + Sync + 'static,
    {
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
