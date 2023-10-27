use crate::foundation::configuration::{AppKeyIndex, ConfigurationMessage, KeyIndex};
use crate::{Message, Status};
use btmesh_common::address::Address;
use btmesh_common::address::{GroupAddress, LabelUuid, UnicastAddress, VirtualAddress};
use btmesh_common::opcode::Opcode;
use btmesh_common::{opcode, InsufficientBuffer, ModelIdentifier, ParseError, Ttl};
use heapless::Vec;

opcode!( CONFIG_MODEL_PUBLICATION_SET 0x03 );
opcode!( CONFIG_MODEL_PUBLICATION_GET 0x80, 0x18);
opcode!( CONFIG_MODEL_PUBLICATION_STATUS 0x80, 0x19);
opcode!( CONFIG_MODEL_PUBLICATION_VIRTUAL_ADDRESS_SET 0x80, 0x1A);

/// Model publication message.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub enum ModelPublicationMessage {
    /// Model Publication Get message.
    Get(ModelPublicationGetMessage),
    /// Model Publication Set message.
    Set(ModelPublicationSetMessage),
    /// Model Publication Virtual Address Set message.
    VirtualAddressSet(ModelPublicationSetMessage),
    /// Model Publication Status message.
    Status(ModelPublicationStatusMessage),
}

impl From<ModelPublicationMessage> for ConfigurationMessage {
    fn from(inner: ModelPublicationMessage) -> Self {
        ConfigurationMessage::ModelPublication(inner)
    }
}

impl Message for ModelPublicationMessage {
    fn opcode(&self) -> Opcode {
        match self {
            ModelPublicationMessage::Get(_) => CONFIG_MODEL_PUBLICATION_GET,
            ModelPublicationMessage::Set(_) => CONFIG_MODEL_PUBLICATION_SET,
            ModelPublicationMessage::VirtualAddressSet(_) => {
                CONFIG_MODEL_PUBLICATION_VIRTUAL_ADDRESS_SET
            }
            ModelPublicationMessage::Status(_) => CONFIG_MODEL_PUBLICATION_STATUS,
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        match self {
            ModelPublicationMessage::Get(inner) => inner.emit_parameters(xmit),
            ModelPublicationMessage::Set(inner) => inner.emit_parameters(xmit),
            ModelPublicationMessage::VirtualAddressSet(inner) => inner.emit_parameters(xmit),
            ModelPublicationMessage::Status(inner) => inner.emit_parameters(xmit),
        }
    }
}

impl ModelPublicationMessage {
    /// Parses parameters into Model Publication Set message.
    pub fn parse_set(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Set(ModelPublicationSetMessage::parse(parameters)?))
    }

    /// Parses parameters into Model Publication Virtual Address Set message.
    pub fn parse_virtual_address_set(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Set(
            ModelPublicationSetMessage::parse_virtual_address(parameters)?,
        ))
    }

    /// Parses parameters into Model Publication Get message.
    pub fn parse_get(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Get(ModelPublicationGetMessage::parse(parameters)?))
    }

    /// Parses parameters into Model Publication Status message.
    pub fn parse_status(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Status(ModelPublicationStatusMessage::parse(
            parameters,
        )?))
    }
}

/// Model Publication Get is an acknowledged message used to get the publish address
/// and parameters of an outgoing message that originates from a model.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct ModelPublicationGetMessage {
    /// Address of the element.
    pub element_address: UnicastAddress,
    /// SIG Model ID or Vendor Model ID.
    pub model_identifier: ModelIdentifier,
}

impl ModelPublicationGetMessage {
    fn emit_parameters<const N: usize>(
        &self,
        _xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        todo!()
    }

    /// Parses parameters into Model Publication Get message.
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

/// Publish Address state determines the destination address in messages sent by a model.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PublishAddress {
    /// Unicast address.
    Unicast(UnicastAddress),
    /// Group address.
    Group(GroupAddress),
    /// Label UUID.
    Label(LabelUuid),
    /// Virtual address.
    Virtual(VirtualAddress),
    /// Unassigned value.
    Unassigned,
}

impl From<Address> for PublishAddress {
    fn from(address: Address) -> Self {
        match address {
            Address::Unicast(addr) => PublishAddress::Unicast(addr),
            Address::Virtual(addr) => PublishAddress::Virtual(addr),
            Address::Group(addr) => PublishAddress::Group(addr),
            Address::Unassigned => PublishAddress::Unassigned,
        }
    }
}

/// Model Publication Set is an acknowledged message used to set the Model Publication
/// state of an outgoing message that originates from a model.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct ModelPublicationSetMessage {
    /// Publication Set message details.
    pub details: PublicationDetails,
}

impl ModelPublicationSetMessage {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        self.details.emit_parameters(xmit)?;
        Ok(())
    }

    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self {
            details: PublicationDetails::parse(parameters)?,
        })
    }

    fn parse_virtual_address(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self {
            details: PublicationDetails::parse_virtual_address(parameters)?,
        })
    }
}

/// Model Publication Status is an unacknowledged message used to report the model Publication state
/// of an outgoing message that is published by the model.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct ModelPublicationStatusMessage {
    /// Status Code for the requesting message.
    pub status: Status,
    /// Publication details.
    pub details: PublicationDetails,
}

impl ModelPublicationStatusMessage {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        xmit.push(self.status as u8)
            .map_err(|_| InsufficientBuffer)?;
        self.details.emit_parameters(xmit)?;
        Ok(())
    }

    /// Parses parameters into Model Publication status message.
    pub fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        let status: Status = parameters[0].try_into()?;
        let details: PublicationDetails = PublicationDetails::parse(&parameters[1..])?;
        Ok(Self { status, details })
    }
}

/// Publication details.
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Copy, Clone, Eq, Debug, PartialEq, Hash)]
pub struct PublicationDetails {
    /// Address of the element.
    pub element_address: UnicastAddress,
    /// Value of the publish address.
    pub publish_address: PublishAddress,
    /// Index of the application key.
    pub app_key_index: AppKeyIndex,
    /// Value of the Friendship Credential Flag.
    pub credential_flag: bool,
    /// Default TTL value for the outgoing messages.
    pub publish_ttl: Option<Ttl>,
    /// Period for periodic status publishing.
    pub publish_period: PublishPeriod,
    /// Retransmissions configuration for each published message.
    pub publish_retransmit: PublishRetransmit,
    /// SIG Model ID or Vendor Model ID
    pub model_identifier: ModelIdentifier,
}

impl PublicationDetails {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        let addr_bytes = self.element_address.as_bytes();
        xmit.push(addr_bytes[1]).map_err(|_| InsufficientBuffer)?;
        xmit.push(addr_bytes[0]).map_err(|_| InsufficientBuffer)?;
        match self.publish_address {
            PublishAddress::Unicast(addr) => {
                let addr_bytes = addr.as_bytes();
                xmit.push(addr_bytes[1]).map_err(|_| InsufficientBuffer)?;
                xmit.push(addr_bytes[0]).map_err(|_| InsufficientBuffer)?;
            }
            PublishAddress::Group(_addr) => {
                todo!("group address")
            }
            PublishAddress::Label(addr) => {
                xmit.extend_from_slice(addr.label_uuid())
                    .map_err(|_| InsufficientBuffer)?;
            }
            PublishAddress::Unassigned => {
                xmit.push(0).map_err(|_| InsufficientBuffer)?;
                xmit.push(0).map_err(|_| InsufficientBuffer)?;
            }
            PublishAddress::Virtual(addr) => {
                let addr_bytes = addr.as_bytes();
                xmit.push(addr_bytes[1]).map_err(|_| InsufficientBuffer)?;
                xmit.push(addr_bytes[0]).map_err(|_| InsufficientBuffer)?;
            }
        }
        self.app_key_index.emit(xmit)?;
        if self.credential_flag {
            if let Some(last) = xmit.last_mut() {
                *last |= 0b00001000;
            } else {
                return Err(InsufficientBuffer);
            }
        }
        if let Some(ttl) = self.publish_ttl {
            xmit.push(ttl.value()).map_err(|_| InsufficientBuffer)?;
        } else {
            xmit.push(0xFF).map_err(|_| InsufficientBuffer)?;
        }
        xmit.push(u8::from(self.publish_period))
            .map_err(|_| InsufficientBuffer)?;

        xmit.push(u8::from(self.publish_retransmit))
            .map_err(|_| InsufficientBuffer)?;
        self.model_identifier.emit(xmit)?;
        Ok(())
    }

    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 11 {
            let element_address = UnicastAddress::parse([parameters[1], parameters[0]])?;
            let publish_address =
                PublishAddress::from(Address::parse([parameters[3], parameters[2]]));
            let app_key_index = AppKeyIndex(KeyIndex::parse_one(&parameters[4..=5])?);
            let credential_flag = (parameters[5] & 0b0001000) != 0;
            let publish_ttl = parameters[6];
            let publish_ttl = if publish_ttl == 0xFF {
                None
            } else {
                Some(Ttl::new(publish_ttl))
            };
            let publish_period = PublishPeriod::from(parameters[7]);
            let publish_retransmit = PublishRetransmit::from(parameters[8]);
            let model_identifier = ModelIdentifier::parse(&parameters[9..])?;
            Ok(Self {
                element_address,
                publish_address,
                app_key_index,
                credential_flag,
                publish_ttl,
                publish_period,
                publish_retransmit,
                model_identifier,
            })
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    fn parse_virtual_address(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 25 {
            let element_address = UnicastAddress::parse([parameters[1], parameters[0]])?;
            let publish_address = PublishAddress::Label(LabelUuid::parse(&parameters[2..=17])?);

            let app_key_index = AppKeyIndex(KeyIndex::parse_one(&parameters[18..=19])?);
            let credential_flag = (parameters[19] & 0b0001000) != 0;
            let publish_ttl = parameters[20];
            let publish_ttl = if publish_ttl == 0xFF {
                None
            } else {
                Some(Ttl::new(publish_ttl))
            };
            let publish_period = PublishPeriod::from(parameters[21]);
            let publish_retransmit = PublishRetransmit::from(parameters[22]);
            let model_identifier = ModelIdentifier::parse(&parameters[23..])?;
            Ok(Self {
                element_address,
                publish_address,
                app_key_index,
                credential_flag,
                publish_ttl,
                publish_period,
                publish_retransmit,
                model_identifier,
            })
        } else {
            Err(ParseError::InvalidLength)
        }
    }
}

/// Retransmit resolution.
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Copy, Clone, Eq, Debug, PartialEq, Hash)]
pub enum Resolution {
    /// 100 miliseconds.
    Milliseconds100 = 0b00,
    /// Seconds.
    Seconds1 = 0b01,
    /// 10 Seconds.
    Seconds10 = 0b10,
    /// 10 Minutes.
    Minutes10 = 0b11,
}

impl Resolution {
    fn from_u8(value: u8) -> Resolution {
        match value {
            0b00 => Resolution::Milliseconds100,
            0b01 => Resolution::Seconds1,
            0b10 => Resolution::Seconds10,
            0b11 => Resolution::Minutes10,
            _ => unreachable!(),
        }
    }
}

/// Period for periodic status publishing.
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Copy, Clone, Eq, Debug, PartialEq, Hash)]
pub struct PublishPeriod {
    period: u8,
}

impl PublishPeriod {
    /// Creates new publish period using steps and resolution.
    pub fn new(steps: u8, resolution: Resolution) -> Self {
        Self {
            period: (resolution as u8) << 6 | steps,
        }
    }

    /// Returns resolution in period.
    /// The step resolution is stored in bits 6 and 7 of the Publish Period byte.
    pub fn resolution(&self) -> Resolution {
        let resolution = (self.period & 0b11000000) >> 6;
        match resolution {
            0b00 => Resolution::Milliseconds100,
            0b01 => Resolution::Seconds1,
            0b10 => Resolution::Seconds10,
            0b11 => Resolution::Minutes10,
            _ => unreachable!(),
        }
    }

    /// Returns steps in period.
    /// The period number of steps is stored in bits 0 to 5 of the Publish Period byte.
    pub fn steps(&self) -> u8 {
        self.period & 0b00111111
    }
}

impl From<PublishPeriod> for u8 {
    fn from(val: PublishPeriod) -> Self {
        val.period
    }
}

impl From<u8> for PublishPeriod {
    fn from(period: u8) -> Self {
        Self { period }
    }
}

/// Publish retransmit configuration.
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Copy, Clone, Eq, Debug, PartialEq, Hash)]
pub struct PublishRetransmit {
    retransmit: u8,
}

impl PublishRetransmit {
    /// Creates new publish retransmit using count and steps.
    pub fn new(count: u8, interval_steps: u8) -> Self {
        Self {
            retransmit: (count << 5) | (interval_steps & 0b00011111),
        }
    }
    /// Creates new publish period from u8.
    pub fn from_u8(retransmit: u8) -> Self {
        Self { retransmit }
    }

    /// Returns publish retransmit count.
    pub fn count(&self) -> u8 {
        self.retransmit >> 5
    }

    /// Returns publish retransmit steps.
    pub fn interval_steps(&self) -> u8 {
        self.retransmit & 0b00011111
    }
}

impl From<u8> for PublishRetransmit {
    fn from(retransmit: u8) -> Self {
        PublishRetransmit::from_u8(retransmit)
    }
}

impl From<PublishRetransmit> for u8 {
    fn from(val: PublishRetransmit) -> Self {
        val.retransmit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pubset_details() {
        let data = PublicationDetails {
            element_address: UnicastAddress::new(0x00aa).unwrap(),
            publish_address: PublishAddress::Label(
                LabelUuid::new([
                    0xf0, 0xbf, 0xd8, 0x03, 0xcd, 0xe1, 0x84, 0x13, 0x30, 0x96, 0xf0, 0x03, 0xea,
                    0x4a, 0x3d, 0xc2,
                ])
                .unwrap(),
            ),
            app_key_index: AppKeyIndex::new(1),
            credential_flag: false,
            publish_ttl: None,
            publish_period: PublishPeriod::from(0x29),
            publish_retransmit: PublishRetransmit::new(1, 2),
            model_identifier: ModelIdentifier::SIG(0x1001),
        };
        let msg = ModelPublicationSetMessage {
            details: data.clone(),
        };
        let mut parameters: heapless::Vec<u8, 386> = heapless::Vec::new();
        msg.emit_parameters(&mut parameters).unwrap();

        let parsed: ModelPublicationSetMessage =
            ModelPublicationSetMessage::parse_virtual_address(&parameters[..]).unwrap();
        assert_eq!(parsed.details, data);
    }

    #[test]
    fn test_publish_period() {
        let period1 = PublishPeriod::new(60, Resolution::Seconds10);
        assert_eq!(0xBC, period1.period);

        // Publish Period: 0x29 = 4.1 seconds (41 * 100 ms).
        let period2 = PublishPeriod::from(0x29);
        assert_eq!(41, period2.steps());
        assert_eq!(
            Resolution::Milliseconds100 as u8,
            period2.resolution() as u8
        );

        // Publish Period: 0x51 = 17 seconds.
        let period3 = PublishPeriod::from(0x51);
        assert_eq!(17, period3.steps());
        assert_eq!(Resolution::Seconds1 as u8, period3.resolution() as u8);

        // Publish Period: 0x5E = 30 seconds.
        let period4 = PublishPeriod::from(0x5E);
        assert_eq!(30, period4.steps());
        assert_eq!(Resolution::Seconds1 as u8, period4.resolution() as u8);
    }

    #[test]
    fn test_retransmit() {
        let rxt = PublishRetransmit::from(0xa0);
        assert_eq!(rxt.count(), 5);
        assert_eq!(rxt.interval_steps(), 0);

        assert_eq!(u8::from(rxt), 0b10100000);
    }
}
