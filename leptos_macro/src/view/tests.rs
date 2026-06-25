//! Locality tests for `view!` diagnostics.
//!
//! These assert two things the previous `emit_error!`/`abort!` approach had no
//! way to guarantee or check:
//!
//! * **Locality** – a diagnostic underlines the specific offending token, not
//!   the whole `view!` invocation (which is what `Span::call_site()` does).
//! * **Bounded output** – duplicate diagnostics collapse and the total is
//!   capped (see [`MAX_DIAGNOSTICS`]).
//!
//! The span→source mapping relies on `proc-macro2`'s `span-locations` feature,
//! enabled for the test build in `Cargo.toml`.

use super::diagnostics::{self, Diagnostic, MAX_DIAGNOSTICS};
use proc_macro2::{Span, TokenStream};
use std::str::FromStr;

/// Lower `src` exactly the way the `view!` macro does and return the
/// diagnostics it records, without emitting them.
fn diagnostics_for(src: &str) -> Vec<Diagnostic> {
    let tokens = TokenStream::from_str(src).expect("test input must tokenize");
    let config = rstml::ParserConfig::default().recover_block(true);
    let parser = rstml::Parser::new(config);
    let (mut nodes, _parse_errors) =
        parser.parse_recoverable(tokens).split_vec();

    let (_output, diagnostics) = diagnostics::collect(|| {
        super::render_view(&mut nodes, None, None, false)
    });
    diagnostics
}

/// Assert that `span` starts exactly where `needle` begins in `src` (first
/// occurrence), i.e. the diagnostic is anchored at the offending token rather
/// than at the macro call site (which would be line 1, column 0).
fn assert_anchored_at(src: &str, span: Span, needle: &str) {
    let expected_col = src
        .find(needle)
        .unwrap_or_else(|| panic!("`{needle}` not in source"));
    let start = span.start();
    assert_eq!(start.line, 1, "diagnostic should be on the first line");
    assert_eq!(
        start.column, expected_col,
        "diagnostic should be anchored at `{needle}` (column {expected_col}), \
         but started at column {}",
        start.column
    );
}

/// Recursively find the span of the first `Ident` named `name` in `tokens`.
fn find_ident_span(
    tokens: &TokenStream,
    name: &str,
) -> Option<proc_macro2::Span> {
    for tt in tokens.clone() {
        match tt {
            proc_macro2::TokenTree::Ident(id) if id == name => {
                return Some(id.span());
            }
            proc_macro2::TokenTree::Group(g) => {
                if let Some(span) = find_ident_span(&g.stream(), name) {
                    return Some(span);
                }
            }
            _ => {}
        }
    }
    None
}

#[test]
fn missing_required_prop_error_points_at_component() {
    // `<Greeting excitement=4/>` is missing the required `name` prop. The
    // TypedBuilder error is raised on `.build()`, so `build` must be spanned to
    // the `Greeting` tag, not to the whole `view!` (which is what a default
    // `quote!` call-site span would do).
    let src = "<Greeting excitement=4/>";
    let tokens = TokenStream::from_str(src).unwrap();
    let (mut nodes, _) = super::parse_nodes(tokens);
    let output = super::render_view(&mut nodes, None, None, false)
        .expect("component should lower");

    let build_span = find_ident_span(&output, "build")
        .expect("`.build()` should be emitted");
    let expected_col = src.find("Greeting").unwrap();
    assert_eq!(build_span.start().line, 1);
    assert_eq!(
        build_span.start().column,
        expected_col,
        "`.build()` should be anchored at the `Greeting` tag, not the macro"
    );
}

#[test]
fn event_modifier_error_is_local_and_recovers() {
    let src = "<button on:click:bad=foo>\"x\"</button>";
    let diagnostics = diagnostics_for(src);

    // Exactly one diagnostic: lowering recovered instead of aborting, and the
    // single mistake produced a single error.
    assert_eq!(diagnostics.len(), 1, "expected exactly one diagnostic");
    let diagnostic = &diagnostics[0];
    assert!(
        diagnostic.message.contains("unknown event modifier"),
        "unexpected message: {}",
        diagnostic.message
    );
    // The error points at the event attribute, not the whole `view!`.
    assert_anchored_at(src, diagnostic.span, "on:click:bad");
}

#[test]
fn duplicate_attribute_error_is_local() {
    let src = r#"<div id="a" id="b">"x"</div>"#;
    let diagnostics = diagnostics_for(src);

    assert_eq!(diagnostics.len(), 1, "expected exactly one diagnostic");
    let diagnostic = &diagnostics[0];
    assert!(
        diagnostic.message.contains("already has a `id` attribute"),
        "unexpected message: {}",
        diagnostic.message
    );
    // Anchored at the *second* `id`, where the conflict is.
    let second_id = src.rfind("id").unwrap();
    assert_eq!(
        diagnostic.span.start().column,
        second_id,
        "duplicate-attribute error should underline the second `id`"
    );
}

/// A half-typed component (`<Greeting ` with no closing `/>`) normally parses
/// to nothing, so the editor has no props builder to complete against. The
/// debug-only IDE recovery in `parse_nodes` should surface the builder chain
/// anyway. (Only meaningful in debug builds, where the recovery is compiled.)
#[cfg(debug_assertions)]
#[test]
fn incomplete_component_tag_recovers_props_builder() {
    let incomplete = TokenStream::from_str("<Greeting ").unwrap();
    let (mut nodes, _errors) = super::parse_nodes(incomplete);
    assert!(
        !nodes.is_empty(),
        "incomplete tag should be recovered into a node"
    );

    let output = super::render_view(&mut nodes, None, None, false)
        .expect("recovered nodes should lower")
        .to_string();
    assert!(
        output.contains("component_props_builder"),
        "recovered output should expose the props builder, got: {output}"
    );
}

/// The recovery must also fire when the half-typed tag is *not* the only thing
/// in the view (e.g. a second component being added below an existing one).
#[cfg(debug_assertions)]
#[test]
fn incomplete_second_component_recovers() {
    let incomplete =
        TokenStream::from_str("<Greeting name=\"x\"/> <Greeting ").unwrap();
    let (nodes, _errors) = super::parse_nodes(incomplete);
    assert_eq!(
        nodes.len(),
        2,
        "both the complete and the half-typed component should be present"
    );
}

/// A complete view must never trigger the recovery pass (it parses with zero
/// errors), so its output is identical regardless of the debug-only path. This
/// guards against the recovery ever touching real code.
#[test]
fn complete_view_is_untouched_by_recovery() {
    let complete =
        TokenStream::from_str("<Greeting name=\"x\"/><Greeting name=\"y\"/>")
            .unwrap();
    let (nodes, _errors) = super::parse_nodes(complete);
    assert_eq!(nodes.len(), 2, "a complete view parses to all its nodes");
}

#[test]
fn duplicate_diagnostics_are_collapsed() {
    let (_, diagnostics) = diagnostics::collect(|| {
        let span = Span::call_site();
        diagnostics::error(span, "same message");
        diagnostics::error(span, "same message"); // exact duplicate
        diagnostics::error(span, "different message");
    });

    assert_eq!(
        diagnostics.len(),
        2,
        "identical (span, message) diagnostics should collapse to one"
    );
}

#[test]
fn diagnostics_are_capped() {
    let overflow = 5;
    let (_, diagnostics) = diagnostics::collect(|| {
        for i in 0..(MAX_DIAGNOSTICS + overflow) {
            // Distinct messages so de-dup doesn't interfere with the cap.
            diagnostics::error(Span::call_site(), format!("error {i}"));
        }
    });

    // `MAX_DIAGNOSTICS` real errors plus one summary.
    assert_eq!(diagnostics.len(), MAX_DIAGNOSTICS + 1);
    assert!(
        diagnostics.last().unwrap().message.contains("suppressed"),
        "the final diagnostic should summarize the suppressed ones"
    );
}
