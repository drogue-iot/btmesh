#![allow(clippy::single_match)]
use crate::{BackingStore, Storage};
use btmesh_device::{BluetoothMeshModel, BluetoothMeshModelContext};
use btmesh_models::foundation::configuration::{ConfigurationMessage, ConfigurationServer};
use core::future::Future;

pub mod app_key;
pub mod beacon;
pub mod composition_data;
pub mod default_ttl;
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
                let (message, meta) = ctx.receive().await;
                match message {
                    ConfigurationMessage::Beacon(beacon) => {
                        beacon::dispatch(&ctx, self.storage, beacon, meta)
                            .await
                            .map_err(|_| ())?;
                    }
                    ConfigurationMessage::DefaultTTL(default_ttl) => {
                        default_ttl::dispatch(&ctx, self.storage, default_ttl, meta)
                            .await
                            .map_err(|_| ())?;
                    }
                    ConfigurationMessage::Relay(relay) => {
                        relay::dispatch(&ctx, self.storage, relay, meta)
                            .await
                            .map_err(|_| ())?;
                    }
                    ConfigurationMessage::CompositionData(composition_data) => {
                        composition_data::dispatch(&ctx, self.storage, composition_data, meta)
                            .await
                            .map_err(|_| ())?;
                    }
                    ConfigurationMessage::AppKey(app_key) => {
                        info!("--------> {}", app_key);
                        app_key::dispatch(&ctx, self.storage, app_key, meta)
                            .await
                            .map_err(|_| ())?;
                    }
                    ConfigurationMessage::ModelApp(_model_app) => {}
                    ConfigurationMessage::ModelPublication(_model_publication) => {}
                    ConfigurationMessage::ModelSubscription(_model_subscription) => {}
                    ConfigurationMessage::NodeReset(node_reset) => {
                        node_reset::dispatch(&ctx, self.storage, node_reset, meta)
                            .await
                            .map_err(|_| ())?;
                    }
                }
            }
        }
    }
}
