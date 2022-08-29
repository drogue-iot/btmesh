use crate::models::configuration::convert;
use crate::{BackingStore, DriverError, Storage};
use btmesh_common::address::UnicastAddress;
use btmesh_common::ModelIdentifier;
use btmesh_device::{BluetoothMeshModelContext, InboundMetadata};
use btmesh_models::foundation::configuration::model_subscription::{
    ModelSubscriptionListMessage, ModelSubscriptionMessage, ModelSubscriptionStatusMessage,
    SubscriptionAddress,
};
use btmesh_models::foundation::configuration::ConfigurationServer;
use btmesh_models::Status;
use heapless::Vec;

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: &ModelSubscriptionMessage,
    meta: &InboundMetadata,
) -> Result<(), DriverError> {
    match &message {
        ModelSubscriptionMessage::Add(add) | ModelSubscriptionMessage::VirtualAddressAdd(add) => {
            let composition = storage.composition();

            let status_err = convert(
                &storage
                    .modify_provisioned(|config| {
                        if let Some(element_index) = config
                            .device_info()
                            .local_element_index(add.element_address.into())
                        {
                            config.subscriptions_mut().add(
                                composition.as_ref().unwrap(),
                                element_index,
                                add.model_identifier,
                                add.subscription_address,
                            )?;
                            Ok(())
                        } else {
                            Err(DriverError::InvalidElementAddress)
                        }
                    })
                    .await,
            );

            return respond(
                ctx,
                status_err,
                &add.element_address,
                &add.subscription_address,
                &add.model_identifier,
                meta,
            )
            .await;
        }

        ModelSubscriptionMessage::Delete(delete)
        | ModelSubscriptionMessage::VirtualAddressDelete(delete) => {
            let status_err = convert(
                &storage
                    .modify_provisioned(|config| {
                        if let Some(element_index) = config
                            .device_info()
                            .local_element_index(delete.element_address.into())
                        {
                            config.subscriptions_mut().delete(
                                element_index,
                                delete.model_identifier,
                                delete.subscription_address,
                            )?;
                            Ok(())
                        } else {
                            Err(DriverError::InvalidElementAddress)
                        }
                    })
                    .await,
            );

            return respond(
                ctx,
                status_err,
                &delete.element_address,
                &delete.subscription_address,
                &delete.model_identifier,
                meta,
            )
            .await;
        }
        ModelSubscriptionMessage::DeleteAll(delete_all) => {
            let status_err = convert(
                &storage
                    .modify_provisioned(|config| {
                        if let Some(element_index) = config
                            .device_info()
                            .local_element_index(delete_all.element_address.into())
                        {
                            config
                                .subscriptions_mut()
                                .delete_all(element_index, delete_all.model_identifier)?;
                            Ok(())
                        } else {
                            Err(DriverError::InvalidElementAddress)
                        }
                    })
                    .await,
            );

            return respond(
                ctx,
                status_err,
                &delete_all.element_address,
                &SubscriptionAddress::Unassigned,
                &delete_all.model_identifier,
                meta,
            )
            .await;
        }
        ModelSubscriptionMessage::Overwrite(overwrite)
        | ModelSubscriptionMessage::VirtualAddressOverwrite(overwrite) => {
            let composition = storage.composition();
            let status_err = convert(
                &storage
                    .modify_provisioned(|config| {
                        if let Some(element_index) = config
                            .device_info()
                            .local_element_index(overwrite.element_address.into())
                        {
                            config.subscriptions_mut().delete(
                                element_index,
                                overwrite.model_identifier,
                                overwrite.subscription_address,
                            )?;
                            config.subscriptions_mut().add(
                                composition.as_ref().unwrap(),
                                element_index,
                                overwrite.model_identifier,
                                overwrite.subscription_address,
                            )?;
                            Ok(())
                        } else {
                            Err(DriverError::InvalidElementAddress)
                        }
                    })
                    .await,
            );

            return respond(
                ctx,
                status_err,
                &overwrite.element_address,
                &overwrite.subscription_address,
                &overwrite.model_identifier,
                meta,
            )
            .await;
        }
        ModelSubscriptionMessage::VendorGet(get) | ModelSubscriptionMessage::SigGet(get) => {
            let result = storage
                .read_provisioned(|config| {
                    if let Some(element_index) = config
                        .device_info()
                        .local_element_index(get.element_address.into())
                    {
                        let subscriptions: Vec<_, 4> = config
                            .subscriptions()
                            .get(element_index, get.model_identifier);
                        Ok(subscriptions)
                    } else {
                        Err(DriverError::InvalidElementAddress)
                    }
                })
                .await;

            match result {
                Ok(addresses) => {
                    match message {
                        ModelSubscriptionMessage::VendorGet(_) => {
                            ctx.send(
                                ModelSubscriptionMessage::VendorList(
                                    ModelSubscriptionListMessage {
                                        status: Status::Success,
                                        element_address: get.element_address,
                                        model_identifier: get.model_identifier,
                                        addresses: Vec::from_slice(&addresses)?,
                                    },
                                )
                                .into(),
                                meta.reply(),
                            )
                            .await?;
                        }
                        ModelSubscriptionMessage::SigGet(_) => {
                            ctx.send(
                                ModelSubscriptionMessage::SigList(ModelSubscriptionListMessage {
                                    status: Status::Success,
                                    element_address: get.element_address,
                                    model_identifier: get.model_identifier,
                                    addresses: Vec::from_slice(&addresses)?,
                                })
                                .into(),
                                meta.reply(),
                            )
                            .await?;
                        }
                        _ => {
                            // neither of those two
                        }
                    }
                }
                Err(DriverError::Storage(inner)) => {
                    ctx.send(
                        ModelSubscriptionMessage::SigList(ModelSubscriptionListMessage {
                            status: Status::StorageFailure,
                            element_address: get.element_address,
                            model_identifier: get.model_identifier,
                            addresses: Default::default(),
                        })
                        .into(),
                        meta.reply(),
                    )
                    .await?;
                    return Err(inner.into());
                }
                Err(inner) => {
                    ctx.send(
                        ModelSubscriptionMessage::SigList(ModelSubscriptionListMessage {
                            status: Status::UnspecifiedError,
                            element_address: get.element_address,
                            model_identifier: get.model_identifier,
                            addresses: Default::default(),
                        })
                        .into(),
                        meta.reply(),
                    )
                    .await?;

                    return Err(inner);
                }
            }
            return Ok(());
        }
        ModelSubscriptionMessage::Status(_) => {
            // not applicable
        }
        ModelSubscriptionMessage::VendorList(_) => {
            // not applicable
        }
        ModelSubscriptionMessage::SigList(_) => {
            // not applicable
        }
    }
    Ok(())
}

async fn respond<C: BluetoothMeshModelContext<ConfigurationServer>>(
    ctx: &C,
    (status, err): (Status, Option<DriverError>),
    element_address: &UnicastAddress,
    subscription_address: &SubscriptionAddress,
    model_identifier: &ModelIdentifier,
    meta: &InboundMetadata,
) -> Result<(), DriverError> {
    ctx.send(
        ModelSubscriptionMessage::Status(ModelSubscriptionStatusMessage {
            status,
            element_address: *element_address,
            subscription_address: *subscription_address,
            model_identifier: *model_identifier,
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
