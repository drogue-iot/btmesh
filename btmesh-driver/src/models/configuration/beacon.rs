use crate::{BackingStore, DriverError, Storage};
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
            let beacon = storage
                .read(|config| Ok(config.foundation().configuration().beacon()))
                .await?;

            ctx.send(BeaconMessage::Status(beacon).into(), meta.reply())
                .await?;
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
