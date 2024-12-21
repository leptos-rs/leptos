#![cfg_attr(feature = "nightly", feature(proc_macro_span))]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Implementation of the `server_fn` macro.
//!
//! This crate contains the implementation of the `server_fn` macro. [`server_macro_impl`] can be used to implement custom versions of the macro for different frameworks that allow users to pass a custom context from the server to the server function.

use convert_case::{Case, Converter};
use proc_macro2::{Literal, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    *,
};
use token::Where;

/// extract all #[middleware] attributes, removing them from signature of dummy
fn extract_middlewares(body: &mut ServerFnBody) -> Vec<Middleware> {
    let mut middlewares: Vec<Middleware> = vec![];
    body.attrs.retain(|attr| {
        if attr.meta.path().is_ident("middleware") {
            if let Ok(middleware) = attr.parse_args() {
                middlewares.push(middleware);
                false
            } else {
                true
            }
        } else {
            true
        }
    });
    middlewares
}

/// extract register if it exists, return an error if there's more than one
fn extract_register(body: &mut ServerFnBody) -> Result<Option<Register>> {
    let mut register: Vec<Register> = vec![];
    body.attrs.retain(|attr| {
        if attr.meta.path().is_ident("register") {
            if let Ok(r) = attr.parse_args() {
                register.push(r);
                false
            } else {
                true
            }
        } else {
            true
        }
    });
    if register.len() > 1 {
        return Err(syn::Error::new(
            Span::call_site(),
            "cannot use more than 1 register attribute",
        ));
    }
    Ok(register.first().cloned())
}

/// Takes body, and returns a list of field types to compare to.
fn input_types(body: &ServerFnBody) -> Vec<Ident> {
    body.inputs
        .iter()
        .filter_map(|input| {
            if let FnArg::Typed(pat) = input {
                let ty = pat.ty.clone();
                let ident: Ident = parse_quote!(#ty);
                Some(ident)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}

fn extend_fields_with_phantom_types(
    fields: Vec<TokenStream2>,
    phantom_types: Vec<Ident>,
) -> Vec<TokenStream2> {
    if !phantom_types.is_empty() {
        let mut fields = fields;
        let q = quote! {
                #[serde(skip)]
                #[allow(unused_parens)]
                pub _marker: ::std::marker::PhantomData<(#(#phantom_types),*)>

        };
        // panic!("{:?}",phantom_types);
        fields.push(q);
        fields
    } else {
        fields
    }
}

/// construct typed fields for our server functions structure, making use of the attributes on the server function inputs.
fn fields(body: &mut ServerFnBody) -> Result<Vec<TokenStream2>> {
    body.inputs
        .iter_mut()
        .map(|f| {
            let typed_arg = match f {
                FnArg::Receiver(_) => {
                    return Err(syn::Error::new(
                        f.span(),
                        "cannot use receiver types in server function macro",
                    ))
                }
                FnArg::Typed(t) => t,
            };

            // strip `mut`, which is allowed in fn args but not in struct fields
            if let Pat::Ident(ident) = &mut *typed_arg.pat {
                ident.mutability = None;
            }

            fn rename_path(
                path: Path,
                from_ident: Ident,
                to_ident: Ident,
            ) -> Path {
                if path.is_ident(&from_ident) {
                    Path {
                        leading_colon: None,
                        segments: Punctuated::from_iter([PathSegment {
                            ident: to_ident,
                            arguments: PathArguments::None,
                        }]),
                    }
                } else {
                    path
                }
            }

            let attrs = typed_arg
                .attrs
                .iter()
                .cloned()
                .map(|attr| {
                    if attr.path().is_ident("server") {
                        // Allow the following attributes:
                        // - #[server(default)]
                        // - #[server(rename = "fieldName")]

                        // Rename `server` to `serde`
                        let attr = Attribute {
                            meta: match attr.meta {
                                Meta::Path(path) => Meta::Path(rename_path(
                                    path,
                                    format_ident!("server"),
                                    format_ident!("serde"),
                                )),
                                Meta::List(mut list) => {
                                    list.path = rename_path(
                                        list.path,
                                        format_ident!("server"),
                                        format_ident!("serde"),
                                    );
                                    Meta::List(list)
                                }
                                Meta::NameValue(mut name_value) => {
                                    name_value.path = rename_path(
                                        name_value.path,
                                        format_ident!("server"),
                                        format_ident!("serde"),
                                    );
                                    Meta::NameValue(name_value)
                                }
                            },
                            ..attr
                        };

                        let args = attr.parse_args::<Meta>()?;
                        match args {
                            // #[server(default)]
                            Meta::Path(path) if path.is_ident("default") => {
                                Ok(attr.clone())
                            }
                            // #[server(flatten)]
                            Meta::Path(path) if path.is_ident("flatten") => {
                                Ok(attr.clone())
                            }
                            // #[server(default = "value")]
                            Meta::NameValue(name_value)
                                if name_value.path.is_ident("default") =>
                            {
                                Ok(attr.clone())
                            }
                            // #[server(skip)]
                            Meta::Path(path) if path.is_ident("skip") => {
                                Ok(attr.clone())
                            }
                            // #[server(rename = "value")]
                            Meta::NameValue(name_value)
                                if name_value.path.is_ident("rename") =>
                            {
                                Ok(attr.clone())
                            }
                            _ => Err(Error::new(
                                attr.span(),
                                "Unrecognized #[server] attribute, expected \
                                 #[server(default)] or #[server(rename = \
                                 \"fieldName\")]",
                            )),
                        }
                    } else if attr.path().is_ident("doc") {
                        // Allow #[doc = "documentation"]
                        Ok(attr.clone())
                    } else if attr.path().is_ident("allow") {
                        // Allow #[allow(...)]
                        Ok(attr.clone())
                    } else if attr.path().is_ident("deny") {
                        // Allow #[deny(...)]
                        Ok(attr.clone())
                    } else if attr.path().is_ident("ignore") {
                        // Allow #[ignore]
                        Ok(attr.clone())
                    } else {
                        Err(Error::new(
                            attr.span(),
                            "Unrecognized attribute, expected #[server(...)]",
                        ))
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            typed_arg.attrs = vec![];
            Ok(quote! { #(#attrs ) * pub #typed_arg })
        })
        .collect::<Result<Vec<_>>>()
}

/// we need to apply the same sort of Actix SendWrapper workaround here
/// that we do for the body of the function provided in the trait (see below)  
fn actix_workaround(body: &mut ServerFnBody, server_fn_path: &Option<Path>) {
    if cfg!(feature = "actix") {
        let block = body.block.to_token_stream();
        body.block = quote! {
            {
                #server_fn_path::actix::SendWrapper::new(async move {
                    #block
                })
                .await
            }
        };
    }
}

/// This gives the input encoding.
fn input_to_string(input: &Option<Type>) -> Option<String> {
    match &input {
        Some(Type::Path(path)) => {
            path.path.segments.last().map(|seg| seg.ident.to_string())
        }
        None => Some("PostUrl".to_string()),
        _ => None,
    }
}

/// Construct the token stream for the input associated type of our server fn impl.
fn input_encoding_tokens(
    input: Option<Type>,
    builtin_encoding: bool,
    server_fn_path: &Option<Path>,
) -> TokenStream2 {
    input
        .map(|n| {
            if builtin_encoding {
                quote! { #server_fn_path::codec::#n }
            } else {
                n.to_token_stream()
            }
        })
        .unwrap_or_else(|| {
            quote! {
                #server_fn_path::codec::PostUrl
            }
        })
}

/// Construct token stream for our output associated type of our server fn impl.
fn output_encoding_tokens(
    output: Option<Type>,
    builtin_encoding: bool,
    server_fn_path: &Option<Path>,
) -> TokenStream2 {
    output
        .map(|n| {
            if builtin_encoding {
                quote! { #server_fn_path::codec::#n }
            } else {
                n.to_token_stream()
            }
        })
        .unwrap_or_else(|| {
            quote! {
                #server_fn_path::codec::Json
            }
        })
}

/// The name of the server function struct.
/// default to PascalCase version of function name if no struct name given
fn struct_name_ident(struct_name: Option<Ident>, body: &ServerFnBody) -> Ident {
    struct_name.unwrap_or_else(|| {
        let upper_camel_case_name = Converter::new()
            .from_case(Case::Snake)
            .to_case(Case::UpperCamel)
            .convert(body.ident.to_string());
        Ident::new(&upper_camel_case_name, body.ident.span())
    })
}

/// If there is a custom wrapper, we wrap our struct name in it. Otherwise this will be the struct name.
fn possibly_wrap_struct_name(
    struct_name: &Ident,
    custom_wrapper: &Option<Path>,
    ty_generics: Option<&TypeGenerics>,
) -> TokenStream2 {
    if let Some(wrapper) = custom_wrapper {
        quote! { #wrapper<#struct_name #ty_generics> }
    } else {
        quote! { #struct_name #ty_generics }
    }
}

/// If there is a custom wrapper, we create a version with turbofish, where the argument to the turbo fish is the struct name.
/// otherwise its just the struct name.
fn possibly_wrapped_struct_name_turbofish(
    struct_name: &Ident,
    custom_wrapper: &Option<Path>,
    ty_generics: Option<&TypeGenerics>,
) -> TokenStream2 {
    if let Some(wrapper) = custom_wrapper.as_ref() {
        if let Some(ty_generics) = ty_generics {
            let ty_generics = ty_generics.as_turbofish();
            quote! { #wrapper::<#struct_name #ty_generics> }
        } else {
            quote! { #wrapper::<#struct_name> }
        }
    } else if let Some(ty_generics) = ty_generics {
        let ty_generics = ty_generics.as_turbofish();
        quote! { #struct_name #ty_generics }
    } else {
        quote! { #struct_name #ty_generics }
    }
}

/// Produce the fn_args for the server function which is called from the client and the server.
fn fn_args(body: &ServerFnBody) -> Vec<&PatType> {
    body.inputs
        .iter()
        .filter_map(|f| match f {
            FnArg::Receiver(_) => None,
            FnArg::Typed(t) => Some(t),
        })
        .collect::<Vec<_>>()
}

/// Get just the field names of our server function structure. This is useful for deconstructing it.
fn field_names(body: &ServerFnBody) -> Vec<&Box<Pat>> {
    body.inputs
        .iter()
        .filter_map(|f| match f {
            FnArg::Receiver(_) => None,
            FnArg::Typed(t) => Some(&t.pat),
        })
        .collect::<Vec<_>>()
}

/// if there's exactly one field, impl From<T> for the struct.
/// The return type here will be the From<T> implementation which can be added to the macro.
fn impl_from_tokens(
    body: &ServerFnBody,
    impl_from: Option<LitBool>,
    struct_name: &Ident,
) -> Option<TokenStream2> {
    let first_field = body.inputs.iter().find_map(|f| match f {
        FnArg::Receiver(_) => None,
        FnArg::Typed(t) => Some((&t.pat, &t.ty)),
    });
    let impl_from = impl_from.map(|v| v.value).unwrap_or(true);
    (body.inputs.len() == 1 && first_field.is_some() && impl_from).then(|| {
        let field = first_field.unwrap();
        let (name, ty) = field;
        quote! {
            impl From<#struct_name> for #ty {
                fn from(value: #struct_name) -> Self {
                    let #struct_name { #name } = value;
                    #name
                }
            }

            impl From<#ty> for #struct_name {
                fn from(#name: #ty) -> Self {
                    #struct_name { #name }
                }
            }
        }
    })
}

/// The server fn crate exports serde, create path to serde via server fn path.
fn serde_path(server_fn_path: &Option<Path>) -> Option<String> {
    server_fn_path.as_ref().map(|path| {
        let path = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        let path = path.join("::");
        format!("{path}::serde")
    })
}

/// We add documentation to the server function structure to say what it is. (The serialized arguments of the server function)
fn args_docs_tokens(fn_name_as_str: &String) -> TokenStream2 {
    let link_to_server_fn = format!(
        "Serialized arguments for the [`{fn_name_as_str}`] server \
         function.\n\n"
    );
    let args_docs = quote! {
        #[doc = #link_to_server_fn]
    };
    args_docs
}

/// Any documentation from the server function in the users code under the `#[server]` macro should appear on the client and server functions that we generate.
fn docs_tokens(body: &ServerFnBody) -> TokenStream2 {
    body.docs
        .iter()
        .map(|(doc, span)| quote_spanned!(*span=> #[doc = #doc]))
        .collect::<TokenStream2>()
}

/// On the server we need to register the server function with inventory. Otherwise this is empty.
fn inventory_tokens(
    server_fn_path: &TokenStream2,
    wrapped_struct_name_turbofish: &TokenStream2,
    wrapped_struct_name: &TokenStream2,
) -> TokenStream2 {
    if cfg!(feature = "ssr") {
        quote! {
            #server_fn_path::inventory::submit! {{
                use #server_fn_path::{ServerFn, codec::Encoding};
                #server_fn_path::ServerFnTraitObj::new(
                    #wrapped_struct_name_turbofish::PATH,
                    <#wrapped_struct_name as ServerFn>::InputEncoding::METHOD,
                    |req| {
                        Box::pin(#wrapped_struct_name_turbofish::run_on_server(req))
                    },
                    #wrapped_struct_name_turbofish::middlewares
                )
            }}
        }
    } else {
        quote! {}
    }
}

/// The ServerFn trait, has a method called run_body. We have different implementations for the server function struct for the server and the client.
/// This creates both implementations.
fn run_body_tokens(
    custom_wrapper: &Option<Path>,
    struct_name: &Ident,
    field_names: &Vec<&Box<Pat>>,
    dummy_name: &Ident,
    server_fn_path: &TokenStream2,
    output_ty: &GenericArgument,
    error_ty: &TokenStream2,
    has_marker: bool,
    specific_ty_phantom_suffix_removed: Option<&TypeGenerics>,
) -> TokenStream2 {
    if cfg!(feature = "ssr") {
        let marker: Box<Pat> = Box::new(parse_quote!(_marker));
        // We are taking references in the function, I don't want to propagate type changes so I wrote this instead.
        let possibly_extend_field_names_with_marker =
            |field_names: &Vec<&Box<Pat>>| -> Vec<Box<Pat>> {
                if has_marker {
                    let mut field_names = field_names.clone();
                    field_names.push(&marker);
                    field_names
                        .into_iter().cloned()
                        .collect::<Vec<_>>()
                } else {
                    field_names
                        .clone()
                        .into_iter().cloned()
                        .collect::<Vec<_>>()
                }
            };
        let destructure = if let Some(wrapper) = custom_wrapper.as_ref() {
            let field_names =
                possibly_extend_field_names_with_marker(field_names);
            quote! {
                let #wrapper(#struct_name { #(#field_names),* }) = self;
            }
        } else {
            let field_names =
                possibly_extend_field_names_with_marker(field_names);
            quote! {
                let #struct_name { #(#field_names),* } = self;
            }
        };
        let specific_ty =
            specific_ty_phantom_suffix_removed.map(|ty| ty.as_turbofish());
        // using the impl Future syntax here is thanks to Actix
        //
        // if we use Actix types inside the function, here, it becomes !Send
        // so we need to add SendWrapper, because Actix won't actually send it anywhere
        // but if we used SendWrapper in an async fn, the types don't work out because it
        // becomes impl Future<Output = SendWrapper<_>>
        //
        // however, SendWrapper<Future<Output = T>> impls Future<Output = T>
        let body = quote! {
            #destructure
            #dummy_name #specific_ty (#(#field_names),*).await
        };
        let body = if cfg!(feature = "actix") {
            quote! {
                #server_fn_path::actix::SendWrapper::new(async move {
                    #body
                })
            }
        } else {
            quote! { async move {
                #body
            }}
        };
        quote! {
            // we need this for Actix, for the SendWrapper to count as impl Future
            // but non-Actix will have a clippy warning otherwise
            #[allow(clippy::manual_async_fn)]
            fn run_body(self) -> impl std::future::Future<Output = Result< #output_ty, #server_fn_path::error::ServerFnError< #error_ty >>> + Send {
                #body
            }
        }
    } else {
        quote! {
            #[allow(unused_variables)]
            async fn run_body(self) -> Result< #output_ty, #server_fn_path::error::ServerFnError< #error_ty >> {
                unreachable!()
            }
        }
    }
}

/// Our function that we use on the server (and wraps the dummy function), if it has types that take Traits that end with Constraint
/// Then we map those into their ServerTypes and pass them to the dummy function.
fn extend_generics_phantom_suffix_with_server_type_trait(
    impl_generics: Option<&ImplGenerics>,
    where_clause: Option<&WhereClause>,
) -> (Generics, TokenStream2) {
    let signature: Signature =
        parse_quote!(fn dummy #impl_generics () #where_clause);
    let mut g: Generics = signature.generics;
    let server_type: Type = parse_quote!(::server_fn::ssr_generics::ServerType);
    let mut server_type_predicates = Vec::new();
    let mut type_has_trait_constraints = std::collections::HashSet::new();
    // Go through each type parameters in the generics for our original server function
    for ty in g.type_params_mut() {
        let mut server_type_constraints: Punctuated<Ident, Token![+]> =
            Punctuated::new();
        let mut propagate_flag = false;
        // Type is a ssr only type if it implements a TraitConstraint for an ssr only trait,
        // strip the constraint suffix and propagate the bounds to our ServerType where predicate
        for type_bound in ty.bounds.iter() {
            if let TypeParamBound::Trait(trait_bound) = type_bound {
                if let Some(last) = trait_bound.path.segments.last() {
                    let ident = last.ident.clone();
                    let ident_str = ident.to_string();
                    let ident = if ident_str.ends_with("Constraint")
                        && ident_str.len() > "Constraint".len()
                    {
                        type_has_trait_constraints.insert(ty.ident.clone());
                        propagate_flag = true;
                        Ident::new(
                            &ident_str[0..ident_str.len() - "Constraint".len()],
                            Span::call_site(),
                        )
                    } else {
                        ident
                    };
                    server_type_constraints.push(ident);
                }
            }
        }
        let ident = &ty.ident;
        if propagate_flag {
            // Add a where predicate to the public ssr server function that requires that TPhantom implement ServerType.
            ty.bounds.push(parse_quote!(#server_type));
            // Now add the where predicate we've constructed for the server type.
            let predicate: WherePredicate = parse_quote!(< #ident as #server_type > :: ServerType : #server_type_constraints);

            server_type_predicates.push(predicate);
        }
    }
    let where_clause = g.make_where_clause();
    for predicate in server_type_predicates {
        where_clause.predicates.push(predicate);
    }
    // No we have the generics for the ssr server function, we need to construct the canonocalized generics for our dummy function that we call inside of the server function,
    let generics_for_ssr_server_function = g;
    let signature: Signature = parse_quote!(fn dummy #impl_generics ());
    // Our dummy function doesn't take TPhantom, it instead takes <TPhantom as ServerType>::ServerType.
    let mut g: Generics = signature.generics;
    let mut types: Punctuated<TokenStream2, Token![,]> = Punctuated::new();
    for ty in g.type_params_mut() {
        let ident = &ty.ident;
        // if the original server function is accepting a T : TraitConstraint
        if type_has_trait_constraints.contains(ident) {
            types.push(quote!(< #ident as #server_type> :: ServerType));
        } else {
            types.push(quote!(#ident))
        }
    }
    // we return the generics for the outer function and the turbo fish types for the dummy function
    let dummy_fish = if types.is_empty() {
        quote!()
    } else {
        quote!(:: < #types >)
    };
    (generics_for_ssr_server_function, dummy_fish)
}
/// We generate the function that is actually called from the users code. This generates both the function called from the server,
/// and the function called from the client.
fn func_tokens(
    docs: &TokenStream2,
    attrs: &Vec<Attribute>,
    vis: &Visibility,
    fn_name: &Ident,
    fn_args: &Vec<&PatType>,
    output_arrow: &token::RArrow,
    return_ty: &Type,
    dummy_name: &Ident,
    field_names: &Vec<&Box<Pat>>,
    custom_wrapper: &Option<Path>,
    struct_name: &Ident,
    server_fn_path: &TokenStream2,
    impl_generics: Option<&ImplGenerics>,
    impl_generics_with_trait_constraints_and_phantom_suffix: Option<
        &ImplGenerics,
    >,
    ty_generics: Option<&TypeGenerics>,
    where_clause: Option<&WhereClause>,
    output_ty: &GenericArgument,
    error_ty: &TokenStream2,
    has_marker: bool,
) -> TokenStream2 {
    if cfg!(feature = "ssr") {
        let (outer_generics, dummy_types) =
            extend_generics_phantom_suffix_with_server_type_trait(
                impl_generics_with_trait_constraints_and_phantom_suffix,
                where_clause,
            );

        let (impl_generics, _, where_clause) = outer_generics.split_for_impl();
        quote! {
            #docs
            #(#attrs)*
            #vis async fn #fn_name #impl_generics (#(#fn_args),*) #output_arrow #return_ty #where_clause {
                #dummy_name #dummy_types (#(#field_names),*).await
            }
        }
    } else {
        // where clause might be empty even though others are not
        let where_clause = {
            if ty_generics.is_some()
                || impl_generics_with_trait_constraints_and_phantom_suffix
                    .is_some()
            {
                Some(WhereClause {
                    where_token: Where {
                        span: Span::call_site(),
                    },
                    predicates: Punctuated::new(),
                })
            } else {
                where_clause.cloned()
            }
        };
        let where_clause = where_clause.map(|mut where_clause|{
            // we need to extend the where clause of our restructure so that we can call .run_on_client on our data
            // since our serverfn is only implemented for the types we've specified in register, we need to specify we are only calling
            // run_on_client where we have implemented server fn.
            where_clause.predicates.push(
                parse_quote!(#struct_name #ty_generics : #server_fn_path::ServerFn <Output = #output_ty, Error = #error_ty> )
            );
            where_clause
        });
        let ty_generics = ty_generics.map(|this| this.as_turbofish());
        let mut field_names = field_names.clone();
        let marker = Box::new(parse_quote!(_marker));
        if has_marker {
            field_names.push(&marker);
        }
        let make_marker = if has_marker {
            let turbo_fish =
                ty_generics.clone().expect("has_marker iff ty_generics");
            quote!(let _marker = ::std::marker::PhantomData #turbo_fish ;)
        } else {
            quote!()
        };
        let restructure = if let Some(custom_wrapper) = custom_wrapper.as_ref()
        {
            quote! {
                let data = #custom_wrapper(#struct_name #ty_generics { #(#field_names),* });
            }
        } else {
            quote! {
                let data = #struct_name #ty_generics { #(#field_names),* };
            }
        };
        quote! {
            #docs
            #(#attrs)*
            #[allow(unused_variables)]
            #vis async fn #fn_name #impl_generics_with_trait_constraints_and_phantom_suffix  (#(#fn_args),*) #output_arrow #return_ty #where_clause {
                use #server_fn_path::ServerFn;
                #make_marker
                #restructure
                data.run_on_client().await
            }
        }
    }
}

/// Produces an additional path (to go under the derives) and derives for our server function structure, additional path could currently be the path to serde.
fn additional_path_and_derives_tokens(
    input_ident: Option<String>,
    input_derive: &Option<ExprTuple>,
    server_fn_path: &TokenStream2,
    serde_path: &Option<String>,
) -> (TokenStream2, TokenStream2) {
    enum PathInfo {
        Serde,
        Rkyv,
        None,
    }

    let (path, derives) = match input_ident.as_deref() {
        Some("Rkyv") => (
            PathInfo::Rkyv,
            quote! {
                Clone, #server_fn_path::rkyv::Archive, #server_fn_path::rkyv::Serialize, #server_fn_path::rkyv::Deserialize
            },
        ),
        Some("MultipartFormData")
        | Some("Streaming")
        | Some("StreamingText") => (PathInfo::None, quote! {}),
        Some("SerdeLite") => (
            PathInfo::Serde,
            quote! {
                Clone, #server_fn_path::serde_lite::Serialize, #server_fn_path::serde_lite::Deserialize
            },
        ),
        _ => match input_derive {
            Some(derives) => {
                let d = &derives.elems;
                (PathInfo::None, quote! { #d })
            }
            None => (
                PathInfo::Serde,
                quote! {
                    Clone, #server_fn_path::serde::Serialize, #server_fn_path::serde::Deserialize
                },
            ),
        },
    };
    let addl_path = match path {
        PathInfo::Serde => quote! {
            #[serde(crate = #serde_path)]
        },
        PathInfo::Rkyv => quote! {},
        PathInfo::None => quote! {},
    };
    (addl_path, derives)
}

/// The code for our Client associated type on our ServerFn impl
fn client_tokens(
    client: &Option<Type>,
    server_fn_path: &TokenStream2,
) -> TokenStream2 {
    if let Some(client) = client {
        client.to_token_stream()
    } else if cfg!(feature = "reqwest") {
        quote! {
            #server_fn_path::client::reqwest::ReqwestClient
        }
    } else {
        quote! {
            #server_fn_path::client::browser::BrowserClient
        }
    }
}

/// Generates the code for our Req associated type on our ServerFn impl. Generates both client and server versions, as well as framework specific code.
fn req_tokens(
    server_fn_path: &TokenStream2,
    req_ty: &Option<Type>,
    preset_req: &Option<Type>,
) -> TokenStream2 {
    if !cfg!(feature = "ssr") {
        quote! {
            #server_fn_path::request::BrowserMockReq
        }
    } else if cfg!(feature = "axum") {
        quote! {
            #server_fn_path::http_export::Request<#server_fn_path::axum_export::body::Body>
        }
    } else if cfg!(feature = "actix") {
        quote! {
            #server_fn_path::request::actix::ActixRequest
        }
    } else if cfg!(feature = "generic") {
        quote! {
            #server_fn_path::http_export::Request<#server_fn_path::bytes_export::Bytes>
        }
    } else if let Some(req_ty) = req_ty {
        req_ty.to_token_stream()
    } else if let Some(req_ty) = preset_req {
        req_ty.to_token_stream()
    } else {
        // fall back to the browser version, to avoid erroring out
        // in things like doctests
        // in reality, one of the above needs to be set
        quote! {
            #server_fn_path::request::BrowserMockReq
        }
    }
}

/// Generates the code for our Resp associated type on our ServerFn impl. Generates both server and client code, and server framework specific code.
fn resp_tokens(
    server_fn_path: &TokenStream2,
    res_ty: &Option<Type>,
    preset_res: &Option<Type>,
) -> TokenStream2 {
    if !cfg!(feature = "ssr") {
        quote! {
            #server_fn_path::response::BrowserMockRes
        }
    } else if cfg!(feature = "axum") {
        quote! {
            #server_fn_path::http_export::Response<#server_fn_path::axum_export::body::Body>
        }
    } else if cfg!(feature = "actix") {
        quote! {
            #server_fn_path::response::actix::ActixResponse
        }
    } else if cfg!(feature = "generic") {
        quote! {
            #server_fn_path::http_export::Response<#server_fn_path::response::generic::Body>
        }
    } else if let Some(res_ty) = res_ty {
        res_ty.to_token_stream()
    } else if let Some(res_ty) = preset_res {
        res_ty.to_token_stream()
    } else {
        // fall back to the browser version, to avoid erroring out
        // in things like doctests
        // in reality, one of the above needs to be set
        quote! {
            #server_fn_path::response::BrowserMockRes
        }
    }
}

/// The ServerFn impl has an associated const PATH.
fn path_tokens(
    endpoint: &Literal,
    server_fn_path: &TokenStream2,
    prefix: &Literal,
    fn_name_as_str: &String,
    specified_generics: Option<&Punctuated<Ident, Token![,]>>,
) -> TokenStream2 {
    let key_env_var = match option_env!("SERVER_FN_OVERRIDE_KEY") {
        Some(_) => "SERVER_FN_OVERRIDE_KEY",
        None => "CARGO_MANIFEST_DIR",
    };

    // Remove any leading slashes, even if they exist (we'll add them below)
    let endpoint = Literal::string(
        endpoint
            .to_string()
            .trim_start_matches('\"')
            .trim_start_matches('/')
            .trim_end_matches('\"'),
    );

    let endpoint_starts_with_slash = endpoint.to_string().starts_with("\"/");
    let endpoint =
        if endpoint_starts_with_slash || endpoint.to_string() == "\"\"" {
            quote! { #endpoint }
        } else {
            quote! { concat!("/", #endpoint) }
        };
    let mut generics = vec![];
    for ident in specified_generics.unwrap_or(&Punctuated::new()).iter() {
        generics.push(format!("{ident}"));
    }

    let path = quote! {
        if #endpoint.is_empty() {
            #server_fn_path::const_format::concatcp!(
                #prefix,
                "/",
                #fn_name_as_str,
                #server_fn_path::xxhash_rust::const_xxh64::xxh64(
                    concat!(env!(#key_env_var), ":", file!(), ":", line!(), ":", column!(), #(":", #generics)*).as_bytes(),
                    0
                )
            )
        } else {
            #server_fn_path::const_format::concatcp!(
                #prefix,
                #endpoint
            )
        }
    };
    path
}

/// Convert our middlewares list into a token stream for interpolation.
fn middlewares_tokens(middlewares: &Vec<Middleware>) -> TokenStream2 {
    if cfg!(feature = "ssr") {
        quote! {
            vec![
                #(
                    std::sync::Arc::new(#middlewares),
                ),*
            ]
        }
    } else {
        quote! { vec![] }
    }
}

fn error_ty_tokens(
    return_ty: &Type,
    server_fn_path: &Option<Path>,
) -> Result<TokenStream2> {
    let error_ty = err_type(return_ty)?;
    let error_ty =
        error_ty.map(ToTokens::to_token_stream).unwrap_or_else(|| {
            quote! {
                #server_fn_path::error::NoCustomError
            }
        });
    Ok(error_ty)
}

/// The implementation of the `server` macro.
/// ```ignore
/// #[proc_macro_attribute]
/// pub fn server(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
///     match server_macro_impl(
///         args.into(),
///         s.into(),
///         Some(syn::parse_quote!(my_crate::exports::server_fn)),
///     ) {
///         Err(e) => e.to_compile_error().into(),
///         Ok(s) => s.to_token_stream().into(),
///     }
/// }
/// ```
pub fn server_macro_impl(
    args: TokenStream2,
    body: TokenStream2,
    server_fn_path: Option<Path>,
    default_path: &str,
    preset_req: Option<Type>,
    preset_res: Option<Type>,
) -> Result<TokenStream2> {
    let mut body = syn::parse::<ServerFnBody>(body.into())?;

    let middlewares = extract_middlewares(&mut body);
    let maybe_register = extract_register(&mut body)?;
    let fields = fields(&mut body)?;

    let dummy = body.to_dummy_output();
    // the dummy name is the name prefixed with __ so server_fn() -> __server_fn
    let dummy_name = body.to_dummy_ident();
    let args = syn::parse::<ServerFnArgs>(args.into())?;

    // default values for args
    let ServerFnArgs {
        struct_name,
        prefix,
        input,
        input_derive,
        output,
        endpoint,
        builtin_encoding,
        req_ty,
        res_ty,
        client,
        custom_wrapper,
        impl_from,
    } = args;

    // does nothing if no feature = actix
    actix_workaround(&mut body, &server_fn_path);

    // These are used in the PATH construction for our ServerFn impl.
    // They are ignored when using generics in favor of endpoints specified per registered types.
    let prefix = prefix.unwrap_or_else(|| Literal::string(default_path));
    let endpoint = endpoint.unwrap_or_else(|| Literal::string(""));

    let input_ident = input_to_string(&input);

    // build struct for type
    let struct_name = struct_name_ident(struct_name, &body);
    let fn_name = &body.ident;
    let fn_name_as_str = body.ident.to_string();
    let vis = &body.vis;
    let attrs = &body.attrs;

    let fn_args = fn_args(&body);

    let field_names = field_names(&body);

    // check output type
    let output_arrow = &body.output_arrow;
    let return_ty = &body.return_ty;

    // build server fn path
    let serde_path = serde_path(&server_fn_path);

    // turn the server fn path into a tokens, instead of using the empty string for none we'll use 'server_fn'
    let server_fn_path_token = server_fn_path
        .clone()
        .map(|path| quote!(#path))
        .unwrap_or_else(|| quote! { server_fn });

    let args_docs = args_docs_tokens(&fn_name_as_str);

    // pass through docs
    let docs = docs_tokens(&body);
    // for the server function structure
    let (additional_path, derives) = additional_path_and_derives_tokens(
        input_ident,
        &input_derive,
        &server_fn_path_token,
        &serde_path,
    );
    // for the associated types that are not PATH
    let client = client_tokens(&client, &server_fn_path_token);
    let req = req_tokens(&server_fn_path_token, &req_ty, &preset_req);
    let res = resp_tokens(&server_fn_path_token, &res_ty, &preset_res);
    let output_ty = output_type(return_ty)?;
    let input_encoding =
        input_encoding_tokens(input, builtin_encoding, &server_fn_path);
    let output_encoding =
        output_encoding_tokens(output, builtin_encoding, &server_fn_path);
    let error_ty = error_ty_tokens(return_ty, &server_fn_path)?;

    // only emit the dummy (unmodified server-only body) for the server build
    let dummy = cfg!(feature = "ssr").then_some(dummy);
    if let Some(register) = maybe_register {
        let (impl_generics, ty_generics, where_clause) =
            body.generics.split_for_impl();

        let input_types = input_types(&body);

        // let InputFieldTypes = {T ∈ InputFieldTypes | T is a type in the InputTypes of the OriginalServerFunction}
        // let PhantomTypes = {U ∈ GenericParameters of OriginalServerFunction | U ∉ InputFieldTypes}
        // If PhantomTypes is not empty, then we need to add a _marker field to our GenericServerFunctionStructure
        // Whose type is PhantomData<(∀Ui​∈U)>

        // PhantomTypes are not types that end in the word Phantom. But types that are in PhantomData<...> lol
        let phantom_types = {
            let mut phantom_types = body
                .generics
                .type_params()
                .map(|ty| ty.ident.clone())
                .collect::<Vec<_>>();
            phantom_types.retain(|ty| !input_types.contains(ty));
            phantom_types
        };
        let has_marker = !phantom_types.is_empty();

        // for each register entry, we generate a unique implementation and inventory.
        let mut implementations_and_inventories = vec![];

        for RegisterEntry { internal, endpoint } in
            register.as_registered_entries()
        {
            let specific_ty = Punctuated::<Ident, Token![,]>::from_iter(
                internal.iter().map(|i| i.specific_ty.clone()),
            );
            let output_ty = canonicalize_output_type_generic(
                &parse_quote!(#ty_generics),
                &specific_ty,
                output_ty,
            );
            let specific_ty_phantom_suffix_removed = specific_ty
                .clone()
                .into_iter()
                .map(|ident| {
                    let ident_str = ident.to_string();
                    if ident_str.ends_with("Phantom")
                        && ident_str.len() > "Phantom".len()
                    {
                        syn::Ident::new(
                            &ident_str[..ident_str.len() - "Phantom".len()],
                            ident.span(),
                        )
                    } else {
                        ident
                    }
                })
                .collect::<Punctuated<Ident, Token![,]>>();
            let g: Generics = parse_quote!(< #specific_ty >);
            let (_, ty_generics, _) = g.split_for_impl();
            let g: Generics =
                parse_quote!(< #specific_ty_phantom_suffix_removed >);
            let (_, ty_generics_phantom_suffix_removed, _) = g.split_for_impl();
            // These will be the struct name with the specific generics for a given register arg
            // i.e register(<String>) -> #struct_name <String>
            let wrapped_struct_name = possibly_wrap_struct_name(
                &struct_name,
                &custom_wrapper,
                Some(&ty_generics),
            );
            let wrapped_struct_name_turbofish =
                possibly_wrapped_struct_name_turbofish(
                    &struct_name,
                    &custom_wrapper,
                    Some(&ty_generics),
                );
            // auto-registration with inventory
            let inventory = inventory_tokens(
                &server_fn_path_token,
                &wrapped_struct_name_turbofish,
                &wrapped_struct_name,
            );

            let run_body = run_body_tokens(
                &custom_wrapper,
                &struct_name,
                &field_names,
                &dummy_name,
                &server_fn_path_token,
                &output_ty,
                &error_ty,
                has_marker,
                Some(&ty_generics_phantom_suffix_removed),
            );
            // the endpoint of our path should be specified in the register attribute if at all.
            let endpoint = Literal::string(
                &endpoint.map(|e| e.value()).unwrap_or_default(),
            );
            let path = path_tokens(
                &endpoint,
                &server_fn_path_token,
                &prefix,
                &fn_name_as_str,
                Some(&specific_ty),
            );

            let middlewares = middlewares_tokens(&middlewares);

            implementations_and_inventories.push(quote!(
                impl #server_fn_path_token::ServerFn for #wrapped_struct_name {
                    const PATH: &'static str = #path;

                    type Client = #client;
                    type ServerRequest = #req;
                    type ServerResponse = #res;
                    type Output = #output_ty;
                    type InputEncoding = #input_encoding;
                    type OutputEncoding = #output_encoding;
                    type Error = #error_ty;

                    fn middlewares() -> Vec<std::sync::Arc<dyn #server_fn_path_token::middleware::Layer<#req, #res>>> {
                        #middlewares
                    }

                    #run_body
                }

                #inventory
            ))
        }

        let traits_that_should_be_constraints =
            register.produce_trait_idents_that_should_be_constraints();
        let types_that_should_have_phantom_suffix =
            register.produce_type_idents_that_should_have_phantom_suffix();
        let mut g = body.generics.clone();
        for ty in g.type_params_mut() {
            if types_that_should_have_phantom_suffix.contains(&ty.ident) {
                ty.ident = Ident::new(
                    &format!("{}Phantom", &ty.ident),
                    ty.ident.span(),
                );
            }

            ty.bounds = ty
                .bounds
                .clone()
                .into_iter()
                .map(|ty| {
                    if let TypeParamBound::Trait(ref trait_bound) = ty {
                        if let Some(ident) = trait_bound.path.get_ident() {
                            if traits_that_should_be_constraints.contains(ident)
                            {
                                let ident = Ident::new(
                                    &format!("{ident}Constraint"),
                                    Span::call_site(),
                                );
                                let bound: TraitBound = parse_quote!(#ident);
                                TypeParamBound::Trait(bound)
                            } else {
                                ty
                            }
                        } else {
                            ty
                        }
                    } else {
                        ty
                    }
                })
                .collect::<Punctuated<TypeParamBound, Token![+]>>();
        }
        // this will include the defaults for the generic
        let params_with_trait_constraints = g.params.clone();
        let (impl_generics_with_trait_constraints, _, _) = g.split_for_impl();
        let fields = extend_fields_with_phantom_types(fields, phantom_types);
        let func = func_tokens(
            &docs,
            attrs,
            vis,
            fn_name,
            &fn_args,
            output_arrow,
            return_ty,
            &dummy_name,
            &field_names,
            &custom_wrapper,
            &struct_name,
            &server_fn_path_token,
            Some(&impl_generics),
            Some(&impl_generics_with_trait_constraints),
            Some(&ty_generics),
            where_clause,
            output_ty,
            &error_ty,
            has_marker,
        );

        Ok(quote!(
        #args_docs
        #docs
        #[derive(Debug, #derives)]
        #additional_path
        pub struct #struct_name < #params_with_trait_constraints > #where_clause {
            #(#fields),*
        }

        #(#implementations_and_inventories)*

        #func

        #dummy

        ))
    } else {
        // struct name, wrapped in any custom-encoding newtype wrapper (if it exists, otherwise it's just the struct name)
        let wrapped_struct_name =
            possibly_wrap_struct_name(&struct_name, &custom_wrapper, None);
        let wrapped_struct_name_turbofish =
            possibly_wrapped_struct_name_turbofish(
                &struct_name,
                &custom_wrapper,
                None,
            );

        let from_impl = impl_from_tokens(&body, impl_from, &struct_name);

        // auto-registration with inventory
        let inventory = inventory_tokens(
            &server_fn_path_token,
            &wrapped_struct_name_turbofish,
            &wrapped_struct_name,
        );

        // run_body in the trait implementation
        let run_body = run_body_tokens(
            &custom_wrapper,
            &struct_name,
            &field_names,
            &dummy_name,
            &server_fn_path_token,
            output_ty,
            &error_ty,
            false,
            None,
        );

        // the actual function definition
        let func = func_tokens(
            &docs,
            attrs,
            vis,
            fn_name,
            &fn_args,
            output_arrow,
            return_ty,
            &dummy_name,
            &field_names,
            &custom_wrapper,
            &struct_name,
            &server_fn_path_token,
            None,
            None,
            None,
            None,
            output_ty,
            &error_ty,
            false,
        );

        let path = path_tokens(
            &endpoint,
            &server_fn_path_token,
            &prefix,
            &fn_name_as_str,
            None,
        );

        let middlewares = middlewares_tokens(&middlewares);

        Ok(quote::quote! {
            #args_docs
            #docs
            #[derive(Debug, #derives)]
            #additional_path
            pub struct #struct_name {
                #(#fields),*
            }

            #from_impl

            impl #server_fn_path_token::ServerFn for #wrapped_struct_name {
                const PATH: &'static str = #path;

                type Client = #client;
                type ServerRequest = #req;
                type ServerResponse = #res;
                type Output = #output_ty;
                type InputEncoding = #input_encoding;
                type OutputEncoding = #output_encoding;
                type Error = #error_ty;

                fn middlewares() -> Vec<std::sync::Arc<dyn #server_fn_path_token::middleware::Layer<#req, #res>>> {
                    #middlewares
                }

                #run_body
            }

            #inventory

            #func

            #dummy
        })
    }
}

fn type_from_ident(ident: Ident) -> Type {
    let mut segments = Punctuated::new();
    segments.push(PathSegment {
        ident,
        arguments: PathArguments::None,
    });
    Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments,
        },
    })
}

#[derive(Debug)]
struct Middleware {
    expr: syn::Expr,
}

impl ToTokens for Middleware {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let expr = &self.expr;
        tokens.extend(quote::quote! {
            #expr
        });
    }
}

impl Parse for Middleware {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let arg: syn::Expr = input.parse()?;
        Ok(Middleware { expr: arg })
    }
}

#[derive(Debug, Clone)]
struct Register {
    /// Matches something like: (<Type1,...,TypeN> = "some_string", <Type1,...,TypeN>, ...)
    type_value_pairs: Punctuated<
        (Punctuated<InternalRegisterEntry, Token![,]>, Option<LitStr>),
        Token![,],
    >,
}

impl Register {
    fn produce_trait_idents_that_should_be_constraints(&self) -> Vec<Ident> {
        let mut result = Vec::new();
        for (entry, _) in self.type_value_pairs.iter() {
            for internal_entry in entry {
                if let Some(ref trait_list) = internal_entry.maybe_trait_list {
                    for trait_ident in trait_list {
                        let ident_str = trait_ident.to_string();
                        if ident_str.ends_with("Constraint")
                            && ident_str.len() > "Constraint".len()
                        {
                            result.push(Ident::new(
                                &ident_str
                                    [0..&ident_str.len() - "Constraint".len()],
                                trait_ident.span(),
                            ));
                        }
                    }
                }
            }
        }
        result
    }
    fn produce_type_idents_that_should_have_phantom_suffix(
        &self,
    ) -> Vec<Ident> {
        let mut result = Vec::new();
        for (entry, _) in self.type_value_pairs.iter() {
            for internal_entry in entry {
                let ty = internal_entry.specific_ty.clone();
                let ident_str = ty.to_string();
                if ident_str.ends_with("Phantom")
                    && ident_str.len() > "Phantom".len()
                {
                    result.push(Ident::new(
                        &ident_str[0..&ident_str.len() - "Phantom".len()],
                        ty.span(),
                    ));
                }
            }
        }
        result
    }
}
/// When we use PhantomTypes for our backends we can specify that they are over Trait constraints which are shadows for our ssr only traits.
/// ```rust,ignore
/// #[server]
/// #[register(<SsrOnlyTypePhantom:SsrOnlyTraitConstraint + SsrOnlyTrait2Constraint>)]
/// pub async fn example::<SsrOnlyType:SsrOnlyTrait + SsrOnlyTrait2 + Display>() -> Result<(),ServerFnError> {
///     // ...
/// }
/// ```
#[derive(Debug, Clone)]
struct InternalRegisterEntry {
    specific_ty: Ident,
    maybe_colon: Option<Token![:]>,
    maybe_trait_list: Option<Punctuated<Ident, Token![+]>>,
}
impl Parse for InternalRegisterEntry {
    fn parse(input: ParseStream) -> Result<Self> {
        let specific_ty = input.parse::<Ident>()?;
        let maybe_colon = if input.peek(Token![:]) {
            Some(input.parse::<Token![:]>()?)
        } else {
            None
        };
        if maybe_colon.is_none() {
            Ok(Self {
                specific_ty,
                maybe_colon,
                maybe_trait_list: None,
            })
        } else {
            let maybe_trait_list =
                Some(Punctuated::<Ident, Token![+]>::parse_separated_nonempty(
                    input,
                )?);
            Ok(Self {
                specific_ty,
                maybe_colon,
                maybe_trait_list,
            })
        }
    }
}
struct RegisterEntry {
    internal: Punctuated<InternalRegisterEntry, Token![,]>,
    endpoint: Option<LitStr>,
}
impl Register {
    fn as_registered_entries(&self) -> Vec<RegisterEntry> {
        self.type_value_pairs
            .clone()
            .into_iter()
            .map(|(internal, endpoint)| RegisterEntry { internal, endpoint })
            .collect()
    }
}
impl Parse for Register {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut pairs = Punctuated::new();

        while input.peek(Token![<]) {
            // Parse the angle bracketed ident list
            input.parse::<Token![<]>()?;
            let idents =
                Punctuated::<InternalRegisterEntry, Token![,]>::parse_separated_nonempty(
                    input,
                )?;
            input.parse::<Token![>]>()?;

            // Optionally parse `= "..."` if present
            let maybe_value = if input.peek(Token![=]) {
                input.parse::<Token![=]>()?;
                let lit = input.parse::<LitStr>()?;
                Some(lit)
            } else {
                None
            };

            pairs.push((idents, maybe_value));

            // If there's a comma, consume it and parse the next entry
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        let register = Register {
            type_value_pairs: pairs,
        };

        // ensure all brackets have the same len
        let expected_len = register
            .type_value_pairs
            .first()
            .map(|(p, _)| p.len())
            .unwrap_or(0);
        for (p, _) in &register.type_value_pairs {
            if p.len() != expected_len {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "All bracketed lists must have the same length in register",
                ));
            }
        }

        Ok(register)
    }
}

fn output_type(return_ty: &Type) -> Result<&GenericArgument> {
    if let syn::Type::Path(pat) = &return_ty {
        if pat.path.segments[0].ident == "Result" {
            if pat.path.segments.is_empty() {
                panic!("{:#?}", pat.path);
            } else if let PathArguments::AngleBracketed(args) =
                &pat.path.segments[0].arguments
            {
                return Ok(&args.args[0]);
            }
        }
    };

    Err(syn::Error::new(
        return_ty.span(),
        "server functions should return Result<T, ServerFnError> or Result<T, \
         ServerFnError<E>>",
    ))
}

/// In our ServerFn implementation, when we canonocalize the generic of our server function to our specific registed types
/// if one of those types appear as the T in Result<T,E> then we need to canonicalize the output type as well.
fn canonicalize_output_type_generic(
    generics: &Generics,
    specific_ty: &Punctuated<Ident, Token![,]>,
    output_type: &GenericArgument,
) -> GenericArgument {
    match output_type {
        GenericArgument::Lifetime(lifetime) => todo!(),
        GenericArgument::Type(ty) => {
            // if generics include the output type, then the output type is generic.
            if let Type::Path(type_path) = ty {
                if let Some(pos) = generics.type_params().position(|inner_ty| {
                    Some(&inner_ty.ident) == type_path.path.get_ident()
                }) {
                    let normal_form = specific_ty.get(pos).expect(
                        "Specific types should have length of generics",
                    );
                    GenericArgument::Type(parse_quote!(#normal_form))
                } else {
                    output_type.clone()
                }
            } else {
                output_type.clone()
            }
        }
        GenericArgument::Const(expr) => todo!(),
        GenericArgument::AssocType(assoc_type) => todo!(),
        GenericArgument::AssocConst(assoc_const) => todo!(),
        GenericArgument::Constraint(constraint) => todo!(),
        _ => todo!(),
    }
}

fn err_type(return_ty: &Type) -> Result<Option<&GenericArgument>> {
    if let syn::Type::Path(pat) = &return_ty {
        if pat.path.segments[0].ident == "Result" {
            if let PathArguments::AngleBracketed(args) =
                &pat.path.segments[0].arguments
            {
                // Result<T>
                if args.args.len() == 1 {
                    return Ok(None);
                }
                // Result<T, _>
                else if let GenericArgument::Type(Type::Path(pat)) =
                    &args.args[1]
                {
                    if let Some(segment) = pat.path.segments.last() {
                        if segment.ident == "ServerFnError" {
                            let args = &segment.arguments;
                            match args {
                                // Result<T, ServerFnError>
                                PathArguments::None => return Ok(None),
                                // Result<T, ServerFnError<E>>
                                PathArguments::AngleBracketed(args) => {
                                    if args.args.len() == 1 {
                                        return Ok(Some(&args.args[0]));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    };

    Err(syn::Error::new(
        return_ty.span(),
        "server functions should return Result<T, ServerFnError> or Result<T, \
         ServerFnError<E>>",
    ))
}

#[derive(Debug)]
struct ServerFnArgs {
    struct_name: Option<Ident>,
    prefix: Option<Literal>,
    input: Option<Type>,
    input_derive: Option<ExprTuple>,
    output: Option<Type>,
    endpoint: Option<Literal>,
    req_ty: Option<Type>,
    res_ty: Option<Type>,
    client: Option<Type>,
    custom_wrapper: Option<Path>,
    builtin_encoding: bool,
    impl_from: Option<LitBool>,
}

impl Parse for ServerFnArgs {
    fn parse(stream: ParseStream) -> syn::Result<Self> {
        // legacy 4-part arguments
        let mut struct_name: Option<Ident> = None;
        let mut prefix: Option<Literal> = None;
        let mut encoding: Option<Literal> = None;
        let mut endpoint: Option<Literal> = None;

        // new arguments: can only be keyed by name
        let mut input: Option<Type> = None;
        let mut input_derive: Option<ExprTuple> = None;
        let mut output: Option<Type> = None;
        let mut req_ty: Option<Type> = None;
        let mut res_ty: Option<Type> = None;
        let mut client: Option<Type> = None;
        let mut custom_wrapper: Option<Path> = None;
        let mut impl_from: Option<LitBool> = None;

        let mut use_key_and_value = false;
        let mut arg_pos = 0;

        while !stream.is_empty() {
            arg_pos += 1;
            let lookahead = stream.lookahead1();
            if lookahead.peek(Ident) {
                let key_or_value: Ident = stream.parse()?;

                let lookahead = stream.lookahead1();
                if lookahead.peek(Token![=]) {
                    stream.parse::<Token![=]>()?;
                    let key = key_or_value;
                    use_key_and_value = true;
                    if key == "name" {
                        if struct_name.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `name`",
                            ));
                        }
                        struct_name = Some(stream.parse()?);
                    } else if key == "prefix" {
                        if prefix.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `prefix`",
                            ));
                        }
                        prefix = Some(stream.parse()?);
                    } else if key == "encoding" {
                        if encoding.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `encoding`",
                            ));
                        }
                        encoding = Some(stream.parse()?);
                    } else if key == "endpoint" {
                        if endpoint.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `endpoint`",
                            ));
                        }
                        endpoint = Some(stream.parse()?);
                    } else if key == "input" {
                        if encoding.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "`encoding` and `input` should not both be \
                                 specified",
                            ));
                        } else if input.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `input`",
                            ));
                        }
                        input = Some(stream.parse()?);
                    } else if key == "input_derive" {
                        if input_derive.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `input_derive`",
                            ));
                        }
                        input_derive = Some(stream.parse()?);
                    } else if key == "output" {
                        if encoding.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "`encoding` and `output` should not both be \
                                 specified",
                            ));
                        } else if output.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `output`",
                            ));
                        }
                        output = Some(stream.parse()?);
                    } else if key == "req" {
                        if req_ty.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `req`",
                            ));
                        }
                        req_ty = Some(stream.parse()?);
                    } else if key == "res" {
                        if res_ty.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `res`",
                            ));
                        }
                        res_ty = Some(stream.parse()?);
                    } else if key == "client" {
                        if client.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `client`",
                            ));
                        }
                        client = Some(stream.parse()?);
                    } else if key == "custom" {
                        if custom_wrapper.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `custom`",
                            ));
                        }
                        custom_wrapper = Some(stream.parse()?);
                    } else if key == "impl_from" {
                        if impl_from.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `impl_from`",
                            ));
                        }
                        impl_from = Some(stream.parse()?);
                    } else {
                        return Err(lookahead.error());
                    }
                } else {
                    let value = key_or_value;
                    if use_key_and_value {
                        return Err(syn::Error::new(
                            value.span(),
                            "positional argument follows keyword argument",
                        ));
                    }
                    if arg_pos == 1 {
                        struct_name = Some(value)
                    } else {
                        return Err(syn::Error::new(
                            value.span(),
                            "expected string literal",
                        ));
                    }
                }
            } else if lookahead.peek(LitStr) {
                let value: Literal = stream.parse()?;
                if use_key_and_value {
                    return Err(syn::Error::new(
                        value.span(),
                        "If you use keyword arguments (e.g., `name` = \
                         Something), then you can no longer use arguments \
                         without a keyword.",
                    ));
                }
                match arg_pos {
                    1 => return Err(lookahead.error()),
                    2 => prefix = Some(value),
                    3 => encoding = Some(value),
                    4 => endpoint = Some(value),
                    _ => {
                        return Err(syn::Error::new(
                            value.span(),
                            "unexpected extra argument",
                        ))
                    }
                }
            } else {
                return Err(lookahead.error());
            }

            if !stream.is_empty() {
                stream.parse::<Token![,]>()?;
            }
        }

        // parse legacy encoding into input/output
        let mut builtin_encoding = false;
        if let Some(encoding) = encoding {
            match encoding.to_string().to_lowercase().as_str() {
                "\"url\"" => {
                    input = Some(type_from_ident(syn::parse_quote!(PostUrl)));
                    output = Some(type_from_ident(syn::parse_quote!(Json)));
                    builtin_encoding = true;
                }
                "\"cbor\"" => {
                    input = Some(type_from_ident(syn::parse_quote!(Cbor)));
                    output = Some(type_from_ident(syn::parse_quote!(Cbor)));
                    builtin_encoding = true;
                }
                "\"getcbor\"" => {
                    input = Some(type_from_ident(syn::parse_quote!(GetUrl)));
                    output = Some(type_from_ident(syn::parse_quote!(Cbor)));
                    builtin_encoding = true;
                }
                "\"getjson\"" => {
                    input = Some(type_from_ident(syn::parse_quote!(GetUrl)));
                    output = Some(type_from_ident(syn::parse_quote!(Json)));
                    builtin_encoding = true;
                }
                _ => {
                    return Err(syn::Error::new(
                        encoding.span(),
                        "Encoding not found.",
                    ))
                }
            }
        }

        Ok(Self {
            struct_name,
            prefix,
            input,
            input_derive,
            output,
            endpoint,
            builtin_encoding,
            req_ty,
            res_ty,
            client,
            custom_wrapper,
            impl_from,
        })
    }
}

#[derive(Debug)]
struct ServerFnBody {
    pub attrs: Vec<Attribute>,
    pub vis: syn::Visibility,
    pub async_token: Token![async],
    pub fn_token: Token![fn],
    pub ident: Ident,
    pub generics: Generics,
    pub _paren_token: token::Paren,
    pub inputs: Punctuated<FnArg, Token![,]>,
    pub output_arrow: Token![->],
    pub return_ty: syn::Type,
    pub block: TokenStream2,
    pub docs: Vec<(String, Span)>,
}

impl Parse for ServerFnBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attrs: Vec<Attribute> = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;

        let async_token = input.parse()?;

        let fn_token = input.parse()?;
        let ident = input.parse()?;
        let generics: Generics = input.parse()?;

        let content;
        let _paren_token = syn::parenthesized!(content in input);

        let inputs = syn::punctuated::Punctuated::parse_terminated(&content)?;

        let output_arrow = input.parse()?;
        let return_ty = input.parse()?;

        let block = input.parse()?;

        let docs = attrs
            .iter()
            .filter_map(|attr| {
                let Meta::NameValue(attr) = &attr.meta else {
                    return None;
                };
                if !attr.path.is_ident("doc") {
                    return None;
                }

                let value = match &attr.value {
                    syn::Expr::Lit(lit) => match &lit.lit {
                        syn::Lit::Str(s) => Some(s.value()),
                        _ => return None,
                    },
                    _ => return None,
                };

                Some((value.unwrap_or_default(), attr.path.span()))
            })
            .collect();
        attrs.retain(|attr| {
            let Meta::NameValue(attr) = &attr.meta else {
                return true;
            };
            !attr.path.is_ident("doc")
        });

        Ok(Self {
            vis,
            async_token,
            fn_token,
            ident,
            generics,
            _paren_token,
            inputs,
            output_arrow,
            return_ty,
            block,
            attrs,
            docs,
        })
    }
}

impl ServerFnBody {
    fn to_dummy_ident(&self) -> Ident {
        Ident::new(&format!("__{}", self.ident), self.ident.span())
    }

    /// The dummy is the original function that was annotated with `#[server]`.
    fn to_dummy_output(&self) -> TokenStream2 {
        let ident = self.to_dummy_ident();
        let Self {
            attrs,
            vis,
            async_token,
            fn_token,
            generics,
            inputs,
            output_arrow,
            return_ty,
            block,
            ..
        } = &self;
        // will all be empty if no generics...
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        quote! {
            #[doc(hidden)]
            #(#attrs)*
            #vis #async_token #fn_token #ident #impl_generics ( #inputs ) #output_arrow #return_ty #where_clause
            #block
        }
    }
}
