#![allow(clippy::single_match)]
use crate::{BackingStore, Storage};
use btmesh_device::{BluetoothMeshModel, BluetoothMeshModelContext};
use btmesh_models::foundation::configuration::{ConfigurationMessage, ConfigurationServer};
use core::future::Future;

pub mod beacon;
pub mod composition_data;

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
                    ConfigurationMessage::DefaultTTL(_default_ttl) => {}
                    ConfigurationMessage::CompositionData(composition_data) => {
                        composition_data::dispatch(&ctx, self.storage, composition_data, meta)
                            .await
                            .map_err(|_| ())?;
                    }
                    ConfigurationMessage::AppKey(_app_key) => {}
                    ConfigurationMessage::ModelApp(_model_app) => {}
                    ConfigurationMessage::ModelPublication(_model_publication) => {}
                    ConfigurationMessage::ModelSubscription(_model_subscription) => {}
                    ConfigurationMessage::NodeReset(_node_reset) => {}
                }
            }
        }
    }
}
