//! SSR rendering behavior of `ErrorBoundary`.
//!
//! These guard the synchronous `to_html` path, where children are rendered
//! directly into the output buffer and rolled back (via a length marker +
//! `String::truncate`) when a child throws, so the fallback can be emitted in
//! their place.
#![cfg(feature = "ssr")]

use std::{error::Error, fmt};

#[derive(Debug)]
struct MyErr;

impl fmt::Display for MyErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "boom")
    }
}

impl Error for MyErr {}

#[test]
fn error_boundary_renders_children_when_no_error() {
    use leptos::prelude::*;

    let rendered = view! {
        <ErrorBoundary fallback=|_| view! { <p>"FALLBACK"</p> }>
            <ul>{(0..5).map(|i| view! { <li>{i}</li> }).collect_view()}</ul>
        </ErrorBoundary>
    }
    .to_html();

    assert_eq!(
        rendered,
        "<ul><li>0</li><li>1</li><li>2</li><li>3</li><li>4</li><!></ul>"
    );
    assert!(!rendered.contains("FALLBACK"));
}

#[test]
fn error_boundary_rolls_back_children_and_renders_fallback_on_error() {
    use leptos::prelude::*;

    // The `<ul><li>before</li>` bytes are written into the buffer before the
    // throwing child is reached; the boundary must discard them entirely and
    // emit only the fallback.
    let rendered = view! {
        <ErrorBoundary fallback=|_| view! { <p>"FALLBACK"</p> }>
            <ul>
                <li>"before"</li>
                {move || Err::<i32, MyErr>(MyErr)}
            </ul>
        </ErrorBoundary>
    }
    .to_html();

    assert_eq!(rendered, "<p>FALLBACK</p>");
    assert!(!rendered.contains("before"));
}
