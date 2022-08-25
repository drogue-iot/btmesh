use crate::{BackingStore, DriverError, Storage};
use btmesh_device::{BluetoothMeshModelContext, InboundMetadata};
use btmesh_models::foundation::configuration::composition_data::{
    CompositionDataMessage, CompositionStatus,
};
use btmesh_models::foundation::configuration::ConfigurationServer;

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: CompositionDataMessage,
    meta: InboundMetadata,
) -> Result<(), DriverError> {
    info!("COMPOSITION GET");
    match message {
        CompositionDataMessage::Get(page) => {
            if page == 0 {
                ctx.send(
                    CompositionStatus::new(0, storage.composition()).into(),
                    meta.reply(),
                )
                .await?;
                info!("SENT");
            }
        }
        _ => {
            // not applicable to server role
        }
    }
    Ok(())
}
