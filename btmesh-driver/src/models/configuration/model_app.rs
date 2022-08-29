use crate::models::configuration::convert;
use crate::{BackingStore, DriverError, Storage};
use btmesh_device::{BluetoothMeshModelContext, InboundMetadata};
use btmesh_models::foundation::configuration::model_app::{ModelAppMessage, ModelAppStatusMessage};
use btmesh_models::foundation::configuration::ConfigurationServer;

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: &ModelAppMessage,
    meta: &InboundMetadata,
) -> Result<(), DriverError> {
    match message {
        ModelAppMessage::Bind(bind) => {
            let composition = storage.composition();
            let (status, err) = convert(
                &storage
                    .modify_provisioned(|config| {
                        if config.secrets().has_application_key(bind.app_key_index) {
                            if let Some(element_index) = config
                                .device_info()
                                .local_element_index(bind.element_address.into())
                            {
                                config.bindings_mut().bind(
                                    composition.as_ref().unwrap(),
                                    element_index,
                                    bind.model_identifier,
                                    bind.app_key_index,
                                )?;
                            }
                            Ok(())
                        } else {
                            Err(DriverError::InvalidElementAddress)
                        }
                    })
                    .await,
            );

            ctx.send(
                ModelAppMessage::Status(ModelAppStatusMessage {
                    status,
                    payload: *bind,
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
                &storage
                    .modify_provisioned(|config| {
                        if config.secrets().has_application_key(unbind.app_key_index) {
                            if let Some(element_index) = config
                                .device_info()
                                .local_element_index(unbind.element_address.into())
                            {
                                config.bindings_mut().unbind(
                                    composition.as_ref().unwrap(),
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
                    payload: *unbind,
                })
                .into(),
                meta.reply(),
            )
            .await?;

            if let Some(err) = err {
                return Err(err);
            }
        }

        ModelAppMessage::Status(_) => {
            // not applicable
        }
    }

    Ok(())
}
