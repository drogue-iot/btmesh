#![feature(proc_macro_diagnostic)]

extern crate proc_macro2;

use btmesh_common::{CompanyIdentifier, ProductIdentifier, VersionIdentifier};
use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use regex::Regex;
use syn::{Field, File};

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

    let generics = device_struct.generics.clone();
    let generic_params = device_struct.generics.clone();

    let struct_fields = match &mut device_struct.fields {
        syn::Fields::Named(n) => n,
        _ => {
            device_struct
                .ident
                .span()
                .unwrap()
                .error("device structs must have named fields, not tuples.")
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
    let mut ctor_params = TokenStream2::new();

    for (i, field) in fields.iter().enumerate() {
        let field_name = field.ident.as_ref().unwrap();
        populate.extend(quote! {
            self.#field_name.populate(&mut composition);
        });
        dispatch.extend(quote! {
            #i => {
                self.#field_name.dispatch(opcode, parameters).await?;
            }
        });
        ctor_params.extend(quote! {
            //self.#field_name.run(),
            ::btmesh_device::BluetoothMeshElement::run(&self.#field_name)
        })
    }

    let struct_name = device_struct.ident.clone();

    let mut device_impl = TokenStream2::new();

    let future_struct_name = future_struct_name(struct_name.clone());
    let device_future = fields_future(struct_name.clone(), fields);

    device_impl.extend(quote!(
        impl #generics ::btmesh_device::BluetoothMeshDevice for #struct_name #generic_params {

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

            type RunFuture<'f> = impl Future<Output = Result<(), ()>> + 'f
                where Self: 'f;

            fn run(&self) -> Self::RunFuture<'_> {
                #future_struct_name::new(
                    #ctor_params
                )
            }

            type DispatchFuture<'f> = impl Future<Output = Result<(), ()>> + 'f
                where Self: 'f;

            fn dispatch<'f>(&'f self, index: usize, opcode: ::btmesh_device::Opcode, parameters: &'f [u8]) -> Self::DispatchFuture<'f> {
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

        #device_future
    ) );

    let result = quote!(
        #device_struct

        #device_impl
    );

    let pretty = result.clone();
    let file: File = syn::parse(pretty.into()).unwrap();
    let pretty = prettyplease::unparse(&file);
    println!("{}", pretty);

    result.into()
}

#[proc_macro_attribute]
pub fn element(args: TokenStream, item: TokenStream) -> TokenStream {
    let element_struct = syn::parse_macro_input!(item as syn::ItemStruct);

    let generics = element_struct.generics.clone();
    let generic_params = element_struct.generics.clone();

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

    let struct_fields = match element_struct.fields.clone() {
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
    let mut ctor_params = TokenStream2::new();

    populate.extend(quote! {
        let mut descriptor = ::btmesh_device::ElementDescriptor::new( #location_arg );
    });

    for field in fields.clone() {
        let field_name = field.ident.as_ref().unwrap();
        populate.extend(quote! {
            descriptor.add_model( self.#field_name.model_identifier() );
        });
        dispatch.extend(quote! {
            if let Ok(Some(message)) = self.#field_name.parse(opcode, parameters) {
                self.#field_name.handle(message).await?;
            }
        });
        ctor_params.extend(quote! {
            self.#field_name.run(),
        })
    }

    let mut element_impl = TokenStream2::new();

    let future_struct_name = future_struct_name(struct_name.clone());

    element_impl.extend(quote!(
        impl #generics ::btmesh_device::BluetoothMeshElement for #struct_name #generic_params {
            fn populate(&self, composition: &mut ::btmesh_device::Composition) {
                #populate
                composition.add_element(descriptor);
            }

            type RunFuture<'f> = impl Future<Output = Result<(), ()>> + 'f
                where Self: 'f;

            fn run(&self) -> Self::RunFuture<'_> {
                #future_struct_name::new(
                    #ctor_params
                )
            }

            type DispatchFuture<'f> = impl Future<Output = Result<(), ()>> + 'f
                where Self: 'f;

            fn dispatch<'f>(&'f self, opcode: ::btmesh_device::Opcode, parameters: &'f [u8]) -> Self::DispatchFuture<'f> {
                async move {
                    #dispatch
                    Ok(())
                }
            }
        }
    ));

    let element_future = fields_future(struct_name, fields);

    let result = quote!(
        #element_struct

        #element_impl

        #element_future
    );

    let pretty = result.clone();
    let file: File = syn::parse(pretty.into()).unwrap();
    let pretty = prettyplease::unparse(&file);
    println!("{}", pretty);

    result.into()
}

fn future_struct_name(struct_name: Ident) -> Ident {
    format_ident!("{}MultiFuture", struct_name)
}

fn fields_future(struct_name: Ident, fields: Vec<Field>) -> TokenStream2 {
    let mut future = TokenStream2::new();

    let mut generics = TokenStream2::new();
    let mut generic_params = TokenStream2::new();
    let mut future_fields = TokenStream2::new();
    let mut field_poll = TokenStream2::new();
    let mut ctor = TokenStream2::new();
    let mut singleton_params = TokenStream2::new();

    if !fields.is_empty() {
        generics.extend(quote!( < ));
        generic_params.extend(quote!( < ));
        for field in fields {
            let field_type = field.ty.clone();
            singleton_params.extend(quote! {
                <#field_type as ElementModelPublisher>::RunFuture,
            });
            let field_future_type = field.ident.clone().unwrap().to_string().to_uppercase();
            let field_future_type = format_ident!("{}", field_future_type);
            generics.extend(
                quote!( #field_future_type: ::core::future::Future<Output=Result<(),()>>, ),
            );
            generic_params.extend(quote!( #field_future_type, ));

            let field_future_name = field.ident.clone().unwrap();
            future_fields.extend(quote!( #field_future_name: #field_future_type, ));

            ctor.extend(quote! {
                #field_future_name,
            });

            field_poll.extend( quote! {
                let result = unsafe { ::core::pin::Pin::new_unchecked(&mut self_mut.#field_future_name)  }.poll(cx);
                if let::core::task::Poll::Ready(_) = result {
                    return result
                }
            })
        }
        generics.extend(quote!( > ));
        generic_params.extend(quote!( > ));
    }

    let future_struct_name = future_struct_name(struct_name);

    future.extend(quote! {
        struct #future_struct_name #generics {
            #future_fields
        }

        impl #generics #future_struct_name #generic_params {
            const fn new(#future_fields) -> Self {
                Self {
                    #ctor
                }
            }
        }

        impl #generics ::core::future::Future for #future_struct_name #generic_params {
            type Output = Result<(), ()>;

            fn poll(self: ::core::pin::Pin<&mut Self>, cx: &mut ::core::task::Context<'_>) -> ::core::task::Poll<Self::Output> {
                //use ::core::future::Future;
                let mut self_mut = unsafe{ self.get_unchecked_mut() };
                #field_poll

                ::core::task::Poll::Pending
            }
        }
    });

    future
}
