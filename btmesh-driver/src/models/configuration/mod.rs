use btmesh_device::{BluetoothMeshModel, BluetoothMeshModelContext};
use btmesh_models::foundation::configuration::{ConfigurationMessage, ConfigurationServer};
use core::future::Future;
use core::marker::PhantomData;
use crate::{BackingStore, Storage};

pub mod composition_data;

pub struct Configuration<'s, B: BackingStore + 's> {
    storage: &'s Storage<B>,
}

impl<'s, B: BackingStore + 's> Configuration<'s, B> {
    pub fn new(storage: &'s Storage<B>) -> Self {
        Self {
            storage,
        }
    }
}

impl<'s, B: BackingStore + 's> BluetoothMeshModel<ConfigurationServer> for Configuration<'s, B> {
    type RunFuture<'f, C>  = impl Future<Output=Result<(),()>> + 'f
    where Self: 'f,
    C: BluetoothMeshModelContext<ConfigurationServer> + 'f;

    fn run<'run, C: BluetoothMeshModelContext<ConfigurationServer> + 'run>(
        &'run self,
        ctx: C,
    ) -> Self::RunFuture<'_, C> {
        async move {
            info!("running configuration server");
            let (message, meta) = ctx.receive().await;
            match message {
                ConfigurationMessage::Beacon(beacon) => {}
                ConfigurationMessage::DefaultTTL(default_ttl) => {}
                ConfigurationMessage::CompositionData(composition_data) => {
                    info!("received {}", composition_data);
                    composition_data::dispatch(ctx, self.storage, composition_data, meta).await;
                }
                ConfigurationMessage::AppKey(app_key) => {}
                ConfigurationMessage::ModelApp(model_app) => {}
                ConfigurationMessage::ModelPublication(model_publication) => {}
                ConfigurationMessage::ModelSubscription(model_subscription) => {}
                ConfigurationMessage::NodeReset(node_reset) => {}
            }

            Ok(())
        }
    }
}
