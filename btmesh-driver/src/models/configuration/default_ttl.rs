use crate::{BackingStore, Configuration, DriverError, Storage};
use btmesh_device::{BluetoothMeshModelContext, InboundMetadata};
use btmesh_models::foundation::configuration::default_ttl::DefaultTTLMessage;
use btmesh_models::foundation::configuration::ConfigurationServer;

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: DefaultTTLMessage,
    meta: InboundMetadata,
) -> Result<(), DriverError> {
    match message {
        DefaultTTLMessage::Get => {
            if let Configuration::Provisioned(config) = storage.get().await? {
                ctx.send(
                    DefaultTTLMessage::Status(config.foundation().configuration().default_ttl())
                        .into(),
                    meta.reply(),
                )
                .await?;
            }
        }
        DefaultTTLMessage::Set(default_ttl) => {
            if let Configuration::Provisioned(mut config) = storage.get().await? {
                *config
                    .foundation_mut()
                    .configuration_mut()
                    .default_ttl_mut() = default_ttl;
                ctx.send(DefaultTTLMessage::Status(default_ttl).into(), meta.reply())
                    .await?;
            }
        }
        _ => {
            // not applicable
        }
    }
    Ok(())
}
