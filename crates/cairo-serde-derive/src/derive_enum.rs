use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{DataEnum, Ident, Type, Variant};
use unzip_n::unzip_n;

pub fn derive_enum(ident: Ident, data: DataEnum) -> TokenStream {
    let matches = &data
        .variants
        .iter()
        .map(|v| derive_enum_matches(&ident, v))
        .collect::<Vec<_>>();

    unzip_n!(3);
    let (serialized_size, serialize, deserialize) = data
        .variants
        .iter()
        .enumerate()
        .map(|(i, v)| derive_enum_variant(&ident, i, v))
        .collect::<Vec<_>>()
        .into_iter()
        .unzip_n_vec();

    let cairo_serialized_size = quote! {
        fn cairo_serialized_size(rust: &Self::RustType) -> usize {
            match rust {
                #(
                    #matches => #serialized_size,
                )*
            }
        }
    };

    let cairo_serialize = quote! {
        fn cairo_serialize(rust: &Self::RustType) -> Vec<::starknet::core::types::Felt> {
            match rust {
                #(
                    #matches => #serialize,
                )*
            }
        }
    };

    let deserialize_matches = data
        .variants
        .iter()
        .enumerate()
        .map(|(i, _)| syn::LitInt::new(&i.to_string(), Span::call_site()))
        .collect::<Vec<_>>();
    let cairo_deserialize = quote! {
        fn cairo_deserialize(felt: &[Felt], offset: usize) -> Result<Self::RustType, ::cainome_cairo_serde::Error> {
            let offset = offset + 1;
            #(
                if felt[offset - 1] == ::starknet::core::types::Felt::from(#deserialize_matches) {
                    return Ok(#deserialize);
                }
            )*
            Err(::cainome_cairo_serde::Error::Deserialize("Invalid variant Id".to_string()))
        }
    };

    // There is no easy way to check for the members being staticaly sized at compile time.
    // Any of the members of the composite type can have a dynamic size.
    // This is why we return `None` for the `SERIALIZED_SIZE` constant.
    let output = quote! {
        impl ::cainome_cairo_serde::CairoSerde for #ident {
            type RustType = Self;

            const SERIALIZED_SIZE: Option<usize> = None;

            #cairo_serialized_size
            #cairo_serialize
            #cairo_deserialize
        }
    };
    output
}

fn derive_enum_matches(ident: &Ident, variant: &Variant) -> TokenStream {
    let variant_ident = variant.ident.clone();
    let (fields, _) = fields_idents_and_types(&variant.fields);
    match &variant.fields {
        syn::Fields::Named(_) => quote! {
            #ident::#variant_ident { #(#fields,)* }
        },
        syn::Fields::Unnamed(_) => quote! {
            #ident::#variant_ident(#(#fields,)*)
        },
        syn::Fields::Unit => quote! {
            #ident::#variant_ident
        },
    }
}

fn derive_enum_variant(
    ident: &Ident,
    index: usize,
    variant: &Variant,
) -> (TokenStream, TokenStream, TokenStream) {
    let (fields, types) = fields_idents_and_types(&variant.fields);
    (
        derive_variant_cairo_serialized_size(&fields, &types),
        derive_variant_cairo_serialize(index, &fields, &types),
        derive_variant_cairo_deserialize(ident, variant, &fields, &types),
    )
}

fn derive_variant_cairo_serialized_size(fields: &[TokenStream], types: &[Type]) -> TokenStream {
    quote! {
        {
            1
            #(
                + <#types as ::cainome_cairo_serde::CairoSerde>::cairo_serialized_size(&#fields)
            )*
        }
    }
}

fn derive_variant_cairo_serialize(
    index: usize,
    fields: &[TokenStream],
    types: &[Type],
) -> TokenStream {
    let index = syn::LitInt::new(&index.to_string(), Span::call_site());
    quote! {
        {
            let mut result = Vec::new();
            result.push(::starknet::core::types::Felt::from(#index));
            #(
                result.extend(<#types as ::cainome_cairo_serde::CairoSerde>::cairo_serialize(&#fields));
            )*
            result
        }
    }
}

fn derive_variant_cairo_deserialize(
    ident: &Ident,
    variant: &Variant,
    fields: &[TokenStream],
    types: &[Type],
) -> TokenStream {
    let variant_ident = &variant.ident;

    match &variant.fields {
        syn::Fields::Named(_) => quote! {
            {
                let mut current_offset = offset;
                #ident::#variant_ident {
                    #(
                        #fields: {
                            let value = <#types as ::cainome_cairo_serde::CairoSerde>::cairo_deserialize(felt, current_offset)?;
                            current_offset += <#types as ::cainome_cairo_serde::CairoSerde>::cairo_serialized_size(&value);
                            value
                        },
                    )*
                }
            }
        },
        syn::Fields::Unnamed(_) => quote! {
            {
                let mut current_offset = offset;
                #ident::#variant_ident (
                    #(
                        {
                            let value = <#types as ::cainome_cairo_serde::CairoSerde>::cairo_deserialize(felt, current_offset)?;
                            current_offset += <#types as ::cainome_cairo_serde::CairoSerde>::cairo_serialized_size(&value);
                            value
                        },
                    )*
                )
            }
        },
        syn::Fields::Unit => quote! { #ident::#variant_ident},
    }
}

fn fields_idents_and_types(fields: &syn::Fields) -> (Vec<TokenStream>, Vec<Type>) {
    fields
        .iter()
        .cloned()
        .enumerate()
        .map(field_ident_and_type)
        .unzip()
}

fn field_ident_and_type((i, field): (usize, syn::Field)) -> (TokenStream, Type) {
    (
        field
            .ident
            .clone()
            .map(|ident| quote! { #ident })
            .unwrap_or({
                let i = syn::Ident::new(&format!("__self_{}", i), Span::call_site());
                quote! { #i }
            }),
        field.ty,
    )
}
