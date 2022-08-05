use core::future::Future;
use btmesh_device::{BluetoothMeshModel, BluetoothMeshModelContext};
use btmesh_models::foundation::configuration::{ConfigurationMessage, ConfigurationServer};

pub mod composition_data;

pub struct Configuration {

}

impl Configuration {
    pub fn new() -> Self {
        Self {

        }
    }
}

impl BluetoothMeshModel<ConfigurationServer> for Configuration {
    type RunFuture<'f, C>  = impl Future<Output=Result<(),()>> + 'f
    where Self: 'f,
    C: BluetoothMeshModelContext<ConfigurationServer> + 'f;

    fn run<'run, C: BluetoothMeshModelContext<ConfigurationServer> + 'run>(
        &'run self,
        ctx: C,
    ) -> Self::RunFuture<'_, C> {
        async move {
            info!("running configuration server");
            match ctx.receive().await {
                ConfigurationMessage::Beacon(beacon) => { }
                ConfigurationMessage::DefaultTTL(default_ttl) => { }
                ConfigurationMessage::CompositionData(composition_data) => {
                    info!("received {}", composition_data);
                }
                ConfigurationMessage::AppKey(app_key) => { }
                ConfigurationMessage::ModelApp(model_app) => { }
                ConfigurationMessage::ModelPublication(model_publication) => { }
                ConfigurationMessage::ModelSubscription(model_subscription) => { }
                ConfigurationMessage::NodeReset(node_reset) => { }
            }

            Ok(())
        }
    }
}