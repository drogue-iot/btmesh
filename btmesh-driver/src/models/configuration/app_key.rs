use crate::models::configuration::convert;
use crate::{BackingStore, DriverError, Storage};
use btmesh_device::{BluetoothMeshModelContext, InboundMetadata};
use btmesh_models::foundation::configuration::app_key::{AppKeyMessage, AppKeyStatusMessage};
use btmesh_models::foundation::configuration::ConfigurationServer;

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: &AppKeyMessage,
    meta: &InboundMetadata,
) -> Result<(), DriverError> {
    match message {
        AppKeyMessage::Add(add) => {
            info!("app-key add");
            let (status, err) = convert(
                &storage
                    .modify_provisioned(|config| {
                        config.secrets_mut().add_application_key(
                            add.net_key_index(),
                            add.app_key_index(),
                            add.app_key(),
                        )?;
                        Ok(())
                    })
                    .await,
            );

            ctx.send(
                AppKeyMessage::Status(AppKeyStatusMessage {
                    status,
                    indexes: add.indexes,
                })
                .into(),
                meta.reply(),
            )
            .await?;

            if let Some(err) = err {
                return Err(err);
            }
        }
        AppKeyMessage::Get(_get) => {}
        AppKeyMessage::Delete(delete) => {
            info!("app-key delete");
            let (status, err) = convert(
                &storage
                    .modify_provisioned(|config| {
                        config.secrets_mut().delete_application_key(
                            delete.net_key_index(),
                            delete.app_key_index(),
                        )?;
                        Ok(())
                    })
                    .await,
            );

            ctx.send(
                AppKeyMessage::Status(AppKeyStatusMessage {
                    status,
                    indexes: delete.indexes,
                })
                .into(),
                meta.reply(),
            )
            .await?;
            if let Some(err) = err {
                return Err(err);
            }
        }
        AppKeyMessage::List(_list) => {}
        AppKeyMessage::Update(_update) => {}
        AppKeyMessage::Status(_) => {
            // not applicable
        }
    }

    Ok(())
}
