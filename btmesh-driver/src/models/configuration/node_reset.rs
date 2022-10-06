use crate::{BackingStore, DriverError, Storage};
use btmesh_device::Signal;
use btmesh_device::{BluetoothMeshModelContext, CompletionStatus, InboundMetadata};
use btmesh_models::foundation::configuration::node_reset::NodeResetMessage;
use btmesh_models::foundation::configuration::ConfigurationServer;

static SIGNAL: Signal<CompletionStatus> = Signal::new();

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: &NodeResetMessage,
    meta: &InboundMetadata,
) -> Result<(), DriverError> {
    match message {
        NodeResetMessage::Reset => {
            ctx.send_with_completion(NodeResetMessage::Status.into(), meta.reply(), &SIGNAL)
                .await;

            storage.reset().await?;
        }
        NodeResetMessage::Status => {
            // not applicable
        }
    }
    Ok(())
}
