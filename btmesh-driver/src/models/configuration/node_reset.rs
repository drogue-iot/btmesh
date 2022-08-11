use crate::{BackingStore, DriverError, Storage};
use btmesh_device::{BluetoothMeshModelContext, InboundMetadata};
use btmesh_models::foundation::configuration::node_reset::NodeResetMessage;
use btmesh_models::foundation::configuration::ConfigurationServer;

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: NodeResetMessage,
    meta: InboundMetadata,
) -> Result<(), DriverError> {
    match message {
        NodeResetMessage::Reset => {
            ctx.send(NodeResetMessage::Status.into(), meta.reply())
                .await?;
            // TODO: storage.reset() IFF status gets sent?
            storage.reset().await?
        }
        _ => {
            // not applicable to server role
        }
    }
    Ok(())
}
