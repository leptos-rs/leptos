//! A macro to make path definitions easier with [`leptos_router`].
//!
//! [`leptos_router`]: https://docs.rs/leptos_router/latest/leptos_router/components/fn.Route.html

#![deny(missing_docs)]

use proc_macro::{TokenStream, TokenTree};
use proc_macro2::Span;
use proc_macro_error2::{abort, proc_macro_error};
use quote::{quote, ToTokens};
use syn::{
    spanned::Spanned, Block, Ident, ImplItem, ItemImpl, Path, Type, TypePath,
};

const RFC3986_UNRESERVED: [char; 4] = ['-', '.', '_', '~'];
const RFC3986_PCHAR_OTHER: [char; 1] = ['@'];

/// Constructs a path for use in a [`Route`] definition.
///
/// Note that this is an optional convenience. Manually defining route segments
/// is equivalent.
///
/// # Examples
///
/// ```rust
/// use leptos_router::{
///     path, OptionalParamSegment, ParamSegment, StaticSegment,
///     WildcardSegment,
/// };
///
/// let path = path!("/foo/:bar/:baz?/*any");
/// let output = (
///     StaticSegment("foo"),
///     ParamSegment("bar"),
///     OptionalParamSegment("baz"),
///     WildcardSegment("any"),
/// );
///
/// assert_eq!(path, output);
/// ```
/// [`Route`]: https://docs.rs/leptos_router/latest/leptos_router/components/fn.Route.html
#[proc_macro_error2::proc_macro_error]
#[proc_macro]
pub fn path(tokens: TokenStream) -> TokenStream {
    let mut parser = SegmentParser::new(tokens);
    parser.parse_all();
    let segments = Segments(parser.segments);
    segments.into_token_stream().into()
}

#[derive(Debug, PartialEq)]
struct Segments(pub Vec<Segment>);

#[derive(Debug, PartialEq)]
enum Segment {
    Static(String),
    Param(String),
    OptionalParam(String),
    Wildcard(String),
}

struct SegmentParser {
    input: proc_macro::token_stream::IntoIter,
    segments: Vec<Segment>,
}

impl SegmentParser {
    pub fn new(input: TokenStream) -> Self {
        Self {
            input: input.into_iter(),
            segments: Vec::new(),
        }
    }
}

impl SegmentParser {
    pub fn parse_all(&mut self) {
        for input in self.input.by_ref() {
            match input {
                TokenTree::Literal(lit) => {
                    let lit = lit.to_string();
                    if lit.contains("//") {
                        abort!(
                            proc_macro2::Span::call_site(),
                            "Consecutive '/' is not allowed"
                        );
                    }
                    Self::parse_str(
                        &mut self.segments,
                        lit.trim_start_matches(['"', '/'])
                            .trim_end_matches(['"', '/']),
                    );
                    if lit.ends_with(r#"/""#) && lit != r#""/""# {
                        self.segments.push(Segment::Static("/".to_string()));
                    }
                }
                TokenTree::Group(_) => unimplemented!(),
                TokenTree::Ident(_) => unimplemented!(),
                TokenTree::Punct(_) => unimplemented!(),
            }
        }
    }

    pub fn parse_str(segments: &mut Vec<Segment>, current_str: &str) {
        if ["", "*"].contains(&current_str) {
            return;
        }

        for segment in current_str.split('/') {
            if let Some(segment) = segment.strip_prefix(':') {
                if let Some(segment) = segment.strip_suffix('?') {
                    segments.push(Segment::OptionalParam(segment.to_string()));
                } else {
                    segments.push(Segment::Param(segment.to_string()));
                }
            } else if let Some(segment) = segment.strip_prefix('*') {
                segments.push(Segment::Wildcard(segment.to_string()));
            } else {
                segments.push(Segment::Static(segment.to_string()));
            }
        }
    }
}

impl Segment {
    fn is_valid(segment: &str) -> bool {
        segment == "/"
            || segment.chars().all(|c| {
                c.is_ascii_digit()
                    || c.is_ascii_lowercase()
                    || c.is_ascii_uppercase()
                    || RFC3986_UNRESERVED.contains(&c)
                    || RFC3986_PCHAR_OTHER.contains(&c)
            })
    }

    fn ensure_valid(&self) {
        match self {
            Self::Wildcard(s) if !Self::is_valid(s) => {
                abort!(Span::call_site(), "Invalid wildcard segment: {}", s)
            }
            Self::Static(s) if !Self::is_valid(s) => {
                abort!(Span::call_site(), "Invalid static segment: {}", s)
            }
            Self::Param(s) if !Self::is_valid(s) => {
                abort!(Span::call_site(), "Invalid param segment: {}", s)
            }
            _ => (),
        }
    }
}

impl Segments {
    fn ensure_valid(&self) {
        if let Some((_last, segments)) = self.0.split_last() {
            if let Some(Segment::Wildcard(s)) =
                segments.iter().find(|s| matches!(s, Segment::Wildcard(_)))
            {
                abort!(Span::call_site(), "Wildcard must be at end: {}", s)
            }
        }
    }
}

impl ToTokens for Segment {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.ensure_valid();
        match self {
            Segment::Wildcard(s) => {
                tokens.extend(quote! { leptos_router::WildcardSegment(#s) });
            }
            Segment::Static(s) => {
                tokens.extend(quote! { leptos_router::StaticSegment(#s) });
            }
            Segment::Param(p) => {
                tokens.extend(quote! { leptos_router::ParamSegment(#p) });
            }
            Segment::OptionalParam(p) => {
                tokens
                    .extend(quote! { leptos_router::OptionalParamSegment(#p) });
            }
        }
    }
}

impl ToTokens for Segments {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.ensure_valid();
        match self.0.as_slice() {
            [] => tokens.extend(quote! { () }),
            [segment] => tokens.extend(quote! { (#segment,) }),
            segments => tokens.extend(quote! { (#(#segments),*) }),
        }
    }
}

/// When added to an [`impl LazyRoute`] implementation block, this will automatically
/// add a [`lazy`] annotation to the `view` method, which will cause the code for the view
/// to lazy-load concurrently with the `data` being loaded for the route.
///
/// [`impl LazyRoute`]: https://docs.rs/leptos_router/latest/leptos_router/trait.LazyRoute.html
/// [`lazy`]: https://docs.rs/leptos_macro/latest/leptos_macro/macro.lazy.html
#[proc_macro_attribute]
#[proc_macro_error]
pub fn lazy_route(
    args: proc_macro::TokenStream,
    s: TokenStream,
) -> TokenStream {
    lazy_route_impl(args, s)
}

fn lazy_route_impl(
    _args: proc_macro::TokenStream,
    s: TokenStream,
) -> TokenStream {
    let mut im = syn::parse::<ItemImpl>(s).unwrap_or_else(|e| {
        abort!(e.span(), "`lazy_route` can only be used on an `impl` block")
    });
    if im.trait_.is_none() {
        abort!(
            im.span(),
            "`lazy_route` can only be used on an `impl LazyRoute for ...` \
             block"
        )
    }

    let self_ty = im.self_ty.clone();
    let ty_name_to_snake = match &*self_ty {
        Type::Path(TypePath {
            path: Path { segments, .. },
            ..
        }) => segments.last().unwrap().ident.to_string(),
        _ => abort!(self_ty.span(), "only path types are supported"),
    };
    let lazy_view_ident = Ident::new(&ty_name_to_snake, im.self_ty.span());

    let item = im.items.iter_mut().find_map(|item| match item {
        ImplItem::Fn(inner) => {
            if inner.sig.ident.to_string().as_str() == "view" {
                Some(inner)
            } else {
                None
            }
        }
        _ => None,
    });
    match item {
        None => abort!(im.span(), "must contain a fn called `view`"),
        Some(fun) => {
            let body = fun.block.clone();
            let new_block = quote! {{
                    #[cfg_attr(feature = "split", wasm_split::wasm_split(#lazy_view_ident))]
                    async fn view(this: #self_ty) -> ::leptos::prelude::AnyView {
                        #body
                    }

                    view(self).await
            }};
            let block =
                syn::parse::<Block>(new_block.into()).unwrap_or_else(|e| {
                    abort!(
                        e.span(),
                        "`lazy_route` can only be used on an `impl` block"
                    )
                });
            fun.block = block;
        }
    }

    quote! { #im }.into()
}
