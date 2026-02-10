use std::collections::{HashMap, HashSet};

use cainome_parser::tokens::{genericity, Enum, NamedToken, Token, TypePath};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::expand::genericity::{get_generic_args_fields, resolve_generics};
use crate::expand::types::{get_additional_derive_requirements, CairoToRust};
use crate::expand::{
    utils, Expandable, ExpansionContext, ExpansionContextFactory, ExpansionResult,
};

pub fn enum_declaration(
    type_name: &str,
    variants: &[NamedToken],
    generic_arg_names: &Vec<Ident>,
    fields_to_generics: &HashMap<String, HashSet<String>>,
    ctx: &ExpansionContext,
) -> TokenStream {
    let enum_name = utils::str_to_ident(type_name);
    let is_generic = !generic_arg_names.is_empty();
    let mut generated_variants: Vec<TokenStream> = vec![];

    for inner in variants {
        let name = utils::str_to_ident(&inner.name);
        let token = &*inner.token.borrow();

        let generic_type = if is_generic {
            resolve_generics(type_name, inner, fields_to_generics, ctx)
        } else {
            token.to_rust_type_path(ctx)
        };

        let ty = utils::str_to_type(&generic_type);

        let serde = utils::serde_hex_derive(&generic_type, ctx);

        match &*inner.token.borrow() {
            Token::Basic(TypePath { type_path }) if type_path == "()" => {
                generated_variants.push(quote!(#serde #name));
            }
            _ => {
                generated_variants.push(quote!(#serde #name(#ty)));
            }
        }
    }

    let mut internal_derives = vec![];

    for d in ctx.derives.iter() {
        internal_derives.push(utils::str_to_type(d));
    }

    let derive = if !internal_derives.is_empty() {
        quote! { #[derive(#(#internal_derives,)*)] }
    } else {
        quote! {}
    };

    let generic_args = if is_generic {
        quote! (<#(#generic_arg_names),*>)
    } else {
        quote!()
    };

    quote! {
        #derive

        pub enum #enum_name #generic_args {
            #(#generated_variants),*
        }
    }
}

pub fn enum_implementation(
    type_name: &str,
    variants: &[NamedToken],
    generic_arg_names: &Vec<Ident>,
    fields_to_generics: &HashMap<String, HashSet<String>>,
    ctx: &ExpansionContext,
) -> TokenStream {
    let enum_name = utils::str_to_ident(type_name);
    let enum_name_str = utils::str_to_litstr(type_name);
    let is_generic = !generic_arg_names.is_empty();

    let mut serialized_sizes: Vec<TokenStream> = vec![];
    let mut serializations: Vec<TokenStream> = vec![];
    let mut deserializations: Vec<TokenStream> = vec![];

    for (variant_index, inner) in variants.iter().enumerate() {
        let variant_name = utils::str_to_ident(&inner.name);
        let token = &*inner.token.borrow();

        let generic_type_path = if is_generic {
            // Calculate default value (in case resolver won't work)
            let default_generic_type = fields_to_generics
                // Check if generic candidates exist for the field
                .get(&inner.name)
                .unwrap_or(&HashSet::new())
                .iter()
                .next()
                .cloned()
                // if not then use type from ABI
                .unwrap_or(token.to_rust_type(ctx));

            ctx.generic_resolver
                .resolve_generic_member(type_name, inner, ctx)
                .unwrap_or(default_generic_type)
        } else {
            token.to_rust_type(ctx)
        };

        let ty = utils::str_to_type(&generic_type_path);

        // Tuples type used as rust type path must be surrounded
        // by angle brackets.
        let ty_punctuated = if inner.token.borrow().is_tuple() {
            quote!(<#ty>)
        } else {
            quote!(#ty)
        };

        match &*inner.token.borrow() {
            Token::Basic(TypePath { type_path }) if type_path == "()" => {
                serializations.push(quote! {
                    #enum_name::#variant_name => usize::cairo_serialize(&#variant_index)
                });
                deserializations.push(quote! {
                    #variant_index => Ok(#enum_name::#variant_name)
                });
                serialized_sizes.push(quote! {
                    #enum_name::#variant_name => 1
                });
            }
            _ => {
                serializations.push(quote! {
                    #enum_name::#variant_name(val) => {
                        let mut temp = vec![];
                        temp.extend(usize::cairo_serialize(&#variant_index));
                        temp.extend(#ty_punctuated::cairo_serialize(val));
                        temp
                    }
                });
                deserializations.push(quote! {
                    #variant_index => Ok(#enum_name::#variant_name(#ty_punctuated::cairo_deserialize(__felts, __offset + 1)?))
                });
                // +1 because we have to handle the variant index also.
                serialized_sizes.push(quote! {
                    #enum_name::#variant_name(val) => #ty_punctuated::cairo_serialized_size(val) + 1
                })
            }
        }
    }

    let ccs = utils::str_to_type(&ctx.cainome_serde_path);

    serialized_sizes.push(quote! {
        _ => 0
    });

    serializations.push(quote! {
        _ => vec![]
    });

    deserializations.push(quote! {
        _ => return Err(#ccs::Error::Deserialize(format!("Index not handle for enum {}", #enum_name_str)))
    });

    let (generic_args, generic_where) = if is_generic {
        (
            quote! (<#(#generic_arg_names),*>),
            quote! ( where #(#generic_arg_names: #ccs::CairoSerde<RustType = #generic_arg_names>),*),
        )
    } else {
        (quote!(), quote!())
    };

    let (impl_line, rust_type) = (
        quote!(impl #generic_args #ccs::CairoSerde for #enum_name #generic_args #generic_where),
        quote!(
            type RustType = Self;
        ),
    );

    quote! {
        #impl_line {

            #rust_type

            const SERIALIZED_SIZE: std::option::Option<usize> = std::option::Option::None;

            #[inline]
            fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
                match __rust {
                    #(#serialized_sizes),*
                }
            }

            fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
                match __rust {
                    #(#serializations),*
                }
            }

            fn cairo_deserialize(__felts: &[starknet::core::types::Felt], __offset: usize) -> #ccs::Result<Self::RustType> {
                let __f = __felts[__offset];
                let __index = u128::from_be_bytes(__f.to_bytes_be()[16..].try_into().unwrap());

                match __index as usize {
                    #(#deserializations),*
                }

            }
        }
    }
}

impl Expandable for Enum {
    fn expand(&self, ctx: &ExpansionContext) -> Vec<super::ExpansionResult> {
        let full_path = ctx.apply_alias(&self.type_path);
        let full_path_no_generic = genericity::type_path_no_generic(&full_path);
        let name = &full_path_no_generic.split("::").last().unwrap().to_owned();

        let ctx = ExpansionContextFactory::from(ctx)
            .with_derives(get_additional_derive_requirements(self.get_variants(), ctx))
            .build();

        let generic_arg_names = get_generic_args_fields(self.generic_args);

        &self
            .generic_args
            .iter()
            .map(|(name, _)| utils::str_to_ident(name))
            .collect::<Vec<_>>();

        let declaration = enum_declaration(
            name,
            self.get_variants(),
            generic_arg_names,
            &self.fields_to_generics,
            &ctx,
        );

        let implementation = enum_implementation(
            name,
            self.get_variants(),
            generic_arg_names,
            &self.fields_to_generics,
            &ctx,
        );

        let item = quote! {
            #declaration

            #implementation
        };

        vec![ExpansionResult::new(&full_path_no_generic).with_item(name, item)]
    }
}
