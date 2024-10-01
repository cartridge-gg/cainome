use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataStruct, Ident, Type};

pub fn derive_struct(ident: Ident, data: DataStruct) -> TokenStream {
    let (fields, types) = fields_accessors_and_types(&data.fields);

    let cairo_serialized_size = quote! {
        fn cairo_serialized_size(rust: &Self::RustType) -> usize {
            0
            #(
                + <#types as ::cainome_cairo_serde::CairoSerde>::cairo_serialized_size(&rust.#fields)
            )*
        }
    };

    let cairo_serialize = quote! {
        fn cairo_serialize(rust: &Self::RustType) -> Vec<::starknet::core::types::Felt> {
            let mut result = Vec::new();
            #(
                result.extend(<#types as ::cainome_cairo_serde::CairoSerde>::cairo_serialize(&rust.#fields));
            )*
            result
        }
    };

    let cairo_deserialize = quote! {
        fn cairo_deserialize(felt: &[Felt], offset: usize) -> Result<Self::RustType, ::cainome_cairo_serde::Error> {
            let mut current_offset = offset;
            Ok(Self {
                #(
                    #fields: {
                        let value = <#types as ::cainome_cairo_serde::CairoSerde>::cairo_deserialize(felt, current_offset)?;
                        current_offset += <#types as ::cainome_cairo_serde::CairoSerde>::cairo_serialized_size(&value);
                        value
                    },
                )*
            })
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

fn fields_accessors_and_types(fields: &syn::Fields) -> (Vec<TokenStream>, Vec<Type>) {
    fields
        .iter()
        .cloned()
        .enumerate()
        .map(field_accessor_and_type)
        .unzip()
}

fn field_accessor_and_type((i, field): (usize, syn::Field)) -> (TokenStream, Type) {
    (
        field
            .ident
            .clone()
            .map(|ident| quote! { #ident })
            .unwrap_or({
                let i = syn::Index::from(i);
                quote! { #i }
            }),
        field.ty,
    )
}
