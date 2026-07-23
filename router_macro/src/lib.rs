//! A macro to make path definitions easier with [`leptos_router`].
//!
//! [`leptos_router`]: https://docs.rs/leptos_router/latest/leptos_router/components/fn.Route.html

#![deny(missing_docs)]

use proc_macro::{TokenStream, TokenTree};
use proc_macro_error2::{abort, proc_macro_error, set_dummy};
use proc_macro2::Span;
use quote::{ToTokens, format_ident, quote};
use syn::{
    FnArg, Ident, ImplItem, ItemImpl, Path, Type, TypePath, parse_quote,
    spanned::Spanned,
};

const RFC3986_UNRESERVED: [char; 4] = ['-', '.', '_', '~'];
// RFC 3986 `pchar` also allows `:`, `@`, and the `sub-delims` set
// (`! $ & ' ( ) * + , ; =`). `*` is intentionally excluded here because the
// `path!` DSL reserves it as the wildcard sigil (see `Segment::Wildcard`);
// allowing it inside a static segment would mask misplaced-wildcard mistakes
// such as `path!("/home/any*")`.
const RFC3986_PCHAR_OTHER: [char; 12] =
    ['@', ':', '!', '$', '&', '\'', '(', ')', '+', ',', ';', '='];

/// Constructs a path for use in a [`Route`] definition.
///
/// Note that this is an optional convenience. Manually defining route segments
/// is equivalent.
///
/// # Examples
///
/// ```rust
/// use leptos_router::{
///     OptionalParamSegment, ParamSegment, StaticSegment, WildcardSegment,
///     path,
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
    let segments = Segments {
        span: parser.span,
        segments: parser.segments,
    };
    segments.into_token_stream().into()
}

struct Segments {
    segments: Vec<Segment>,
    // Span of the path string literal, used to anchor validation errors at the
    // literal rather than the whole `path!(...)` invocation.
    span: Span,
}

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
    span: Span,
}

impl SegmentParser {
    pub fn new(input: TokenStream) -> Self {
        Self {
            input: input.into_iter(),
            segments: Vec::new(),
            span: Span::call_site(),
        }
    }
}

impl SegmentParser {
    pub fn parse_all(&mut self) {
        let mut parsed = false;
        for input in self.input.by_ref() {
            match input {
                TokenTree::Literal(lit) => {
                    if parsed {
                        let span: Span = lit.span().into();
                        abort!(
                            span,
                            "`path!` accepts a single string literal; use \
                             `concat!` to build one from several pieces"
                        );
                    }
                    parsed = true;

                    // Parse via `syn::LitStr` so we operate on the literal's
                    // *value*, not its source text. This handles raw strings
                    // (`r"…"`, `r#"…"#`) and escapes uniformly, whereas
                    // `Literal::to_string()` returns the `r`/`#`/quote
                    // characters as part of the text.
                    let lit_str: syn::LitStr =
                        syn::parse(TokenStream::from(TokenTree::Literal(lit)))
                            .unwrap_or_else(|e| {
                                abort!(
                                    e.span(),
                                    "`path!` expects a string literal"
                                )
                            });
                    self.span = lit_str.span();
                    let value = lit_str.value();

                    if value.contains("//") {
                        abort!(self.span, "Consecutive '/' is not allowed");
                    }
                    Self::parse_str(
                        &mut self.segments,
                        value.trim_start_matches('/').trim_end_matches('/'),
                    );
                    if value.ends_with('/') && value != "/" {
                        self.segments.push(Segment::Static("/".to_string()));
                    }
                }
                other => {
                    let span: Span = other.span().into();
                    abort!(
                        span,
                        "`path!` expects a string literal, e.g. \
                         `path!(\"/users/:id\")`"
                    );
                }
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
        !segment.is_empty()
            && (segment == "/"
                || segment.chars().all(|c| {
                    c.is_ascii_digit()
                        || c.is_ascii_lowercase()
                        || c.is_ascii_uppercase()
                        || RFC3986_UNRESERVED.contains(&c)
                        || RFC3986_PCHAR_OTHER.contains(&c)
                }))
    }

    fn ensure_valid(&self, span: Span) {
        match self {
            Self::Wildcard(s) if !Self::is_valid(s) => {
                abort!(span, "Invalid wildcard segment: {}", s)
            }
            Self::Static(s) if !Self::is_valid(s) => {
                abort!(span, "Invalid static segment: {}", s)
            }
            Self::Param(s) if !Self::is_valid(s) => {
                abort!(span, "Invalid param segment: {}", s)
            }
            _ => (),
        }
    }
}

impl Segments {
    fn ensure_valid(&self) {
        if let Some((_last, segments)) = self.segments.split_last()
            && let Some(Segment::Wildcard(s)) =
                segments.iter().find(|s| matches!(s, Segment::Wildcard(_)))
        {
            abort!(self.span, "Wildcard must be at end: {}", s)
        }
        for segment in &self.segments {
            segment.ensure_valid(self.span);
        }
    }
}

impl ToTokens for Segment {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
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
        match self.segments.as_slice() {
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
/// If the view's WASM chunk fails to load (e.g. a transient network error), the error is
/// surfaced to the nearest [`ErrorBoundary`](leptos::prelude::ErrorBoundary) instead of
/// panicking, and the (uncached) failure is retried on the next navigation. Wrap your
/// `<Routes/>` in an `<ErrorBoundary>` to render a fallback; without one, a failed chunk
/// renders nothing.
///
/// ```rust
/// use leptos::prelude::*;
/// use leptos_router::{lazy_route, LazyRoute};
///
/// // the route definition
/// #[derive(Debug)]
/// struct BlogListingRoute {
///     titles: Resource<Vec<String>>
/// }
///
/// #[lazy_route]
/// impl LazyRoute for BlogListingRoute {
///     fn data() -> Self {
///         Self {
///             titles: Resource::new(|| (), |_| async {
///                 vec![/* todo: load blog posts */]
///             })
///         }
///     }
///
///     // this function will be lazy-loaded, concurrently with data()
///     fn view(this: Self) -> AnyView {
///         let BlogListingRoute { titles } = this;
///
///         // ... now you can use the `posts` resource with Suspense, etc.,
///         // and return AnyView by calling .into_any() on a view
///         # ().into_any()
///     }
/// }
/// ```
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
    set_dummy(s.clone().into());

    let mut im = syn::parse::<ItemImpl>(s.clone()).unwrap_or_else(|e| {
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
    let lazy_view_ident =
        Ident::new(&format!("__{ty_name_to_snake}_View"), im.self_ty.span());
    let preload_ident = format_ident!("__preload_{lazy_view_ident}");

    im.items.push(
        syn::parse::<ImplItem>(
            quote! {
                async fn preload() {
                    // TODO for 0.9 this is not precise
                    // we don't split routes for wasm32 ssr
                    // but we don't require a `hydrate`/`csr` feature on leptos_router
                    //
                    // Best-effort prefetch: discard a load error here (it is not
                    // cached) so a failed chunk does not panic; `view()` surfaces
                    // the error on the retry, where an `<ErrorBoundary>` can catch it.
                    #[cfg(target_arch = "wasm32")]
                    let _ = #preload_ident().await;
                }
            }
            .into(),
        )
        .unwrap_or_else(|e| {
            abort!(e.span(), "could not parse preload item impl")
        }),
    );

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
        None => abort!(
            im.span(),
            "`#[lazy_route]` requires a `view` method on the impl block"
        ),
        Some(fun) => {
            if let Some(a) = fun.sig.asyncness {
                abort!(a.span(), "`view` method should not be async")
            }
            if fun.sig.inputs.len() != 1 {
                abort!(
                    fun.sig.inputs.span(),
                    "`view` must take exactly one argument (`this: Self`)"
                )
            }
            fun.sig.asyncness = Some(Default::default());

            let first_arg = match fun.sig.inputs.first_mut() {
                Some(FnArg::Typed(arg)) => arg,
                Some(other) => abort!(
                    other.span(),
                    "this must be a typed argument like `this: Self`"
                ),
                None => abort!(fun.sig.span(), "must have an argument"),
            };

            // Preserve the user's binding pattern (`mut this`, `Self { .. }`,
            // `_`, …) on the generated lazy function, where pattern syntax is
            // valid. The trait `view` method instead binds a fresh identifier
            // and forwards it as an *expression*, since interpolating an
            // arbitrary pattern into call position produces invalid code.
            let user_pat = (*first_arg.pat).clone();
            let this_ident = Ident::new("__this", Span::call_site());
            first_arg.pat = parse_quote!(#this_ident);

            let body = std::mem::replace(
                &mut fun.block,
                parse_quote!({
                    // The split view fn is `fallible`, so it returns
                    // `Result<AnyView, LazyViewError>`. Erase it with
                    // `into_any()`: on `Err`, the `Result` renders to the
                    // nearest `<ErrorBoundary>` instead of panicking.
                    ::leptos::prelude::IntoAny::into_any(
                        #lazy_view_ident(#this_ident).await,
                    )
                }),
            );

            quote! {
                #[allow(non_snake_case)]
                #[::leptos::lazy(fallible)]
                fn #lazy_view_ident(
                    #user_pat: #self_ty,
                ) -> ::core::result::Result<
                    ::leptos::prelude::AnyView,
                    ::leptos::LazyViewError,
                > {
                    ::core::result::Result::Ok(#body)
                }

                #im
            }
            .into()
        }
    }
}
