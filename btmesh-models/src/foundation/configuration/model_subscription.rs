use crate::foundation::configuration::ConfigurationMessage;
use crate::{Message, Status};
use btmesh_common::address::{Address, GroupAddress, LabelUuid, UnicastAddress};
use btmesh_common::opcode::Opcode;
use btmesh_common::{opcode, InsufficientBuffer, ModelIdentifier, ParseError};
use core::convert::TryInto;
use heapless::Vec;
//use serde::{Deserialize, Serialize};

opcode!( CONFIG_MODEL_SUBSCRIPTION_ADD 0x80, 0x1B);
opcode!( CONFIG_MODEL_SUBSCRIPTION_DELETE 0x80, 0x1C);
opcode!( CONFIG_MODEL_SUBSCRIPTION_DELETE_ALL 0x80, 0x1D);
opcode!( CONFIG_MODEL_SUBSCRIPTION_OVERWRITE 0x80, 0x1E);
opcode!( CONFIG_MODEL_SUBSCRIPTION_STATUS 0x80, 0x1F);
opcode!( CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_ADD 0x80, 0x20);
opcode!( CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_DELETE 0x80, 0x21);
opcode!( CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_OVERWRITE 0x80, 0x22);
opcode!( CONFIG_SIG_MODEL_SUBSCRIPTION_GET 0x80, 0x29);
opcode!( CONFIG_SIG_MODEL_SUBSCRIPTION_LIST 0x80, 0x2A);
opcode!( CONFIG_VENDOR_MODEL_SUBSCRIPTION_GET 0x80, 0x2B);
opcode!( CONFIG_VENDOR_MODEL_SUBSCRIPTION_LIST 0x80, 0x2C);

/// Model subscription message.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub enum ModelSubscriptionMessage {
    /// Model Subscription Add is an acknowledged message used to add an address to a Subscription List of a model.
    Add(ModelSubscriptionPayload),
    /// Model Subscription Delete is an acknowledged message used to delete a subscription address from the Subscription List of a model.
    Delete(ModelSubscriptionPayload),
    /// Model Subscription Delete All is an acknowledged message used to discard the Subscription List of a model.
    DeleteAll(ModelSubscriptionDeleteAllMessage),
    /// Model Subscription Overwrite is an acknowledged message used to discard the Subscription List and add an address to the cleared Subscription List of a model.
    Overwrite(ModelSubscriptionPayload),
    /// Model Subscription Status is an unacknowledged message used to report a status of the operation on the Subscription List.
    Status(ModelSubscriptionStatusMessage),
    /// Model Subscription Virtual Address Add is an acknowledged message used to add an address to a Subscription List of a model.
    VirtualAddressAdd(ModelSubscriptionPayload),
    /// Model Subscription Virtual Address Delete is an acknowledged message used to delete a subscription address from the Subscription List of a model.
    VirtualAddressDelete(ModelSubscriptionPayload),
    /// Model Subscription Virtual Address Overwrite is an acknowledged message used to discard the Subscription List
    /// and add an address to the cleared Subscription List of a model.
    VirtualAddressOverwrite(ModelSubscriptionPayload),
    /// Vendor Model Subscription Get is an acknowledged message used to get the list of subscription addresses of a model within the element. This message is only for Vendor Models.
    VendorGet(ModelSubscriptionGetMessage),
    /// Vendor Model Subscription List is an unacknowledged message used to report all addresses from the Subscription List of the model.
    VendorList(ModelSubscriptionListMessage),
    /// SIG Model Subscription Get is an acknowledged message used to get the list of subscription addresses of a model within the element. This message is only for SIG Models.
    SigGet(ModelSubscriptionGetMessage),
    /// SIG Model Subscription List is an unacknowledged message used to report all addresses from the Subscription List of the model.
    SigList(ModelSubscriptionListMessage),
}

impl From<ModelSubscriptionMessage> for ConfigurationMessage {
    fn from(inner: ModelSubscriptionMessage) -> Self {
        Self::ModelSubscription(inner)
    }
}

#[allow(unused)]
impl Message for ModelSubscriptionMessage {
    fn opcode(&self) -> Opcode {
        match self {
            Self::Add(_) => CONFIG_MODEL_SUBSCRIPTION_ADD,
            Self::Delete(_) => CONFIG_MODEL_SUBSCRIPTION_DELETE,
            Self::DeleteAll(_) => CONFIG_MODEL_SUBSCRIPTION_DELETE_ALL,
            Self::Overwrite(_) => CONFIG_MODEL_SUBSCRIPTION_OVERWRITE,
            Self::Status(_) => CONFIG_MODEL_SUBSCRIPTION_STATUS,
            Self::VirtualAddressAdd(_) => CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_ADD,
            Self::VirtualAddressDelete(_) => CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_DELETE,
            Self::VirtualAddressOverwrite(_) => CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_OVERWRITE,
            Self::VendorGet(_) => CONFIG_VENDOR_MODEL_SUBSCRIPTION_GET,
            Self::VendorList(_) => CONFIG_VENDOR_MODEL_SUBSCRIPTION_LIST,
            Self::SigGet(_) => CONFIG_SIG_MODEL_SUBSCRIPTION_GET,
            Self::SigList(_) => CONFIG_SIG_MODEL_SUBSCRIPTION_LIST,
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        match self {
            Self::Add(inner) => inner.emit_parameters(xmit),
            Self::Delete(inner) => inner.emit_parameters(xmit),
            Self::DeleteAll(inner) => inner.emit_parameters(xmit),
            Self::Overwrite(inner) => inner.emit_parameters(xmit),
            Self::Status(inner) => inner.emit_parameters(xmit),
            Self::VirtualAddressAdd(inner) => inner.emit_parameters(xmit),
            Self::VirtualAddressDelete(inner) => inner.emit_parameters(xmit),
            Self::VirtualAddressOverwrite(inner) => inner.emit_parameters(xmit),
            Self::VendorGet(inner) => inner.emit_parameters(xmit),
            Self::VendorList(inner) => inner.emit_parameters(xmit),
            Self::SigGet(inner) => inner.emit_parameters(xmit),
            Self::SigList(inner) => inner.emit_parameters(xmit),
        }
    }
}

impl ModelSubscriptionMessage {
    /// Parses paramateters into Model Subscription Add message.
    pub fn parse_add(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Add(ModelSubscriptionPayload::parse(parameters)?))
    }

    /// Parses paramateters into Model Subscription Virtual Address Add message.
    pub fn parse_virtual_address_add(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Add(ModelSubscriptionPayload::parse_virtual_address(
            parameters,
        )?))
    }

    /// Parses paramateters into Model Subscription Delete message.
    pub fn parse_delete(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Delete(ModelSubscriptionPayload::parse(parameters)?))
    }

    /// Parses paramateters into Model Subscription Virtual Address Delete message.
    pub fn parse_virtual_address_delete(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Delete(
            ModelSubscriptionPayload::parse_virtual_address(parameters)?,
        ))
    }

    /// Parses paramateters into Model Subscription Overwrite message.
    pub fn parse_overwrite(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Overwrite(ModelSubscriptionPayload::parse(
            parameters,
        )?))
    }

    /// Parses paramateters into Model Subscription VirtualAddress Overwrite message.
    pub fn parse_virtual_address_overwrite(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Overwrite(
            ModelSubscriptionPayload::parse_virtual_address(parameters)?,
        ))
    }

    /// Parses paramateters into Model Subscription Delete All message.
    pub fn parse_delete_all(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::DeleteAll(ModelSubscriptionDeleteAllMessage::parse(
            parameters,
        )?))
    }

    /// Parses paramateters into Model Subscription Vendor Get message.
    pub fn parse_vendor_get(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::VendorGet(ModelSubscriptionGetMessage::parse(
            parameters,
        )?))
    }

    /// Parses paramateters into Model Subscription SIG Get message.
    pub fn parse_sig_get(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::SigGet(ModelSubscriptionGetMessage::parse(
            parameters,
        )?))
    }

    /// Parses paramateters into Model Subscription Status message.
    pub fn parse_status(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Status(ModelSubscriptionStatusMessage::parse(
            parameters,
        )?))
    }
}

/// Subscription address.
#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SubscriptionAddress {
    /// Unicast address.
    Unicast(UnicastAddress),
    /// Group address.
    Group(GroupAddress),
    /// Label UUID.
    Label(LabelUuid),
    /// Unassigned value.
    Unassigned,
}

impl TryInto<SubscriptionAddress> for Address {
    type Error = ();

    fn try_into(self) -> Result<SubscriptionAddress, Self::Error> {
        match self {
            Address::Unassigned => Err(()),
            Address::Unicast(inner) => Ok(SubscriptionAddress::Unicast(inner)),
            Address::Virtual(_) => Err(()),
            Address::Group(inner) => Ok(SubscriptionAddress::Group(inner)),
            //Address::LabelUuid(inner) => Ok(SubscriptionAddress::Virtual(inner)),
        }
    }
}

/// Payload of Model Subscribe message.
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ModelSubscriptionPayload {
    /// Address of the element.
    pub element_address: UnicastAddress,
    /// Value of the address.
    pub subscription_address: SubscriptionAddress,
    /// SIG Model ID or Vendor Model ID.
    pub model_identifier: ModelIdentifier,
}

impl ModelSubscriptionPayload {
    /// Parses parameters into Model Subscription message payload.
    pub fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 6 {
            let element_address = UnicastAddress::parse([parameters[1], parameters[0]])?;
            let subscription_address = Address::parse([parameters[3], parameters[2]])
                .try_into()
                .map_err(|_| ParseError::InvalidValue)?;
            let model_identifier = ModelIdentifier::parse(&parameters[4..])?;
            Ok(Self {
                element_address,
                subscription_address,
                model_identifier,
            })
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    /// Parses parameters into virtual address.
    pub fn parse_virtual_address(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 19 {
            let element_address = UnicastAddress::parse([parameters[1], parameters[0]])?;
            let subscription_address =
                SubscriptionAddress::Label(LabelUuid::parse(&parameters[2..=17])?);

            let model_identifier = ModelIdentifier::parse(&parameters[18..])?;
            Ok(Self {
                element_address,
                subscription_address,
                model_identifier,
            })
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    /// Emits payload into array of bytes.
    pub fn emit_parameters<const N: usize>(
        &self,
        _xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        todo!()
    }
}

/// Model Subscription Delete All message.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct ModelSubscriptionDeleteAllMessage {
    /// Address of the element.
    pub element_address: UnicastAddress,
    /// SIG Model ID or Vendor Model ID.
    pub model_identifier: ModelIdentifier,
}

impl ModelSubscriptionDeleteAllMessage {
    /// Parses parameters into Delete All message.
    pub fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 4 {
            let element_address = UnicastAddress::parse([parameters[1], parameters[0]])?;
            let model_identifier = ModelIdentifier::parse(&parameters[2..])?;
            Ok(Self {
                element_address,
                model_identifier,
            })
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    /// Emits Delete All message into array of bytes.
    pub fn emit_parameters<const N: usize>(
        &self,
        _xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        todo!()
    }
}

/// Model Subscription overwrite message.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct ModelSubscriptionOverwriteMessage {
    /// Address of the element.
    pub element_address: UnicastAddress,
    /// Value of the Address.
    pub subscription_address: SubscriptionAddress,
    /// SIG Model ID or Vendor Model ID.
    pub model_identifier: ModelIdentifier,
}

impl ModelSubscriptionOverwriteMessage {
    /// Parses parameters into message.
    pub fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 6 {
            let element_address = UnicastAddress::parse([parameters[1], parameters[0]])?;
            let subscription_address = SubscriptionAddress::Unicast(UnicastAddress::parse([
                parameters[3],
                parameters[2],
            ])?);
            let model_identifier = ModelIdentifier::parse(&parameters[9..])?;
            Ok(Self {
                element_address,
                subscription_address,
                model_identifier,
            })
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    /// Parses parameters into virtual address.
    pub fn parse_virtual_address(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 19 {
            let element_address = UnicastAddress::parse([parameters[1], parameters[0]])?;
            let subscription_address =
                SubscriptionAddress::Label(LabelUuid::parse(&parameters[2..=17])?);

            let model_identifier = ModelIdentifier::parse(&parameters[18..])?;
            Ok(Self {
                element_address,
                subscription_address,
                model_identifier,
            })
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    /// Emits message into array of bytes.
    pub fn emit_parameters<const N: usize>(
        &self,
        _xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        todo!()
    }
}

/// Model subscription status message.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct ModelSubscriptionStatusMessage {
    /// Status Code for the requesting message.
    pub status: Status,
    /// Address of the element.
    pub element_address: UnicastAddress,
    /// Value of the address.
    pub subscription_address: SubscriptionAddress,
    /// SIG Model ID or Vendor Model ID.
    pub model_identifier: ModelIdentifier,
}

impl ModelSubscriptionStatusMessage {
    /// Parses parameters into message.
    pub fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        let status: Status = parameters[0].try_into()?;
        let element_address = UnicastAddress::parse([parameters[2], parameters[1]])?;
        let subscription_address: Address = Address::parse([parameters[4], parameters[3]]);
        let model_identifier: ModelIdentifier = ModelIdentifier::parse(&parameters[5..])?;
        Ok(Self {
            status,
            element_address,
            subscription_address: subscription_address.try_into()?,
            model_identifier,
        })
    }

    /// Emits message into array of bytes.
    pub fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        xmit.push(self.status as u8)
            .map_err(|_| InsufficientBuffer)?;
        let addr_bytes = self.element_address.as_bytes();
        xmit.push(addr_bytes[1]).map_err(|_| InsufficientBuffer)?;
        xmit.push(addr_bytes[0]).map_err(|_| InsufficientBuffer)?;
        match self.subscription_address {
            SubscriptionAddress::Unicast(addr) => {
                let addr_bytes = addr.as_bytes();
                xmit.push(addr_bytes[1]).map_err(|_| InsufficientBuffer)?;
                xmit.push(addr_bytes[0]).map_err(|_| InsufficientBuffer)?;
            }
            SubscriptionAddress::Group(addr) => {
                let addr_bytes = addr.as_bytes();
                xmit.push(addr_bytes[1]).map_err(|_| InsufficientBuffer)?;
                xmit.push(addr_bytes[0]).map_err(|_| InsufficientBuffer)?;
            }
            SubscriptionAddress::Label(addr) => {
                let addr_bytes = addr.virtual_address().as_bytes();
                xmit.push(addr_bytes[1]).map_err(|_| InsufficientBuffer)?;
                xmit.push(addr_bytes[0]).map_err(|_| InsufficientBuffer)?;
            }
            SubscriptionAddress::Unassigned => {
                xmit.push(0).map_err(|_| InsufficientBuffer)?;
                xmit.push(0).map_err(|_| InsufficientBuffer)?;
            }
        }
        self.model_identifier.emit(xmit)?;
        Ok(())
    }
}

/// Model Subscription Get message.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct ModelSubscriptionGetMessage {
    /// Address of the element.
    pub element_address: UnicastAddress,
    /// SIG Model ID.
    pub model_identifier: ModelIdentifier,
}

impl ModelSubscriptionGetMessage {
    /// Emits message into array of bytes.
    pub fn emit_parameters<const N: usize>(
        &self,
        _xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        todo!()
    }

    /// Parses parameters into message.
    pub fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 4 {
            let element_address = UnicastAddress::parse([parameters[1], parameters[0]])?;
            let model_identifier = ModelIdentifier::parse(&parameters[2..])?;
            Ok(Self {
                element_address,
                model_identifier,
            })
        } else {
            Err(ParseError::InvalidLength)
        }
    }
}

/// Model Subscription List message.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct ModelSubscriptionListMessage {
    /// Status Code for the requesting message.
    pub status: Status,
    /// Address of the element.
    pub element_address: UnicastAddress,
    /// SIG Model ID.
    pub model_identifier: ModelIdentifier,
    /// A block of all addresses from the Subscription List.
    pub addresses: Vec<SubscriptionAddress, 8>,
}

impl ModelSubscriptionListMessage {
    /// Emits message into array of bytes.
    pub fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        xmit.push(self.status as u8)
            .map_err(|_| InsufficientBuffer)?;
        let addr_bytes = self.element_address.as_bytes();
        xmit.push(addr_bytes[1]).map_err(|_| InsufficientBuffer)?;
        xmit.push(addr_bytes[0]).map_err(|_| InsufficientBuffer)?;
        self.model_identifier.emit(xmit)?;

        for address in &self.addresses {
            match address {
                SubscriptionAddress::Unicast(addr) => {
                    let addr_bytes = addr.as_bytes();
                    xmit.push(addr_bytes[1]).map_err(|_| InsufficientBuffer)?;
                    xmit.push(addr_bytes[0]).map_err(|_| InsufficientBuffer)?;
                }
                SubscriptionAddress::Group(_addr) => {
                    todo!("group address")
                }
                SubscriptionAddress::Label(addr) => {
                    let addr_bytes = addr.virtual_address().as_bytes();
                    xmit.push(addr_bytes[1]).map_err(|_| InsufficientBuffer)?;
                    xmit.push(addr_bytes[0]).map_err(|_| InsufficientBuffer)?;
                }
                SubscriptionAddress::Unassigned => {
                    // not valid in this context
                }
            }
        }
        Ok(())
    }
}
