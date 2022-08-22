use heapless::Vec;
use crate::storage::ModifyError;
use crate::{BackingStore, DriverError, Storage};
use btmesh_common::address::UnicastAddress;
use btmesh_common::ModelIdentifier;
use btmesh_device::{BluetoothMeshModelContext, InboundMetadata};
use btmesh_models::foundation::configuration::model_subscription::{ModelSubscriptionListMessage, ModelSubscriptionMessage, ModelSubscriptionStatusMessage, SubscriptionAddress};
use btmesh_models::foundation::configuration::ConfigurationServer;
use btmesh_models::Status;

pub async fn dispatch<C: BluetoothMeshModelContext<ConfigurationServer>, B: BackingStore>(
    ctx: &C,
    storage: &Storage<B>,
    message: ModelSubscriptionMessage,
    meta: InboundMetadata,
) -> Result<(), DriverError> {
    match &message {
        ModelSubscriptionMessage::Add(add) | ModelSubscriptionMessage::VirtualAddressAdd(add) => {
            let composition = storage.composition();

            let status_err = convert(
                storage
                    .modify(|config| {
                        if let Some(element_index) = config
                            .device_info()
                            .local_element_index(add.element_address.into())
                        {
                            config.subscriptions_mut().add(
                                composition,
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
                add.element_address,
                add.subscription_address,
                add.model_identifier,
                meta,
            )
            .await;
        }

        ModelSubscriptionMessage::Delete(delete)
        | ModelSubscriptionMessage::VirtualAddressDelete(delete) => {
            let status_err = convert(
                storage
                    .modify(|config| {
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
                delete.element_address,
                delete.subscription_address,
                delete.model_identifier,
                meta,
            )
            .await;
        }
        ModelSubscriptionMessage::DeleteAll(delete_all) => {
            let status_err = convert(
                storage
                    .modify(|config| {
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
                delete_all.element_address,
                SubscriptionAddress::Unassigned,
                delete_all.model_identifier,
                meta,
            )
            .await;
        }
        ModelSubscriptionMessage::Overwrite(overwrite)
        | ModelSubscriptionMessage::VirtualAddressOverwrite(overwrite) => {
            let composition = storage.composition();
            let status_err = convert(
                storage
                    .modify(|config| {
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
                                composition,
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
                overwrite.element_address,
                overwrite.subscription_address,
                overwrite.model_identifier,
                meta,
            )
            .await;
        }
        ModelSubscriptionMessage::VendorGet(get) | ModelSubscriptionMessage::SigGet(get) => {
            let result = storage
                .read(|config| {
                    if let Some(element_index) = config
                        .device_info()
                        .local_element_index(get.element_address.into())
                    {
                        Ok(config
                            .subscriptions()
                            .get(element_index, get.model_identifier))
                    } else {
                        Err(DriverError::InvalidElementAddress)
                    }
                })
                .await;

            match result {
                Ok(addresses) => {
                    match message {
                        ModelSubscriptionMessage::VendorGet(_) => {
                                ctx.send( ModelSubscriptionMessage::VendorList(
                                    ModelSubscriptionListMessage {
                                        status: Status::Success,
                                        element_address: get.element_address,
                                        model_identifier: get.model_identifier,
                                        addresses: Vec::from_slice( &*addresses )?,
                                    }
                                ).into(), meta.reply()).await?;
                        }
                        ModelSubscriptionMessage::SigGet(_) => {
                            ctx.send( ModelSubscriptionMessage::SigList(
                                ModelSubscriptionListMessage {
                                    status: Status::Success,
                                    element_address: get.element_address,
                                    model_identifier: get.model_identifier,
                                    addresses: Vec::from_slice( &*addresses )?,
                                }
                            ).into(), meta.reply()).await?;
                        }
                        _=> {
                            // neither of those two
                        }
                    }
                }
                Err(DriverError::Storage(inner)) => {
                    ctx.send( ModelSubscriptionMessage::SigList(
                        ModelSubscriptionListMessage {
                            status: Status::StorageFailure,
                            element_address: get.element_address,
                            model_identifier: get.model_identifier,
                            addresses: Default::default(),
                        }
                    ).into(), meta.reply()).await?;
                    return Err(inner.into());
                }
                Err(inner) => {
                    ctx.send( ModelSubscriptionMessage::SigList(
                        ModelSubscriptionListMessage {
                            status: Status::UnspecifiedError,
                            element_address: get.element_address,
                            model_identifier: get.model_identifier,
                            addresses: Default::default(),
                        }
                    ).into(), meta.reply()).await?;

                   return  Err(inner)
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
    element_address: UnicastAddress,
    subscription_address: SubscriptionAddress,
    model_identifier: ModelIdentifier,
    meta: InboundMetadata,
) -> Result<(), DriverError> {
    ctx.send(
        ModelSubscriptionMessage::Status(ModelSubscriptionStatusMessage {
            status,
            element_address,
            subscription_address,
            model_identifier,
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

fn convert(input: Result<(), ModifyError>) -> (Status, Option<DriverError>) {
    if let Err(result) = input {
        match result {
            ModifyError::Driver(DriverError::InvalidElementAddress) => {
                (Status::InvalidAddress, None)
            }
            ModifyError::Driver(DriverError::InvalidModel) => (Status::InvalidModel, None),
            ModifyError::Driver(DriverError::InvalidAppKeyIndex) => {
                (Status::InvalidAppKeyIndex, None)
            }
            ModifyError::Storage(_) => (Status::StorageFailure, None),
            ModifyError::Driver(inner) => (Status::UnspecifiedError, Some(inner)),
        }
    } else {
        (Status::Success, None)
    }
}
