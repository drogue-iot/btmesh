use crate::models::configuration::convert;
use crate::{BackingStore, DriverError, Storage};
use btmesh_device::{BluetoothMeshModelContext, InboundMetadata};
use btmesh_models::foundation::configuration::relay::{Relay, RelayConfig, RelayMessage};
use btmesh_models::foundation::configuration::ConfigurationServer;
use btmesh_models::Status;

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: &RelayMessage,
    meta: &InboundMetadata,
) -> Result<(), DriverError> {
    match message {
        RelayMessage::Get => {
            let relay = storage
                .read_provisioned(|config| Ok(*config.foundation().configuration().relay()))
                .await?;

            ctx.send(RelayMessage::Status(relay).into(), meta.reply())
                .await?;
            Ok(())
        }
        RelayMessage::Set(relay) => {
            let (status, err) = convert(
                &storage
                    .modify_provisioned(|config| {
                        let relay_config = config.foundation_mut().configuration_mut().relay_mut();
                        if let Relay::NotSupported = relay_config.relay() {
                            Err(DriverError::FeatureNotSupported)
                        } else {
                            *relay_config = *relay;
                            Ok(())
                        }
                    })
                    .await,
            );

            if let Status::Success = status {
                ctx.send(RelayMessage::Status(*relay).into(), meta.reply())
                    .await?;
            } else {
                ctx.send(
                    RelayMessage::Status(RelayConfig::not_supported()).into(),
                    meta.reply(),
                )
                .await?;
            }

            if let Some(err) = err {
                Err(err)
            } else {
                Ok(())
            }
        }
        RelayMessage::Status(_) => {
            // not applicable
            Ok(())
        }
    }
}
