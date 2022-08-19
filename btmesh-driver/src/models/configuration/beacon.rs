use crate::{BackingStore, Configuration, DriverError, Storage};
use btmesh_device::{BluetoothMeshModelContext, InboundMetadata};
use btmesh_models::foundation::configuration::beacon::BeaconMessage;
use btmesh_models::foundation::configuration::ConfigurationServer;

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: BeaconMessage,
    meta: InboundMetadata,
) -> Result<(), DriverError> {
    match message {
        BeaconMessage::Get => {
            if let Configuration::Provisioned(config) = storage.get().await? {
                ctx.send(
                    BeaconMessage::Status(config.foundation().configuration().beacon()).into(),
                    meta.reply(),
                )
                .await?;
            }
        }
        BeaconMessage::Set(beacon) => {
            storage
                .modify(|config| {
                    *config.foundation_mut().configuration_mut().beacon_mut() = beacon;
                    Ok(())
                })
                .await
                .ok();
            ctx.send(BeaconMessage::Status(beacon).into(), meta.reply())
                .await?;
        }
        _ => {
            // not applicable
        }
    }
    Ok(())
}
