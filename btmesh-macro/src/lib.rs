#![feature(proc_macro_diagnostic)]

extern crate proc_macro2;

use btmesh_common::{CompanyIdentifier, ProductIdentifier, VersionIdentifier};
use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{Field, GenericParam, Type};

#[derive(FromMeta)]
struct DeviceArgs {
    cid: CompanyIdentifier,
    pid: ProductIdentifier,
    vid: VersionIdentifier,
}

#[derive(FromMeta)]
struct ElementArgs {
    location: syn::Lit,
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
    let mut generic_params = TokenStream2::new();
    if !generics.params.is_empty() {
        generic_params.extend(quote!( < ));
        for param in generics.params.iter() {
            match param {
                GenericParam::Type(t) => {
                    let t = t.ident.clone();
                    generic_params.extend(quote! {
                        #t,
                    })
                }
                GenericParam::Lifetime(l) => {
                    let l = l.lifetime.clone();
                    generic_params.extend(quote! {
                        #l,
                    })
                }
                GenericParam::Const(c) => {
                    let c = c.ident.clone();
                    generic_params.extend(quote! {
                        #c,
                    })
                }
            }
        }
        generic_params.extend(quote!( > ));
    }

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
        .filter(|e| !matches!(e.ty, Type::Reference(_)))
        .collect::<Vec<syn::Field>>();

    let mut populate = TokenStream2::new();
    let mut ctor_params = TokenStream2::new();
    let mut run_prolog = TokenStream2::new();
    let mut static_channels = TokenStream2::new();
    let mut fanout = TokenStream2::new();

    for (i, field) in fields.iter().enumerate() {
        let field_name = field.ident.as_ref().unwrap();
        populate.extend(quote! {
            self.#field_name.populate(&mut composition);
        });

        let element_channel_name = format_ident!(
            "{}",
            field.ident.as_ref().unwrap().to_string().to_uppercase()
        );

        static_channels.extend( quote!{
            static #element_channel_name: ::btmesh_device::InboundChannel = ::btmesh_device::InboundChannel::new();
        });

        let ctx_name = format_ident!("{}_ctx", field_name);
        run_prolog.extend(quote! {
            let #ctx_name = ctx.element_context(#i, #element_channel_name.receiver() );
        });
        fanout.extend(quote! {
            if target_element_index == #i {
                #element_channel_name.send(message.clone()).await;
            }
        });
        ctor_params.extend(quote! {
            ::btmesh_device::BluetoothMeshElement::run(&mut self.#field_name, #ctx_name),
        })
    }

    let struct_name = device_struct.ident.clone();

    let mut device_impl = TokenStream2::new();

    let future_struct_name = join_future_name(struct_name.clone());
    let device_future = fields_join_future(struct_name.clone(), fields);

    device_impl.extend(quote!(

        impl #generics ::btmesh_device::BluetoothMeshDevice for #struct_name #generic_params {

            fn composition(&self) -> ::btmesh_device::Composition<::btmesh_device::CompositionExtra> {
                use ::btmesh_device::BluetoothMeshElement;
                let mut composition = ::btmesh_device::Composition::<::btmesh_device::CompositionExtra>::new(
                    ::btmesh_device::CompanyIdentifier(#cid),
                    ::btmesh_device::ProductIdentifier(#pid),
                    ::btmesh_device::VersionIdentifier(#vid),
                );
                #populate

                composition
            }

            type RunFuture<'f, C> = impl Future<Output = Result<(), ()>> + 'f
                where Self: 'f,
                C: ::btmesh_device::BluetoothMeshDeviceContext + 'f;

            fn run<'run, C: ::btmesh_device::BluetoothMeshDeviceContext + 'run>(&'run mut self, ctx: C) -> Self::RunFuture<'run, C> {
                use btmesh_device::BluetoothMeshElementContext;
                async move {
                    #run_prolog
                    ::btmesh_device::join(
                        async move {
                            loop {
                                let message = ctx.receive().await;
                                let target_element_index = message.element_index;
                                #fanout
                            }
                        },
                        #future_struct_name::new(
                            #ctor_params
                        ),
                    ).await.1.ok();

                    Ok(())
                }
            }
        }

        #device_future

        #static_channels
    ) );

    let result = quote!(
        #device_struct

        #device_impl
    );

    /*
    let pretty = result.clone();
    let file: syn::File = syn::parse(pretty.into()).unwrap();
    let pretty = prettyplease::unparse(&file);
    println!("{}", pretty);
     */

    result.into()
}

#[proc_macro_attribute]
pub fn element(args: TokenStream, item: TokenStream) -> TokenStream {
    let element_struct = syn::parse_macro_input!(item as syn::ItemStruct);

    let generics = element_struct.generics.clone();

    let mut generic_params = TokenStream2::new();
    if !generics.params.is_empty() {
        generic_params.extend(quote!( < ));
        for param in generics.params.iter() {
            match param {
                GenericParam::Type(t) => {
                    let t = t.ident.clone();
                    generic_params.extend(quote! {
                        #t,
                    })
                }
                GenericParam::Lifetime(l) => {
                    let l = l.lifetime.clone();
                    generic_params.extend(quote! {
                        #l,
                    })
                }
                GenericParam::Const(c) => {
                    let c = c.ident.clone();
                    generic_params.extend(quote! {
                        #c,
                    })
                }
            }
        }
        generic_params.extend(quote!( > ));
    }

    let args = syn::parse_macro_input!(args as syn::AttributeArgs);
    let args = match ElementArgs::from_list(&args) {
        Ok(v) => v,
        Err(e) => {
            return e.write_errors().into();
        }
    };

    let mut location_arg = TokenStream2::new();
    match args.location {
        syn::Lit::Int(l) => {
            location_arg.extend(quote!(
                ::btmesh_device::location::Location::numeric(#l)
            ));
        }
        syn::Lit::Str(l) => {
            let location = l.value().to_uppercase();
            let location_const = format_ident!("{}", location);
            location_arg.extend(quote!(
                ::btmesh_device::location::#location_const
            ));
        }
        l => panic!(
            "Location must be numeric or a constant. Unsupported literal type: {:?}",
            l
        ),
    }

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
        .filter(|e| !matches!(e.ty, Type::Reference(_)))
        .collect::<Vec<syn::Field>>();
    let struct_name = element_struct.ident.clone();

    let mut populate = TokenStream2::new();
    let mut ctor_params = TokenStream2::new();
    let mut run_prolog = TokenStream2::new();
    let mut fanout = TokenStream2::new();

    populate.extend(quote! {
        let mut descriptor = ::btmesh_device::ElementDescriptor::new( #location_arg );
    });

    for field in fields.clone() {
        let field_name = field.ident.as_ref().unwrap();
        populate.extend(quote! {
            descriptor.add_model( self.#field_name.model_identifier() );
        });

        let ch_name = format_ident!("{}_ch", field_name);
        let ch_sender_name = format_ident!("{}_sender", field_name);
        let ch_receiver_name = format_ident!("{}_receiver", field_name);
        let ch_parser_name = format_ident!("{}_parser", field_name);
        let ch_model_id_name = format_ident!("{}_id", field_name);

        let ctx_name = format_ident!("{}_ctx", field_name);
        run_prolog.extend(quote! {
            let #ch_name = ::btmesh_device::InboundModelChannel::new();
            let #ch_sender_name = #ch_name.sender();
            let #ch_receiver_name = #ch_name.receiver();
            let #ch_parser_name = self.#field_name.parser();
            let #ch_model_id_name = &self.#field_name.model_identifier();
            let #ctx_name = ctx.model_context(#ch_receiver_name );
        });

        fanout.extend(quote! {
            let for_me = message.model_identifier.map(|id| &id == #ch_model_id_name).unwrap_or(true);
            if for_me {
                match &message.body {
                    ::btmesh_device::InboundBody::Message(message) => {
                        if let Ok(Some(model_message)) = #ch_parser_name( &message.opcode, &message.parameters ) {
                            #ch_sender_name.try_send( ::btmesh_device::InboundModelPayload::Message(model_message, message.meta) ).ok();
                        }
                    }
                    ::btmesh_device::InboundBody::Control(control) => {
                        #ch_sender_name.try_send( ::btmesh_device::InboundModelPayload::Control(*control) ).ok();
                    }
                }
            }
        });

        ctor_params.extend(quote! {
            ::btmesh_device::BluetoothMeshModel::run(&mut self.#field_name, #ctx_name),
        });
    }

    let mut element_impl = TokenStream2::new();

    let future_struct_name = select_future_name(struct_name.clone());

    element_impl.extend(quote!(
        impl #generics ::btmesh_device::BluetoothMeshElement for #struct_name #generic_params {
            fn populate(&self, composition: &mut ::btmesh_device::Composition<::btmesh_device::CompositionExtra>) {
                #populate
                composition.add_element(descriptor).ok();
            }

            type RunFuture<'f,C> = impl Future<Output = Result<(), ()>> + 'f
                where Self: 'f,
                C: ::btmesh_device::BluetoothMeshElementContext + 'f;

            fn run<'run, C: ::btmesh_device::BluetoothMeshElementContext + 'run>(&mut self, ctx: C) -> Self::RunFuture<'_, C> {
                                use btmesh_device::BluetoothMeshElementContext;
                async move {
                    #run_prolog
                    ::btmesh_device::join(
                        async {
                            loop {
                                let message = ctx.receive().await;
                                #fanout
                            }
                        },
                        #future_struct_name::new(
                            #ctor_params
                        ),
                    ).await.1.ok();

                    Ok(())
                }
            }
        }
    ));

    let element_future = fields_select_future(struct_name, fields);

    let result = quote!(
        #element_struct

        #element_impl

        #element_future
    );

    /*
    let pretty = result.clone();
    let file: syn::File = syn::parse(pretty.into()).unwrap();
    let pretty = prettyplease::unparse(&file);
    println!("{}", pretty);
     */

    result.into()
}

fn select_future_name(struct_name: Ident) -> Ident {
    format_ident!("{}SelectFuture", struct_name)
}

fn fields_select_future(struct_name: Ident, fields: Vec<Field>) -> TokenStream2 {
    let mut future = TokenStream2::new();

    let mut generics = TokenStream2::new();
    let mut generic_params = TokenStream2::new();
    let mut future_fields = TokenStream2::new();
    let mut field_poll = TokenStream2::new();
    let mut ctor = TokenStream2::new();

    if !fields.is_empty() {
        generics.extend(quote!( < ));
        generic_params.extend(quote!( < ));
        for field in fields {
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

    let future_struct_name = select_future_name(struct_name);

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

fn join_future_name(struct_name: Ident) -> Ident {
    format_ident!("{}JoinFuture", struct_name)
}

fn fields_join_future(struct_name: Ident, fields: Vec<Field>) -> TokenStream2 {
    let mut future = TokenStream2::new();

    let mut generics = TokenStream2::new();
    let mut generic_params = TokenStream2::new();
    let mut future_fields = TokenStream2::new();
    let mut future_complete_fields = TokenStream2::new();
    let mut field_poll = TokenStream2::new();
    let mut field_complete = TokenStream2::new();
    let mut ctor = TokenStream2::new();

    field_complete.extend(quote!(true));

    if !fields.is_empty() {
        generics.extend(quote!( < ));
        generic_params.extend(quote!( < ));
        for field in fields {
            let field_future_type = field.ident.clone().unwrap().to_string().to_uppercase();
            let field_future_type = format_ident!("{}", field_future_type);
            generics.extend(
                quote!( #field_future_type: ::core::future::Future<Output=Result<(),()>>, ),
            );
            generic_params.extend(quote!( #field_future_type, ));

            let field_future_name = field.ident.clone().unwrap();
            future_fields.extend(quote!( #field_future_name: #field_future_type, ));

            let field_future_complete_name =
                format_ident!("{}_complete", field.ident.clone().unwrap());
            future_complete_fields.extend(quote!( #field_future_complete_name: bool, ));

            ctor.extend(quote! {
                #field_future_name,
                #field_future_complete_name: false,
            });

            field_poll.extend( quote! {
                if ! self_mut.#field_future_complete_name {
                    let result = unsafe { ::core::pin::Pin::new_unchecked(&mut self_mut.#field_future_name)  }.poll(cx);
                    if let::core::task::Poll::Ready(_) = result {
                        self_mut.#field_future_complete_name = true;
                    }
                }
            });

            field_complete.extend(quote!( && self_mut.#field_future_complete_name))
        }
        generics.extend(quote!( > ));
        generic_params.extend(quote!( > ));
    }

    let future_struct_name = join_future_name(struct_name);

    future.extend(quote! {
        struct #future_struct_name #generics {
            #future_fields
            #future_complete_fields
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
                let mut self_mut = unsafe{ self.get_unchecked_mut() };
                #field_poll

                if #field_complete {
                    ::core::task::Poll::Ready(Ok(()))
                } else {
                    ::core::task::Poll::Pending
                }
            }
        }
    });

    future
}
