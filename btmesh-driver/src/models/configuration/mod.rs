#![allow(clippy::single_match)]
use crate::{BackingStore, DriverError, Storage};
use btmesh_device::{BluetoothMeshModel, BluetoothMeshModelContext, InboundModelPayload};
use btmesh_models::foundation::configuration::{ConfigurationMessage, ConfigurationServer};
use btmesh_models::Status;
use core::future::Future;

pub mod app_key;
pub mod beacon;
pub mod composition_data;
pub mod default_ttl;
pub mod model_app;
pub mod model_publication;
pub mod model_subscription;
pub mod node_reset;
pub mod relay;

pub struct Configuration<'s, B: BackingStore + 's> {
    storage: &'s Storage<B>,
}

impl<'s, B: BackingStore + 's> Configuration<'s, B> {
    pub fn new(storage: &'s Storage<B>) -> Self {
        Self { storage }
    }
}

impl<'s, B: BackingStore + 's> BluetoothMeshModel<ConfigurationServer> for Configuration<'s, B> {
    type RunFuture<'f, C>  = impl Future<Output=Result<(),()>> + 'f
    where Self: 'f,
    C: BluetoothMeshModelContext<ConfigurationServer> + 'f;

    fn run<'run, C: BluetoothMeshModelContext<ConfigurationServer> + 'run>(
        &'run mut self,
        ctx: C,
    ) -> Self::RunFuture<'_, C> {
        async move {
            loop {
                //let (message, meta) = ctx.receive().await;
                let payload = ctx.receive().await;

                if let InboundModelPayload::Message(message, meta) = payload {
                    match &message {
                        ConfigurationMessage::Beacon(beacon) => {
                            beacon::dispatch(&ctx, self.storage, beacon, &meta)
                                .await
                                .map_err(|_| ())?;
                        }
                        ConfigurationMessage::DefaultTTL(default_ttl) => {
                            default_ttl::dispatch(&ctx, self.storage, default_ttl, &meta)
                                .await
                                .map_err(|_| ())?;
                        }
                        ConfigurationMessage::Relay(relay) => {
                            relay::dispatch(&ctx, self.storage, relay, &meta)
                                .await
                                .map_err(|_| ())?;
                        }
                        ConfigurationMessage::CompositionData(composition_data) => {
                            composition_data::dispatch(&ctx, self.storage, composition_data, &meta)
                                .await
                                .map_err(|_| ())?;
                        }
                        ConfigurationMessage::AppKey(app_key) => {
                            app_key::dispatch(&ctx, self.storage, app_key, &meta)
                                .await
                                .map_err(|_| ())?;
                        }
                        ConfigurationMessage::ModelApp(model_app) => {
                            model_app::dispatch(&ctx, self.storage, model_app, &meta)
                                .await
                                .map_err(|_| ())?;
                        }
                        ConfigurationMessage::ModelPublication(model_publication) => {
                            model_publication::dispatch(
                                &ctx,
                                self.storage,
                                model_publication,
                                &meta,
                            )
                            .await
                            .map_err(|_| ())?;
                        }
                        ConfigurationMessage::ModelSubscription(model_subscription) => {
                            model_subscription::dispatch(
                                &ctx,
                                self.storage,
                                model_subscription,
                                &meta,
                            )
                            .await
                            .map_err(|_| ())?;
                        }
                        ConfigurationMessage::NodeReset(node_reset) => {
                            node_reset::dispatch(&ctx, self.storage, node_reset, &meta)
                                .await
                                .map_err(|_| ())?;
                        }
                    }
                }
            }
        }
    }
}

pub fn convert(input: &Result<(), DriverError>) -> (Status, Option<DriverError>) {
    if let Err(result) = input {
        (result).into()
    } else {
        (Status::Success, None)
    }
}
