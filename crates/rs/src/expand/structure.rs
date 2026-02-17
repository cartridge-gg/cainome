use std::collections::HashSet;

use crate::expand::{
    generic_resolver::GenericResolveResult,
    types::{get_additional_derive_requirements, CairoToRust},
    utils, Expandable, ExpansionContext, ExpansionContextFactory, ExpansionResult,
};
use cainome_parser::{
    tokens::{genericity, NamedToken, Struct},
    CainomeResult,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub fn struct_declaration(
    full_path: &str,
    type_name: &str,
    fields: &Vec<NamedToken>,
    generic_arg_names: &Vec<Ident>,
    ctx: &ExpansionContext,
) -> CainomeResult<TokenStream> {
    tracing::trace!("Generating struct declaration for {}", type_name);

    let struct_name = utils::str_to_ident(type_name);
    let is_generic = !generic_arg_names.is_empty();
    let mut members: Vec<TokenStream> = vec![];
    let mut resolved_generic_args = HashSet::new();

    for inner in fields {
        let name = utils::str_to_ident(&inner.name);
        let token = &*inner.token.borrow();

        let generic_type = if is_generic {
            let resolved_result = ctx
                .generic_resolver
                .resolve_generic_member(full_path, inner, ctx);

            if let GenericResolveResult::Resolved(Some(arg)) = resolved_result {
                resolved_generic_args.insert(arg.clone());
                arg
            } else {
                token.to_rust_type_path(ctx)
            }
        } else {
            token.to_rust_type_path(ctx)
        };

        let ty = utils::str_to_type(&generic_type);

        let serde = utils::serde_hex_derive(&generic_type, ctx);

        members.push(quote!(#serde pub #name: #ty));
    }

    if generic_arg_names.len() != resolved_generic_args.len() {
        let generic_names = generic_arg_names
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>();

        return Err(cainome_parser::Error::GenericResolvationFailed(format!(
            "Not all generic arguments were resolved for enum {}. Resolved: {:?}, expected one of: [{}], fields: [{}]",
            full_path,
            resolved_generic_args,
            generic_names.join(", "),
            fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>().join(", "),
        )));
    }

    let mut internal_derives = vec![];

    for d in ctx.derives.iter() {
        if d.to_lowercase() == "serde" {
            continue;
        }
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

    Ok(quote! {
        #derive
        pub struct #struct_name #generic_args {
            #(#members),*
        }
    })
}

pub fn struct_implementation(
    full_path: &str,
    type_name: &str,
    fields: &Vec<NamedToken>,
    generic_arg_names: &Vec<Ident>,
    ctx: &ExpansionContext,
) -> CainomeResult<TokenStream> {
    let struct_name = utils::str_to_ident(type_name);

    let is_generic = !generic_arg_names.is_empty();
    let mut sizes: Vec<TokenStream> = vec![];
    let mut sers: Vec<TokenStream> = vec![];
    let mut desers: Vec<TokenStream> = vec![];
    let mut names: Vec<TokenStream> = vec![];
    let mut resolved_generic_args = HashSet::new();

    for inner in fields {
        let name = utils::str_to_ident(&inner.name);
        let token = &*inner.token.borrow();

        let generic_type_path = if is_generic {
            let resolved_result = ctx
                .generic_resolver
                .resolve_generic_member(full_path, inner, ctx);

            if let GenericResolveResult::Resolved(Some(arg)) = resolved_result {
                resolved_generic_args.insert(arg.clone());
                arg
            } else {
                token.to_rust_type_path(ctx)
            }
        } else {
            token.to_rust_type(ctx)
        };

        let ty = utils::str_to_type(&generic_type_path);

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

    if generic_arg_names.len() != resolved_generic_args.len() {
        let generic_names = generic_arg_names
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>();

        return Err(cainome_parser::Error::GenericResolvationFailed(format!(
            "Not all generic arguments were resolved for enum {}. Resolved: {:?}, expected one of: [{}], fields: [{}]",
            full_path,
            resolved_generic_args,
            generic_names.join(", "),
            fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>().join(", "),
        )));
    }

    let ccs = utils::str_to_type(&ctx.cainome_serde_path);
    let snrs_types = utils::snrs_types();

    let (generic_args, generic_where) = if is_generic {
        (
            quote! (<#(#generic_arg_names),*>),
            quote! ( where #(#generic_arg_names: #ccs::CairoSerde<RustType = #generic_arg_names>),*),
        )
    } else {
        (quote!(), quote!())
    };

    let (impl_line, rust_type) = (
        quote!(impl #generic_args #ccs::CairoSerde for #struct_name #generic_args #generic_where),
        quote!(
            type RustType = Self;
        ),
    );

    Ok(quote! {
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
    })
}

impl Expandable for Struct {
    fn expand(&self, ctx: &ExpansionContext) -> CainomeResult<Vec<ExpansionResult>> {
        let full_path = ctx.apply_alias(&self.type_path);
        let full_path_no_generic = genericity::type_path_no_generic(&full_path);
        let name = &full_path_no_generic.split("::").last().unwrap().to_owned();

        let ctx = ExpansionContextFactory::from(ctx)
            .with_derives(get_additional_derive_requirements(&self.fields, ctx))
            .build();

        let generic_arg_names = &self
            .generic_args
            .iter()
            .map(|(name, _)| utils::str_to_ident(name))
            .collect::<Vec<_>>();

        let declaration =
            struct_declaration(&full_path, name, &self.fields, generic_arg_names, &ctx)?;
        let implementation =
            struct_implementation(&full_path, name, &self.fields, generic_arg_names, &ctx)?;

        let item = quote! {
            #declaration

            #implementation
        };

        Ok(vec![
            ExpansionResult::new(&full_path_no_generic).with_item(name, item), // .with_imports(deps)
        ])
    }
}
