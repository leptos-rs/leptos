//! Internal code powering the `leptos::view!` macro, and all other leptos macros

#![cfg_attr(all(feature = "nightly", rustc_nightly), feature(proc_macro_span))]
#![forbid(unsafe_code)]
// to prevent warnings from popping up when a nightly feature is stabilized
#![allow(stable_features)]
// FIXME? every use of quote! {} is warning here -- false positive?
#![allow(unknown_lints)]
#![allow(private_macro_use)]
#![deny(missing_docs)]

#[macro_use]
extern crate proc_macro_error2;

use component::DummyModel;
use proc_macro2::{Span, TokenTree};
use quote::{quote, ToTokens};
use std::str::FromStr;
use syn::{parse_macro_input, spanned::Spanned, token::Pub, Visibility};

mod params;
mod view;
use crate::component::unmodified_fn_name_from_fn_name;
mod component;
mod lazy;
mod memo;
mod slice;
mod slot;

fn handle_global_class(
    tokens: proc_macro2::TokenStream,
) -> (proc_macro2::TokenStream, Option<proc_macro2::TokenTree>) {
    let mut tokens = tokens.into_iter();

    let first = tokens.next();
    let second = tokens.next();
    let third = tokens.next();
    let fourth = tokens.next();
    let global_class = match (&first, &second) {
        (Some(TokenTree::Ident(first)), Some(TokenTree::Punct(eq)))
            if *first == "class" && eq.as_char() == '=' =>
        {
            match &fourth {
                Some(TokenTree::Punct(comma)) if comma.as_char() == ',' => {
                    third.clone()
                }
                _ => {
                    abort!(
                        second, "To create a scope class with the view! macro you must put a comma `,` after the value";
                        help = r#"e.g., view!{ class="my-class", <div>...</div>}"#
                    )
                }
            }
        }
        _ => None,
    };
    let tokens = if global_class.is_some() {
        tokens.collect::<proc_macro2::TokenStream>()
    } else {
        [first, second, third, fourth]
            .into_iter()
            .flatten()
            .chain(tokens)
            .collect()
    };
    (tokens, global_class)
}

/// The actual implementation of the [`leptos::view!`](https://docs.rs/leptos/0.8.0-rc3/leptos/macro.view.html) macro
pub fn view_macro_impl(
    tokens: proc_macro2::TokenStream,
    template: bool,
) -> proc_macro2::TokenStream {
    let tokens: proc_macro2::TokenStream = tokens.into();
    let (tokens, global_class) = handle_global_class(tokens);

    let config = rstml::ParserConfig::default().recover_block(true);
    let parser = rstml::Parser::new(config);
    let (mut nodes, errors) = parser.parse_recoverable(tokens).split_vec();
    let errors = errors.into_iter().map(|e| e.emit_as_expr_tokens());
    let nodes_output = view::render_view(
        &mut nodes,
        global_class.as_ref(),
        normalized_call_site(proc_macro2::Span::call_site()),
        template,
    );

    // The allow lint needs to be put here instead of at the expansion of
    // view::attribute_value(). Adding this next to the expanded expression
    // seems to break rust-analyzer, but it works when the allow is put here.
    let output = quote! {
        {
            #[allow(unused_braces)]
            {
                #(#errors;)*
                #nodes_output
            }
        }
    };

    if template {
        quote! {
            ::leptos::prelude::ViewTemplate::new(#output)
        }
    } else {
        output
    }
    .into()
}

fn normalized_call_site(site: proc_macro2::Span) -> Option<String> {
    cfg_if::cfg_if! {
        if #[cfg(all(debug_assertions, feature = "nightly", rustc_nightly))] {
            Some(leptos_hot_reload::span_to_stable_id(
                site.file(),
                site.start().line()
            ))
        } else {
            _ = site;
            None
        }
    }
}

pub fn include_view_impl(tokens: proc_macro2::TokenStream)