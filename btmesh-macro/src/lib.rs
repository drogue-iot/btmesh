#![feature(proc_macro_diagnostic)]

extern crate proc_macro2;

use btmesh_common::{CompanyIdentifier, ProductIdentifier, VersionIdentifier};
use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::{TokenStream as TokenStream2};
use quote::{format_ident, quote};
use regex::Regex;

#[derive(FromMeta)]
struct DeviceArgs {
    cid: CompanyIdentifier,
    pid: ProductIdentifier,
    vid: VersionIdentifier,
}

#[derive(FromMeta)]
struct ElementArgs {
    location: String,
}

#[proc_macro_attribute]
pub fn device(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(args as syn::AttributeArgs);
    let args = match DeviceArgs::from_list(&args) {
        Ok(v) => v,
        Err(e) => {
            return e.write_errors().into();
        }
    };

    let cid = args.cid.0;
    let pid = args.pid.0;
    let vid = args.vid.0;

    let mut device_struct = syn::parse_macro_input!(item as syn::ItemStruct);

    let struct_fields = match &mut device_struct.fields {
        syn::Fields::Named(n) => n,
        _ => {
            device_struct
                .ident
                .span()
                .unwrap()
                .error("element structs must have named fields, not tuples.")
                .emit();
            return TokenStream::new();
        }
    };
    let fields = struct_fields
        .named
        .iter()
        .cloned()
        .collect::<Vec<syn::Field>>();

    let mut populate = TokenStream2::new();
    let mut dispatch = TokenStream2::new();

    for (i, field) in fields.iter().enumerate() {
        let field_name = field.ident.as_ref().unwrap();
        populate.extend(quote! {
            self.#field_name.populate(&mut composition);
        });
        dispatch.extend(quote! {
            #i => {
                self.#field_name.dispatch(opcode, parameters).await?;
            }
        })
    }

    let struct_name = device_struct.ident.clone();

    let mut device_impl = TokenStream2::new();

    device_impl.extend(quote!(
        impl ::btmesh_device::BluetoothMeshDevice for #struct_name {

            fn composition(&self) -> ::btmesh_device::Composition {
                use ::btmesh_device::BluetoothMeshElement;
                let mut composition = ::btmesh_device::Composition::new(
                    ::btmesh_device::CompanyIdentifier(#cid),
                    ::btmesh_device::ProductIdentifier(#pid),
                    ::btmesh_device::VersionIdentifier(#vid),
                );
                #populate

                composition
            }

            type DispatchFuture<'f> = impl Future<Output = Result<(), ()>> + 'f
                where Self: 'f;

            fn dispatch<'f>(&'f mut self, index: usize, opcode: ::btmesh_device::Opcode, parameters: &'f [u8]) -> Self::DispatchFuture<'f> {
                use ::btmesh_device::BluetoothMeshElement;
                async move {
                    match index {
                        #dispatch
                        _ => {
                            return Err(())
                        }
                    }
                    Ok(())
                }
            }
        }
    ));

    let result = quote!(
        #device_struct

        #device_impl
    )
    .into();
    println!("{}", result);

    result
}

#[proc_macro_attribute]
pub fn element(args: TokenStream, item: TokenStream) -> TokenStream {
    let mut element_struct = syn::parse_macro_input!(item as syn::ItemStruct);

    let args = syn::parse_macro_input!(args as syn::AttributeArgs);
    let args = match ElementArgs::from_list(&args) {
        Ok(v) => v,
        Err(e) => {
            return e.write_errors().into();
        }
    };

    let location = args.location.to_uppercase();
    let mut location_arg = TokenStream2::new();

    let re = Regex::new(r"^[0-9]+$").unwrap();
    if re.is_match(&location) {
        location_arg.extend(quote!(
            ::btmesh_device::location::Location::numeric(#location)
        ));
    } else {
        let location_const = format_ident!("{}", location);
        location_arg.extend(quote!(
            ::btmesh_device::location::#location_const
        ));
    };

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
    let fields = struct_fields
        .named
        .iter()
        .cloned()
        .collect::<Vec<syn::Field>>();
    let struct_name = element_struct.ident.clone();

    let mut populate = TokenStream2::new();
    let mut dispatch = TokenStream2::new();

    populate.extend(quote! {
        let mut descriptor = ::btmesh_device::ElementDescriptor::new( #location_arg );
    });

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        populate.extend(quote! {
            descriptor.add_model( self.#field_name.model_identifier() );
        });
        dispatch.extend(quote! {
            if let Ok(Some(message)) = self.#field_name.parse(opcode, parameters) {
                self.#field_name.handle(message).await?;
            }
        });
    }

    let mut element_impl = TokenStream2::new();

    element_impl.extend(quote!(
        impl ::btmesh_device::BluetoothMeshElement for #struct_name {
            fn populate(&self, composition: &mut ::btmesh_device::Composition) {
                #populate
                composition.add_element(descriptor);
            }

            type DispatchFuture<'f> = impl Future<Output = Result<(), ()>> + 'f
                where Self: 'f;

            fn dispatch<'f>(&'f mut self, opcode: ::btmesh_device::Opcode, parameters: &'f [u8]) -> Self::DispatchFuture<'f> {
                async move {
                    #dispatch
                    Ok(())
                }
            }
        }
    ));

    let result = quote!(
        #element_struct

        #element_impl
    )
    .into();

    println!("{}", result);

    result
}
