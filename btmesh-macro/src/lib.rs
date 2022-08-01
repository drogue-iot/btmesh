#![feature(proc_macro_diagnostic)]

extern crate proc_macro2;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;


#[proc_macro_attribute]
pub fn device(_args: TokenStream, item: TokenStream) -> TokenStream {
    //item
    quote!(  ).into()
}

#[proc_macro_attribute]
pub fn element(_args: TokenStream, item: TokenStream) -> TokenStream {
    let mut element_struct = syn::parse_macro_input!(item as syn::ItemStruct);
    let struct_vis = &element_struct.vis;
    let struct_fields = match &mut element_struct.fields {
        syn::Fields::Named(n) => n,
        _ => {
            element_struct
                .ident
                .span()
                .unwrap()
                .error("element structs must have named fields, not tuples.")
                .emit();
            return TokenStream::new();
        }
    };
    let fields = struct_fields.named.iter().cloned().collect::<Vec<syn::Field>>();

    let struct_name = element_struct.ident.clone();

    quote!(  ).into()
}
