use crate::foundation::configuration::{AppKeyIndex, ConfigurationMessage, KeyIndex};
use crate::{Message, Status};
use btmesh_common::address::UnicastAddress;
use btmesh_common::opcode::Opcode;
use btmesh_common::{opcode, InsufficientBuffer, ModelIdentifier, ParseError};
use heapless::Vec;

opcode!( CONFIG_MODEL_APP_BIND 0x80, 0x3D);
opcode!( CONFIG_MODEL_APP_STATUS 0x80, 0x3E);
opcode!( CONFIG_MODEL_APP_UNBIND 0x80, 0x3F);

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub enum ModelAppMessage {
    Bind(ModelAppPayload),
    Status(ModelAppStatusMessage),
    Unbind(ModelAppPayload),
}

impl From<ModelAppMessage> for ConfigurationMessage {
    fn from(inner: ModelAppMessage) -> Self {
        ConfigurationMessage::ModelApp(inner)
    }
}

impl ModelAppMessage {
    pub fn parse_bind(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Bind(ModelAppPayload::parse(parameters)?))
    }

    pub fn parse_unbind(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Unbind(ModelAppPayload::parse(parameters)?))
    }

    pub fn parse_status(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Status(ModelAppStatusMessage::parse(parameters)?))
    }
}

impl Message for ModelAppMessage {
    fn opcode(&self) -> Opcode {
        match self {
            Self::Bind(_) => CONFIG_MODEL_APP_BIND,
            Self::Status(_) => CONFIG_MODEL_APP_STATUS,
            Self::Unbind(_) => CONFIG_MODEL_APP_UNBIND,
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        match self {
            ModelAppMessage::Bind(inner) => inner.emit_parameters(xmit),
            ModelAppMessage::Status(inner) => inner.emit_parameters(xmit),
            ModelAppMessage::Unbind(inner) => inner.emit_parameters(xmit),
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ModelAppPayload {
    pub element_address: UnicastAddress,
    pub app_key_index: AppKeyIndex,
    pub model_identifier: ModelIdentifier,
}

impl ModelAppPayload {
    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 6 {
            // yes, swapped, because in *this* case it's little-endian
            let element_address = UnicastAddress::parse([parameters[1], parameters[0]])
                .map_err(|_| ParseError::InvalidValue)?;
            let app_key_index = AppKeyIndex(KeyIndex::parse_one(&parameters[2..=3])?);
            let model_identifier = ModelIdentifier::parse(&parameters[4..])?;
            Ok(Self {
                element_address,
                app_key_index,
                model_identifier,
            })
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        let addr_bytes = self.element_address.as_bytes();
        xmit.push(addr_bytes[1]).map_err(|_| InsufficientBuffer)?;
        xmit.push(addr_bytes[0]).map_err(|_| InsufficientBuffer)?;
        self.app_key_index.emit(xmit)?;
        self.model_identifier.emit(xmit)?;
        Ok(())
    }
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct ModelAppStatusMessage {
    pub status: Status,
    pub payload: ModelAppPayload,
}

impl ModelAppStatusMessage {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        xmit.push(self.status as u8)
            .map_err(|_| InsufficientBuffer)?;
        self.payload.emit_parameters(xmit)?;
        Ok(())
    }

    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        let status: Status = parameters[0].try_into()?;
        let payload: ModelAppPayload = ModelAppPayload::parse(&parameters[1..])?;
        Ok(Self { status, payload })
    }
}
