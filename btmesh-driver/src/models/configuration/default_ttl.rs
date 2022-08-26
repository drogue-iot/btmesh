use crate::{BackingStore, DriverError, Storage};
use btmesh_device::{BluetoothMeshModelContext, InboundMetadata};
use btmesh_models::foundation::configuration::default_ttl::DefaultTTLMessage;
use btmesh_models::foundation::configuration::ConfigurationServer;

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: &DefaultTTLMessage,
    meta: &InboundMetadata,
) -> Result<(), DriverError> {
    match message {
        DefaultTTLMessage::Get => {
            let ttl = storage
                .read_provisioned(|config| Ok(config.foundation().configuration().default_ttl()))
                .await?;

            ctx.send(DefaultTTLMessage::Status(ttl).into(), meta.reply())
                .await?;
        }
        DefaultTTLMessage::Set(default_ttl) => {
            storage
                .modify_provisioned(|config| {
                    *config
                        .foundation_mut()
                        .configuration_mut()
                        .default_ttl_mut() = *default_ttl;
                    Ok(())
                })
                .await?;
            ctx.send(DefaultTTLMessage::Status(*default_ttl).into(), meta.reply())
                .await?;
        }
        DefaultTTLMessage::Status(_) => {
            // not applicable
        }
    }
    Ok(())
}
