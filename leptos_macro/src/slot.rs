use crate::{
    component::{convert_from_snake_case, drain_filter},
    util::{
        documentation::{
            generate_prop_documentation, prop_to_doc, Docs,
            PropDocumentationStyle,
        },
        generate_companion_internals, type_analysis,
        typed_builder_opts::TypedBuilderOpts,
        CompanionConfig, CompanionModuleBody, PropLike,
    },
};
use attribute_derive::FromAttr;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{
    parse::Parse, parse_quote, Field, ItemStruct, LitStr, Meta, Type,
    Visibility,
};

pub struct Model {
    docs: Docs,
    vis: Visibility,
    name: Ident,
    props: Vec<SlotProp>,
    body: ItemStruct,
}

impl Parse for Model {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut item = ItemStruct::parse(input)?;

        let docs = Docs::new(&item.attrs);

        let props = item
            .fields
            .clone()
            .into_iter()
            .map(SlotProp::new)
            .collect::<Vec<_>>();

        // We need to remove the `#[doc = ""]` and `#[builder(_)]`
        // attrs from the function signature
        drain_filter(&mut item.attrs, |attr| match &attr.meta {
            Meta::NameValue(attr) => attr.path == parse_quote!(doc),
            Meta::List(attr) => attr.path == parse_quote!(prop),
            _ => false,
        });
        item.fields.iter_mut().for_each(|arg| {
            drain_filter(&mut arg.attrs, |attr| match &attr.meta {
                Meta::NameValue(attr) => attr.path == parse_quote!(doc),
                Meta::List(attr) => attr.path == parse_quote!(prop),
                _ => false,
            });
        });

        Ok(Self {
            docs,
            vis: item.vis.clone(),
            name: convert_from_snake_case(&item.ident),
            props,
            body: item,
        })
    }
}

impl ToTokens for Model {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            docs,
            vis,
            name,
            props,
            body,
        } = self;

        let (_, generics, _) = body.generics.split_for_impl();

        let original_generics = &body.generics;

        let field_types: Vec<&Type> = props.iter().map(|p| &p.ty).collect();
        let struct_generics = type_analysis::strip_non_structural_bounds(
            &body.generics,
            &field_types,
        );
        let (struct_impl_generics, _, struct_where_clause) =
            struct_generics.split_for_impl();

        let phantom_type_params = type_analysis::find_unused_type_params(
            &body.generics,
            &field_types,
        );
        let phantom_field =
            type_analysis::generate_phantom_field(&phantom_type_params, false);

        let prop_builder_fields = prop_builder_fields(props);
        let prop_docs = generate_prop_documentation(props);
        let builder_name_doc = LitStr::new(
            &format!("Props for the [`{name}`] slot."),
            name.span(),
        );

        // Module name for the companion module: prefixed with `__`
        // to avoid namespace conflicts with the struct.
        let companion_name = format_ident!("__{}", name);

        let slot_builder_name = format_ident!("{}Builder", name);

        let CompanionModuleBody {
            module_items,
            trait_impls,
            helper_constructor_arg,
        } = generate_companion_internals(&CompanionConfig {
            original_generics,
            stripped_generics: &struct_generics,
            module_name: &companion_name,
            display_name: name,
            kind: "slot",
            props_name: name,
            props: &props,
        });

        let output = quote! {
            // Companion module — contains the slot struct, internal
            // types and traits (wrapper structs, check marker traits,
            // prop presence tracker, and the `Helper` struct).
            //
            // INVARIANT: The view macro NEVER references this module by
            // name. All view-macro-generated code goes through
            // `SlotName::__slot()` (which returns a `Helper`). This
            // means the module name (`__SlotName`) does not need to
            // follow renamed imports — only the struct does.
            #[doc(hidden)]
            #[allow(non_snake_case)]
            #vis mod #companion_name {
                #[allow(unused_imports)]
                use super::*;

                #[doc = #builder_name_doc]
                #[doc = ""]
                #docs
                #prop_docs
                #[derive(::leptos::typed_builder_macro::TypedBuilder)]
                #[builder(doc, crate_module_path=::leptos::typed_builder)]
                pub struct #name #struct_impl_generics #struct_where_clause {
                    #prop_builder_fields
                    #phantom_field
                }

                impl #struct_impl_generics ::leptos::component::Props for #name #generics #struct_where_clause {
                    type Builder = #slot_builder_name #generics;

                    fn builder() -> Self::Builder {
                        #name::builder()
                    }
                }

                impl #struct_impl_generics From<#name #generics> for Vec<#name #generics> #struct_where_clause {
                    fn from(value: #name #generics) -> Self {
                        vec![value]
                    }
                }

                #module_items
            }

            #[allow(unused_imports)]
            #vis use #companion_name::#name;
            #[allow(unused_imports)]
            #vis use #companion_name::#slot_builder_name;

            #trait_impls

            // Single inherent method on the slot struct. The view
            // macro calls `SlotName::__slot()` to get a `Helper`
            // that provides builder, presence, and check methods.
            // This follows renamed imports because `SlotName::` does.
            #[doc(hidden)]
            impl #struct_impl_generics #name #generics #struct_where_clause {
                pub fn __slot() -> #companion_name::Helper #generics {
                    #companion_name::Helper(#helper_constructor_arg)
                }
            }
        };

        tokens.append_all(output)
    }
}

struct SlotProp {
    docs: Docs,
    options: SlotPropOptions,
    name: Ident,
    ty: Type,
}

impl PropLike for SlotProp {
    fn name(&self) -> &Ident {
        &self.name
    }
    fn ty(&self) -> &Type {
        &self.ty
    }
    fn docs(&self) -> &Docs {
        &self.docs
    }
    fn is_optional(&self) -> bool {
        self.options.is_optional()
    }
    fn optional(&self) -> bool {
        self.options.optional
    }
    fn strip_option(&self) -> bool {
        self.options.strip_option
    }
    fn into_prop(&self) -> bool {
        self.options.into
    }
    fn default(&self) -> Option<&syn::Expr> {
        self.options.default.as_ref()
    }
}

impl SlotProp {
    fn new(arg: Field) -> Self {
        let prop_opts = SlotPropOptions::from_attributes(&arg.attrs)
            .unwrap_or_else(|e| {
                // TODO: replace with `.unwrap_or_abort()` once https://gitlab.com/CreepySkeleton/proc-macro-error/-/issues/17 is fixed
                abort!(e.span(), e.to_string());
            });

        let name = if let Some(i) = arg.ident {
            i
        } else {
            abort!(
                arg.ident,
                "only `prop: bool` style types are allowed within the \
                 `#[slot]` macro"
            );
        };

        Self {
            docs: Docs::new(&arg.attrs),
            options: prop_opts,
            name,
            ty: arg.ty,
        }
    }
}

#[derive(Clone, Debug, FromAttr)]
#[attribute(ident = prop)]
struct SlotPropOptions {
    #[attribute(conflicts = [optional_no_strip, strip_option])]
    pub optional: bool,
    #[attribute(conflicts = [optional, strip_option])]
    pub optional_no_strip: bool,
    #[attribute(conflicts = [optional, optional_no_strip])]
    pub strip_option: bool,
    #[attribute(example = "5 * 10")]
    pub default: Option<syn::Expr>,
    pub into: bool,
    pub attrs: bool,
}

impl SlotPropOptions {
    fn is_optional(&self) -> bool {
        self.optional
            || self.optional_no_strip
            || self.attrs
            || self.default.is_some()
    }
}

fn prop_builder_fields(props: &[SlotProp]) -> TokenStream {
    props
        .iter()
        .map(|prop| {
            let builder_attrs = TypedBuilderOpts::from_prop(prop);
            let builder_docs =
                prop_to_doc(prop, PropDocumentationStyle::Inline);

            let docs = prop.docs();
            let name = &prop.name;
            let ty = prop.ty();

            quote! {
                #docs
                #builder_docs
                #builder_attrs
                pub #name: #ty,
            }
        })
        .collect()
}
