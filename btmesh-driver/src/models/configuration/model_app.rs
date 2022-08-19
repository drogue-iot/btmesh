use crate::storage::ModifyError;
use crate::{BackingStore, DriverError, Storage};
use btmesh_device::{BluetoothMeshModelContext, InboundMetadata};
use btmesh_models::foundation::configuration::model_app::{ModelAppMessage, ModelAppStatusMessage};
use btmesh_models::foundation::configuration::ConfigurationServer;
use btmesh_models::Status;

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: ModelAppMessage,
    meta: InboundMetadata,
) -> Result<(), DriverError> {
    match message {
        ModelAppMessage::Bind(bind) => {
            let composition = storage.composition();
            let (status, err) = convert(
                storage
                    .modify(|config| {
                        if config.secrets().has_application_key(bind.app_key_index) {
                            if let Some(element_index) = config
                                .device_info()
                                .local_element_index(bind.element_address.into())
                            {
                                config.bindings_mut().bind(
                                    composition,
                                    element_index,
                                    bind.model_identifier,
                                    bind.app_key_index,
                                )?;
                            }
                            Ok(())
                        } else {
                            Err(DriverError::InvalidAppKeyIndex)
                        }
                    })
                    .await,
            );

            ctx.send(
                ModelAppMessage::Status(ModelAppStatusMessage {
                    status,
                    payload: bind,
                })
                .into(),
                meta.reply(),
            )
            .await?;

            if let Some(err) = err {
                return Err(err);
            }
        }

        ModelAppMessage::Unbind(unbind) => {
            let composition = storage.composition();
            let (status, err) = convert(
                storage
                    .modify(|config| {
                        if config.secrets().has_application_key(unbind.app_key_index) {
                            if let Some(element_index) = config
                                .device_info()
                                .local_element_index(unbind.element_address.into())
                            {
                                config.bindings_mut().unbind(
                                    composition,
                                    element_index,
                                    unbind.model_identifier,
                                    unbind.app_key_index,
                                )?;
                            }
                            Ok(())
                        } else {
                            Err(DriverError::InvalidAppKeyIndex)
                        }
                    })
                    .await,
            );

            ctx.send(
                ModelAppMessage::Status(ModelAppStatusMessage {
                    status,
                    payload: unbind,
                })
                .into(),
                meta.reply(),
            )
            .await?;

            if let Some(err) = err {
                return Err(err);
            }
        }

        //ModelAppMessage::Status(_) => {}
        _ => {
            // not applicable
        }
    }

    Ok(())
}

fn convert(input: Result<(), ModifyError>) -> (Status, Option<DriverError>) {
    if let Err(result) = input {
        match result {
            ModifyError::Driver(DriverError::InvalidModel) => (Status::InvalidModel, None),
            ModifyError::Driver(DriverError::InvalidAppKeyIndex) => {
                (Status::InvalidAppKeyIndex, None)
            }
            ModifyError::Storage(_) => (Status::StorageFailure, None),
            ModifyError::Driver(inner) => {
                info!("---> {}", inner);
                (Status::UnspecifiedError, Some(inner))
            }
        }
    } else {
        (Status::Success, None)
    }
}
