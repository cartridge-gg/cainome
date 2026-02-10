use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataStruct, Generics, Ident, Type};

pub fn derive_struct(ident: Ident, generics: Generics, data: DataStruct) -> TokenStream {
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
        fn cairo_deserialize(felt: &[::starknet::core::types::Felt], offset: usize) -> Result<Self::RustType, ::cainome_cairo_serde::Error> {
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

    let mut generic_contraint_list = vec![];
    for param in generics.type_params() {
        generic_contraint_list.push(quote! {
            #param: ::cainome_cairo_serde::CairoSerde<RustType = #param>
        });
    }

    let generic_contraints = if generic_contraint_list.is_empty() {
        quote! {}
    } else {
        quote! { where #(#generic_contraint_list),* }
    };

    // There is no easy way to check for the members being staticaly sized at compile time.
    // Any of the members of the composite type can have a dynamic size.
    // This is why we return `None` for the `SERIALIZED_SIZE` constant.
    let output = quote! {
        impl #generics ::cainome_cairo_serde::CairoSerde for #ident #generics
        #generic_contraints
        {
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
