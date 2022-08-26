use crate::{BackingStore, DriverError, Storage};
use btmesh_device::{BluetoothMeshModelContext, InboundMetadata};
use btmesh_models::foundation::configuration::beacon::BeaconMessage;
use btmesh_models::foundation::configuration::ConfigurationServer;

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: &BeaconMessage,
    meta: &InboundMetadata,
) -> Result<(), DriverError> {
    match message {
        BeaconMessage::Get => {
            let beacon = storage
                .read_provisioned(|config| Ok(config.foundation().configuration().beacon()))
                .await?;

            ctx.send(BeaconMessage::Status(beacon).into(), meta.reply())
                .await?;
        }
        BeaconMessage::Set(beacon) => {
            storage
                .modify_provisioned(|config| {
                    *config.foundation_mut().configuration_mut().beacon_mut() = *beacon;
                    Ok(())
                })
                .await?;
            ctx.send(BeaconMessage::Status(*beacon).into(), meta.reply())
                .await?;
        }
        BeaconMessage::Status(_) => {
            // not applicable
        }
    }
    Ok(())
}
