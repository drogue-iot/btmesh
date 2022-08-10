use crate::{BackingStore, DriverError, Storage};
use btmesh_device::{BluetoothMeshModelContext, InboundMetadata};
use btmesh_models::foundation::configuration::beacon::BeaconMessage;
use btmesh_models::foundation::configuration::composition_data::{
    CompositionDataMessage, CompositionStatus,
};
use btmesh_models::foundation::configuration::ConfigurationServer;

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: BeaconMessage,
    meta: InboundMetadata,
) -> Result<(), DriverError> {
    match message {
        BeaconMessage::Get => {
            ctx.send(BeaconMessage::Status(true).into(), meta.reply())
                .await?;
        }
        BeaconMessage::Set(_) => {}
        BeaconMessage::Status(_) => {}
    }
    Ok(())
}
