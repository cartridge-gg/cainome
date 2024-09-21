use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Ident, Type};

#[proc_macro_derive(CairoSerde)]
pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let (idents, types): (Vec<Ident>, Vec<Type>) = match data {
        Data::Struct(data) => data
            .fields
            .iter()
            .cloned()
            .map(|field| (field.ident.clone().unwrap(), field.ty))
            .unzip(),
        _ => panic!("CairoSerde can only be derived for structs"),
    };

    let cairo_serialized_size = quote! {
        fn cairo_serialized_size(rust: &Self::RustType) -> usize {
            0
            #(
                + <#types as ::cainome_cairo_serde::CairoSerde>::cairo_serialized_size(&rust.#idents)
            )*
        }
    };

    let cairo_serialize = quote! {
        fn cairo_serialize(rust: &Self::RustType) -> Vec<Felt> {
            let mut result = Vec::new();
            #(
                result.extend(<#types as ::cainome_cairo_serde::CairoSerde>::cairo_serialize(&rust.#idents));
            )*
            result
        }
    };

    let cairo_deserialize = quote! {
        fn cairo_deserialize(felt: &[Felt], offset: usize) -> Result<Self::RustType, ::cainome_cairo_serde::Error> {
            let mut current_offset = offset;
            Ok(Self {
                #(
                    #idents: {
                        let value = <#types as ::cainome_cairo_serde::CairoSerde>::cairo_deserialize(felt, current_offset)?;
                        current_offset += <#types as ::cainome_cairo_serde::CairoSerde>::cairo_serialized_size(&value);
                        value
                    },
                )*
            })
        }
    };

    let output = quote! {
        impl CairoSerde for #ident {
            type RustType = Self;

            #cairo_serialized_size
            #cairo_serialize
            #cairo_deserialize
        }
    };
    output.into()
}
