// Copyright 2019-2020 Parity Technologies (UK) Ltd.
// This file is part of substrate-subxt.
//
// subxt is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// subxt is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with substrate-subxt.  If not, see <http://www.gnu.org/licenses/>.

use crate::utils;
use heck::{
    CamelCase,
    SnakeCase,
};
use proc_macro2::TokenStream;
use quote::{
    format_ident,
    quote,
};
use synstructure::Structure;

pub fn call(s: Structure) -> TokenStream {
    let subxt = utils::use_crate("substrate-subxt");
    let codec = utils::use_crate("parity-scale-codec");
    let sp_core = utils::use_crate("sp-core");
    let sp_runtime = utils::use_crate("sp-runtime");
    let ident = &s.ast().ident;
    let generics = &s.ast().generics;
    let params = utils::type_params(generics);
    let module = utils::module_name(generics);
    let with_module = format_ident!(
        "with_{}",
        utils::path_to_ident(module).to_string().to_snake_case()
    );
    let call_name = ident.to_string().trim_end_matches("Call").to_snake_case();
    let call = format_ident!("{}", call_name);
    let call_trait = format_ident!("{}CallExt", call_name.to_camel_case());
    let bindings = utils::bindings(&s);
    let fields = bindings.iter().map(|bi| {
        let ident = bi.ast().ident.as_ref().unwrap();
        quote!(#ident,)
    });
    let args = bindings.iter().map(|bi| {
        let ident = bi.ast().ident.as_ref().unwrap();
        let ty = &bi.ast().ty;
        quote!(#ident: #ty,)
    });
    let args = quote!(#(#args)*);
    let ret = quote!(#subxt::ExtrinsicSuccess<T>);

    let expanded = quote! {
        impl#generics #subxt::Call<T> for #ident<#(#params),*> {
            const MODULE: &'static str = MODULE;
            const FUNCTION: &'static str = #call_name;
            fn events_decoder(
                decoder: &mut #subxt::EventsDecoder<T>,
            ) -> Result<(), #subxt::EventsError> {
                decoder.#with_module()?;
                Ok(())
            }
        }

        pub trait #call_trait<T: #module> {
            fn #call<'a>(
                self,
                #args
            ) -> core::pin::Pin<Box<dyn core::future::Future<Output = Result<#ret, #subxt::Error>> + Send + 'a>>;
        }

        impl<T, P, S, E> #call_trait<T> for #subxt::EventsSubscriber<T, P, S, E>
        where
            T: #module + #subxt::system::System + Send + Sync,
            P: #sp_core::Pair,
            S: #sp_runtime::traits::Verify + #codec::Codec + From<P::Signature> + Send + 'static,
            S::Signer: From<P::Public> + #sp_runtime::traits::IdentifyAccount<AccountId = T::AccountId>,
            T::Address: From<T::AccountId>,
            E: #subxt::SignedExtra<T> + #sp_runtime::traits::SignedExtension + 'static,
        {
            fn #call<'a>(
                self,
                #args
            ) -> core::pin::Pin<Box<dyn core::future::Future<Output = Result<#ret, #subxt::Error>> + Send + 'a>> {
                Box::pin(self.submit(#ident { #(#fields)* }))
            }
        }
    };

    TokenStream::from(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_call() {
        let input = quote! {
            #[derive(Call, Encode)]
            pub struct TransferCall<'a, T: Balances> {
                pub to: &'a <T as System>::Address,
                #[codec(compact)]
                pub amount: T::Balance,
            }
        };
        let expected = quote! {
            impl<'a, T: Balances> substrate_subxt::Call<T> for TransferCall<'a, T> {
                const MODULE: &'static str = MODULE;
                const FUNCTION: &'static str = "transfer";
                fn events_decoder(
                    decoder: &mut substrate_subxt::EventsDecoder<T>,
                ) -> Result<(), substrate_subxt::EventsError> {
                    decoder.with_balances()?;
                    Ok(())
                }
            }

            pub trait TransferCallExt<T: Balances> {
                fn transfer<'a>(
                    self,
                    to: &'a <T as System>::Address,
                    amount: T::Balance,
                ) -> core::pin::Pin<Box<dyn core::future::Future<Output = Result<substrate_subxt::ExtrinsicSuccess<T>, substrate_subxt::Error>> + Send + 'a>>;
            }

            impl<T, P, S, E> TransferCallExt<T> for substrate_subxt::EventsSubscriber<T, P, S, E>
            where
                T: Balances + substrate_subxt::system::System + Send + Sync,
                P: sp_core::Pair,
                S: sp_runtime::traits::Verify + codec::Codec + From<P::Signature> + Send + 'static,
                S::Signer: From<P::Public> + sp_runtime::traits::IdentifyAccount<
                    AccountId = T::AccountId>,
                T::Address: From<T::AccountId>,
                E: substrate_subxt::SignedExtra<T> + sp_runtime::traits::SignedExtension + 'static,
            {
                fn transfer<'a>(
                    self,
                    to: &'a <T as System>::Address,
                    amount: T::Balance,
                ) -> core::pin::Pin<Box<dyn core::future::Future<Output = Result<substrate_subxt::ExtrinsicSuccess<T>, substrate_subxt::Error>> + Send + 'a>> {
                    Box::pin(self.submit(TransferCall { to, amount, }))
                }
            }
        };
        let derive_input = syn::parse2(input).unwrap();
        let s = Structure::new(&derive_input);
        let result = call(s);
        utils::assert_proc_macro(result, expected);
    }
}
