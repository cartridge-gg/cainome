use proc_macro::{self};
use syn::{parse_macro_input, Data, DeriveInput, Generics};

mod derive_enum;
mod derive_struct;

#[proc_macro_derive(CairoSerde)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let DeriveInput {
        ident,
        generics,
        data,
        ..
    } = parse_macro_input!(input);

    let output = match data {
        Data::Struct(data) => derive_struct::derive_struct(ident, generics, data),
        Data::Enum(data) => derive_enum::derive_enum(ident, generics, data),
        Data::Union(_) => panic!("Unions are not supported for the cairo_serde_derive!"),
    };

    output.into()
}
