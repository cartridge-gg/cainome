use proc_macro::{self};
use syn::{parse_macro_input, Data, DeriveInput};

mod derive_enum;
mod derive_struct;

#[proc_macro_derive(CairoSerde)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let output = match data {
        Data::Struct(data) => derive_struct::derive_struct(ident, data),
        Data::Enum(data) => derive_enum::derive_enum(ident, data),
        Data::Union(_) => panic!("Unions are not supported for the cairo_serde_derive!"),
    };

    output.into()
}
