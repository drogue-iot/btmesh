use crate::{BackingStore, Configuration, DriverError, Storage};
use btmesh_device::{BluetoothMeshModelContext, InboundMetadata};
use btmesh_models::foundation::configuration::beacon::BeaconMessage;
use btmesh_models::foundation::configuration::relay::{Relay, RelayMessage};
use btmesh_models::foundation::configuration::ConfigurationServer;

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: RelayMessage,
    meta: InboundMetadata,
) -> Result<(), DriverError> {
    match message {
        RelayMessage::Get => {
            if let Configuration::Provisioned(config) = storage.get().await? {
                ctx.send(
                    RelayMessage::Status(*config.foundation().configuration().relay()).into(),
                    meta.reply(),
                )
                .await?;
            }
        }
        RelayMessage::Set(relay) => {
            if let Configuration::Provisioned(mut config) = storage.get().await? {
                let mut relay_config = config.foundation_mut().configuration_mut().relay_mut();

                if let Relay::NotSupported = relay_config.relay() {
                    ctx.send(RelayMessage::Status(*relay_config).into(), meta.reply())
                        .await?;
                } else {
                    *relay_config = relay;
                    ctx.send(RelayMessage::Status(relay).into(), meta.reply())
                        .await?;
                }
            }
        }
        _ => {
            // not applicable
        }
    }
    Ok(())
}
