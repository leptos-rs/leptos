use crate::util::documentation::Docs;
use crate::util::property_documentation;
use crate::util::property_documentation::{
    prop_to_doc, PropDocumentationInput, PropDocumentationStyle,
};
use crate::util::typed_builder_opts::TypedBuilderOpts;
use crate::{
    component::{convert_from_snake_case, drain_filter},
    util::{
        generate_module_builder, generate_module_checks,
        generate_module_presence_check, generate_module_required_check,
        type_analysis, ModuleCheckTokens, ModulePresenceTokens,
        ModuleRequiredCheckTokens,
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
        let behavioral_bounds_stripped_generics =
            type_analysis::strip_non_structural_bounds(
                &body.generics,
                &field_types,
            );
        let (struct_impl_generics, _, struct_where_clause) =
            behavioral_bounds_stripped_generics.split_for_impl();

        let phantom_type_params = type_analysis::collect_phantom_type_params(
            &body.generics,
            &field_types,
        );
        let phantom_field =
            type_analysis::generate_phantom_field(&phantom_type_params, false);

        let prop_builder_fields = prop_builder_fields(vis, props);
        let doc_inputs: Vec<PropDocumentationInput> = props
            .iter()
            .map(|p| PropDocumentationInput {
                docs: &p.docs,
                name: &p.name,
                ty: &p.ty,
                is_optional: p.options.is_optional(),
                optional: p.options.optional,
                strip_option: p.options.strip_option,
                into: p.options.into,
            })
            .collect();
        let prop_docs =
            property_documentation::generate_prop_documentation(&doc_inputs);
        let builder_name_doc = LitStr::new(
            &format!("Props for the [`{name}`] slot."),
            name.span(),
        );

        // Module name for the companion module: prefixed with `__`
        // to avoid namespace conflicts with the struct.
        let module_name = format_ident!("__{}", name);

        let prop_pairs: Vec<(&Ident, &Type)> =
            props.iter().map(|p| (&p.name, &p.ty)).collect();
        let ModuleCheckTokens {
            module_check_traits,
            check_trait_impls,
        } = generate_module_checks(
            original_generics,
            &module_name,
            name,
            "slot",
            &prop_pairs,
            &field_types,
        );

        let no_props = props.is_empty();
        let slot_builder_name = format_ident!("{}Builder", name);
        let required_fields: Vec<(&Ident, bool, &Type)> = props
            .iter()
            .map(|p| {
                let required = !p.options.is_optional();
                (&p.name, required, &p.ty)
            })
            .collect();
        let ModuleRequiredCheckTokens { marker_traits } =
            generate_module_required_check(name, "slot", &required_fields);

        let ModulePresenceTokens {
            module_items: module_presence_items,
            check_presence_impl,
        } = generate_module_presence_check(
            &module_name,
            name,
            &required_fields,
        );

        let module_builder = generate_module_builder(
            no_props,
            &behavioral_bounds_stripped_generics,
            name,
        );

        let output = quote! {
            #[doc = #builder_name_doc]
            #[doc = ""]
            #docs
            #prop_docs
            #[derive(::leptos::typed_builder_macro::TypedBuilder)]
            #[builder(doc, crate_module_path=::leptos::typed_builder)]
            #vis struct #name #struct_impl_generics #struct_where_clause {
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

            #marker_traits

            // Companion module — uses `__` prefix since modules and structs share the `type`
            // namespace.
            #[doc(hidden)]
            #[allow(non_snake_case)]
            #vis mod #module_name {
                #[allow(unused_imports)]
                use super::*;
                #module_builder
                #(#module_check_traits)*
                #module_presence_items
            }

            #(#check_trait_impls)*
            #check_presence_impl
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

fn prop_builder_fields(vis: &Visibility, props: &[SlotProp]) -> TokenStream {
    props
        .iter()
        .map(|prop| {
            let SlotProp {
                docs,
                name,
                options,
                ty,
            } = prop;

            let builder_attrs = TypedBuilderOpts::new(
                options.is_optional(),
                &options.default,
                options.strip_option,
                options.optional,
                options.into,
                ty,
            );

            let builder_documentation = prop_to_doc(
                &PropDocumentationInput {
                    docs,
                    name,
                    ty,
                    is_optional: options.is_optional(),
                    optional: options.optional,
                    strip_option: options.strip_option,
                    into: options.into,
                },
                PropDocumentationStyle::Inline,
            );

            quote! {
                #docs
                #builder_documentation
                #builder_attrs
                #vis #name: #ty,
            }
        })
        .collect()
}
