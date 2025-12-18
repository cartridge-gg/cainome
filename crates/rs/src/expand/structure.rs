use crate::expand::{
    types::{get_additional_derive_requirements, CairoToRust},
    utils, Expandable, ExpansionContext, ExpansionResult,
};
use cainome_parser::tokens::{NamedToken, Struct};
use proc_macro2::TokenStream;
use quote::quote;

// TODO: create Structure struct with type_name and variants and From<Enum> and From<Event> trait implementation.

pub fn struct_declaration(
    type_name: &str,
    fields: &Vec<NamedToken>,
    ctx: &ExpansionContext,
) -> TokenStream {
    let _ = ctx;

    let struct_name = utils::str_to_ident(&type_name);

    let mut members: Vec<TokenStream> = vec![];
    for inner in fields {
        let name = utils::str_to_ident(&inner.name);
        let token = &*inner.token.borrow();
        let ty = utils::str_to_type(&token.to_rust_type(ctx));

        let serde = utils::serde_hex_derive(&token.to_rust_type(ctx));

        members.push(quote!(#serde pub #name: #ty));
    }

    let mut internal_derives = vec![];

    for d in ctx.derives.iter() {
        if d.to_lowercase() == "serde" {
            continue;
        }
        internal_derives.push(utils::str_to_type(d));
    }

    let derive = if internal_derives.len() > 0 {
        quote! { #[derive(#(#internal_derives,)*)] }
    } else {
        quote! {}
    };

    quote! {
        #derive
        pub struct #struct_name {
            #(#members),*
        }
    }
}

pub fn struct_implementation(
    type_name: &str,
    fields: &Vec<NamedToken>,
    ctx: &ExpansionContext,
) -> TokenStream {
    let struct_name = utils::str_to_ident(&type_name);

    let mut sizes: Vec<TokenStream> = vec![];
    let mut sers: Vec<TokenStream> = vec![];
    let mut desers: Vec<TokenStream> = vec![];
    let mut names: Vec<TokenStream> = vec![];

    for inner in fields {
        let name = utils::str_to_ident(&inner.name);
        let token = &*inner.token.borrow();
        let ty = utils::str_to_type(&token.to_rust_type_path(ctx));

        // Tuples type used as rust type path item path must be surrounded
        // by angle brackets.

        let ty_punctuated = if inner.token.borrow().is_tuple() {
            quote!(<#ty>)
        } else {
            quote!(#ty)
        };

        names.push(quote!(#name));

        sizes.push(quote! {
            __size += #ty_punctuated::cairo_serialized_size(&__rust.#name);
        });

        sers.push(quote!(__out.extend(#ty_punctuated::cairo_serialize(&__rust.#name));));

        desers.push(quote! {
            let #name = #ty_punctuated::cairo_deserialize(__felts, __offset)?;
            __offset += #ty_punctuated::cairo_serialized_size(&#name);
        });
    }

    let ccs = utils::cainome_cairo_serde();
    let snrs_types = utils::snrs_types();

    let (impl_line, rust_type) = (
        quote!(impl #ccs::CairoSerde for #struct_name),
        quote!(
            type RustType = Self;
        ),
    );

    quote! {
        #impl_line {

            #rust_type

            const SERIALIZED_SIZE: std::option::Option<usize> = None;

            #[inline]
            fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
                let mut __size = 0;
                #(#sizes)*
                __size
            }

            fn cairo_serialize(__rust: &Self::RustType) -> Vec<#snrs_types::Felt> {
                let mut __out: Vec<#snrs_types::Felt> = vec![];
                #(#sers)*
                __out
            }

            fn cairo_deserialize(__felts: &[#snrs_types::Felt], __offset: usize) -> #ccs::Result<Self::RustType> {
                let mut __offset = __offset;
                #(#desers)*
                Ok(#struct_name {
                    #(#names),*
                })
            }
        }
    }
}

impl Expandable for Struct {
    fn expand(&self, ctx: &ExpansionContext) -> Vec<ExpansionResult> {
        let full_path = self.type_path_no_generic();
        let name = full_path.split("::").last().unwrap().to_owned();

        let ctx = ctx
            .clone()
            .with_derives(get_additional_derive_requirements(&self.fields, &ctx));

        let declaration = struct_declaration(&name, &self.fields, &ctx);
        let implementation = struct_implementation(&name, &self.fields, &ctx);

        let item = quote! {
            #declaration

            #implementation
        };

        vec![
            ExpansionResult::new(&full_path).with_item(&name, item), // .with_imports(deps)
        ]
    }
}
