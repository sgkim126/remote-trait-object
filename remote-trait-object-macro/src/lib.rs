// Copyright 2020 Kodebox, Inc.
// This file is part of CodeChain.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#[macro_use]
extern crate quote;

mod helper;
mod service;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

/// Those necessary components for the macro is specially exported in the remote-trait-object.
/// The macro will always specify full path using this.
fn create_env_path() -> syn::Path {
    syn::parse2(quote! {remote_trait_object::macro_env}).unwrap()
}

#[proc_macro_attribute]
pub fn service(args: TokenStream, input: TokenStream) -> TokenStream {
    match service::service(TokenStream2::from(args), TokenStream2::from(input)) {
        Ok(x) => TokenStream::from(x),
        Err(x) => TokenStream::from(x),
    }
}

#[proc_macro_attribute]
pub fn service_debug(args: TokenStream, input: TokenStream) -> TokenStream {
    match service::service(TokenStream2::from(args), TokenStream2::from(input)) {
        Ok(x) => println!("{}", x),
        Err(x) => println!("{}", x),
    }
    TokenStream::new()
}
