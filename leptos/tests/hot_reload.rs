use leptos::prelude::*;

#[tokio::test]
async fn resolve_keeps_view_marker() {
    let view = View::new("Ready".to_string()).with_view_marker("child");
    assert_eq!(
        view.resolve().await,
        View::new("Ready".to_string()).with_view_marker("child")
    );
}
