use crate::models::configuration::convert;
use crate::{BackingStore, DriverError, Storage};
use btmesh_device::{BluetoothMeshModelContext, InboundMetadata};
use btmesh_models::foundation::configuration::model_publication::{
    ModelPublicationMessage, ModelPublicationStatusMessage, PublicationDetails, PublishAddress,
};
use btmesh_models::foundation::configuration::{AppKeyIndex, ConfigurationServer};
use btmesh_models::Status;

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: &ModelPublicationMessage,
    meta: &InboundMetadata,
) -> Result<(), DriverError> {
    match message {
        ModelPublicationMessage::Get(get) => {
            let result = storage
                .read_provisioned(|config| {
                    if let Some(element_index) = config
                        .device_info()
                        .local_element_index(get.element_address.into())
                    {
                        Ok(config
                            .publications()
                            .get(element_index, get.model_identifier))
                    } else {
                        Err(DriverError::InvalidElementAddress)
                    }
                })
                .await;

            let ((status, err), details) = match result {
                Err(err) => (
                    convert(Err(err)),
                    PublicationDetails {
                        element_address: get.element_address,
                        publish_address: PublishAddress::Unassigned,
                        app_key_index: AppKeyIndex::new(0),
                        credential_flag: false,
                        publish_ttl: None,
                        publish_period: 0,
                        publish_retransmit_count: 0,
                        publish_retransmit_interval_steps: 0,
                        model_identifier: get.model_identifier,
                    },
                ),
                Ok(None) => (
                    (Status::Success, None),
                    PublicationDetails {
                        element_address: get.element_address,
                        publish_address: PublishAddress::Unassigned,
                        app_key_index: AppKeyIndex::new(0),
                        credential_flag: false,
                        publish_ttl: None,
                        publish_period: 0,
                        publish_retransmit_count: 0,
                        publish_retransmit_interval_steps: 0,
                        model_identifier: get.model_identifier,
                    },
                ),
                Ok(Some(publication)) => (
                    (Status::Success, None),
                    PublicationDetails {
                        element_address: get.element_address,
                        publish_address: publication.publish_address,
                        app_key_index: publication.app_key_index,
                        credential_flag: publication.credential_flag,
                        publish_ttl: publication.publish_ttl,
                        publish_period: publication.publish_period,
                        publish_retransmit_count: publication.publish_retransmit_count,
                        publish_retransmit_interval_steps: publication
                            .publish_retransmit_interval_steps,
                        model_identifier: get.model_identifier,
                    },
                ),
            };

            info!("+++++ {} {} {}", status, err, details);

            ctx.send(
                ModelPublicationMessage::Status(ModelPublicationStatusMessage { status, details })
                    .into(),
                meta.reply(),
            )
            .await?;

            if let Some(err) = err {
                Err(err)
            } else {
                Ok(())
            }
        }
        ModelPublicationMessage::Set(set) | ModelPublicationMessage::VirtualAddressSet(set) => {
            let composition = storage.composition();

            let (status, err) = convert(
                storage
                    .modify_provisioned(|config| {
                        if let Some(element_index) = config
                            .device_info()
                            .local_element_index(set.details.element_address.into())
                        {
                            config.publications_mut().set(
                                &composition,
                                element_index,
                                set.details,
                            )?;
                            Ok(())
                        } else {
                            Err(DriverError::InvalidElementAddress)
                        }
                    })
                    .await,
            );

            ctx.send(
                ModelPublicationMessage::Status(ModelPublicationStatusMessage {
                    status,
                    details: set.details,
                })
                .into(),
                meta.reply(),
            )
            .await?;

            if let Some(err) = err {
                Err(err)
            } else {
                Ok(())
            }
        }
        ModelPublicationMessage::Status(_) => {
            // not applicable
            Ok(())
        }
    }
}
