use crate::storage::ModifyError;
use crate::{BackingStore, DriverError, Storage};
use btmesh_device::{BluetoothMeshModelContext, InboundMetadata};
use btmesh_models::foundation::configuration::app_key::{AppKeyMessage, AppKeyStatusMessage};
use btmesh_models::foundation::configuration::ConfigurationServer;
use btmesh_models::Status;

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: AppKeyMessage,
    meta: InboundMetadata,
) -> Result<(), DriverError> {
    match message {
        AppKeyMessage::Add(add) => {
            let (status, err) = convert(
                storage
                    .modify(|config| {
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
            let (status, err) = convert(
                storage
                    .modify(|config| {
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
        _ => {}
    }

    Ok(())
}

fn convert(input: Result<(), ModifyError>) -> (Status, Option<DriverError>) {
    if let Err(result) = input {
        match result {
            ModifyError::Driver(DriverError::InvalidAppKeyIndex) => {
                (Status::InvalidAppKeyIndex, None)
            }
            ModifyError::Driver(DriverError::InvalidNetKeyIndex) => {
                (Status::InvalidNetKeyIndex, None)
            }
            ModifyError::Driver(DriverError::AppKeyIndexAlreadyStored) => {
                (Status::KeyIndexAlreadyStored, None)
            }
            ModifyError::Storage(_) => (Status::StorageFailure, None),
            ModifyError::Driver(inner) => (Status::UnspecifiedError, Some(inner)),
        }
    } else {
        (Status::Success, None)
    }
}
