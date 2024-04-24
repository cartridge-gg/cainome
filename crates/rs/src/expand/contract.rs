use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use super::utils;

pub struct CairoContract;

impl CairoContract {
    pub fn expand(contract_name: Ident) -> TokenStream2 {
        let reader = utils::str_to_ident(format!("{}Reader", contract_name).as_str());

        let snrs_types = utils::snrs_types();
        let snrs_accounts = utils::snrs_accounts();
        let snrs_providers = utils::snrs_providers();

        let q = quote! {

            #[derive(Debug)]
            pub struct #contract_name<A: #snrs_accounts::ConnectedAccount + Sync> {
                pub address: #snrs_types::FieldElement,
                pub account: A,
                pub block_id: #snrs_types::BlockId,
            }

            impl<A: #snrs_accounts::ConnectedAccount + Sync> #contract_name<A> {
                pub fn new(address: #snrs_types::FieldElement, account: A) -> Self {
                    Self { address, account, block_id: #snrs_types::BlockId::Tag(#snrs_types::BlockTag::Pending) }
                }

                pub fn set_contract_address(mut self, address: #snrs_types::FieldElement) {
                    self.address = address;
                }

                pub fn provider(&self) -> &A::Provider {
                    self.account.provider()
                }

                pub fn with_block(self, block_id: #snrs_types::BlockId) -> Self {
                    Self { block_id, ..self }
                }
            }

            #[derive(Debug)]
            pub struct #reader<P: #snrs_providers::Provider + Sync> {
                pub address: #snrs_types::FieldElement,
                pub provider: P,
                pub block_id: #snrs_types::BlockId,
            }

            impl<P: #snrs_providers::Provider + Sync> #reader<P> {
                pub fn new(
                    address: #snrs_types::FieldElement,
                    provider: P,
                ) -> Self {
                    Self { address, provider, block_id: #snrs_types::BlockId::Tag(#snrs_types::BlockTag::Pending) }
                }

                pub fn set_contract_address(mut self, address: #snrs_types::FieldElement) {
                    self.address = address;
                }

                pub fn provider(&self) -> &P {
                    &self.provider
                }

                pub fn with_block(self, block_id: #snrs_types::BlockId) -> Self {
                    Self { block_id, ..self }
                }
            }
        };

        q
    }
}
