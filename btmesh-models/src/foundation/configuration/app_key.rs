use crate::foundation::configuration::{
    AppKeyIndex, ConfigurationMessage, KeyIndex, NetKeyAppKeyIndexesPair, NetKeyIndex,
};
use crate::{Message, Status};
use btmesh_common::crypto::application::ApplicationKey;
use btmesh_common::opcode::Opcode;
use btmesh_common::{opcode, InsufficientBuffer, ParseError};
use core::convert::TryInto;
use heapless::Vec;

opcode!( CONFIG_APPKEY_ADD 0x00 );
opcode!( CONFIG_APPKEY_DELETE 0x80, 0x00 );
opcode!( CONFIG_APPKEY_GET 0x80, 0x01 );
opcode!( CONFIG_APPKEY_LIST 0x80, 0x02 );
opcode!( CONFIG_APPKEY_STATUS 0x80, 0x03 );
opcode!( CONFIG_APPKEY_UPDATE 0x01 );

/// Application key message.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub enum AppKeyMessage {
    /// AppKey Add message.
    Add(AppKeyAddMessage),
    /// AppKey Delete message.
    Delete(AppKeyDeleteMessage),
    /// AppKey Get message.
    Get(AppKeyGetMessage),
    /// AppKey List message.
    List(AppKeyListMessage),
    /// AppKey Status message.
    Status(AppKeyStatusMessage),
    /// AppKey Update message.
    Update(AppKeyUpdateMessage),
}

impl From<AppKeyMessage> for ConfigurationMessage {
    fn from(inner: AppKeyMessage) -> Self {
        Self::AppKey(inner)
    }
}

impl AppKeyMessage {
    /// Parses byte array into AppKey Add message.
    pub fn parse_add(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() == 19 {
            let indexes = NetKeyAppKeyIndexesPair::parse(&parameters[0..=2])?;
            let app_key = ApplicationKey::new(
                parameters[3..]
                    .try_into()
                    .map_err(|_| ParseError::InvalidLength)?,
            )?;
            Ok(Self::Add(AppKeyAddMessage { indexes, app_key }))
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    /// Parses byte array into AppKey Delete message.
    pub fn parse_delete(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() == 3 {
            let indexes = NetKeyAppKeyIndexesPair::parse(&parameters[0..=2])?;
            Ok(Self::Delete(AppKeyDeleteMessage { indexes }))
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    /// Parses byte array into AppKey Get message.
    pub fn parse_get(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() == 2 {
            let net_key_index = NetKeyIndex(KeyIndex::parse_one(parameters)?);
            Ok(Self::Get(AppKeyGetMessage { net_key_index }))
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    /// Parses byte array into AppKey Status message.
    pub fn parse_status(parameters: &[u8]) -> Result<Self, ParseError> {
        Ok(Self::Status(AppKeyStatusMessage::parse(parameters)?))
    }
}

impl Message for AppKeyMessage {
    fn opcode(&self) -> Opcode {
        match self {
            Self::Add(_) => CONFIG_APPKEY_ADD,
            Self::Delete(_) => CONFIG_APPKEY_DELETE,
            Self::Get(_) => CONFIG_APPKEY_GET,
            Self::List(_) => CONFIG_APPKEY_LIST,
            Self::Status(_) => CONFIG_APPKEY_STATUS,
            Self::Update(_) => CONFIG_APPKEY_STATUS,
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        match self {
            AppKeyMessage::Add(inner) => inner.emit_parameters(xmit),
            AppKeyMessage::Delete(inner) => inner.emit_parameters(xmit),
            AppKeyMessage::Get(inner) => inner.emit_parameters(xmit),
            AppKeyMessage::List(inner) => inner.emit_parameters(xmit),
            AppKeyMessage::Status(inner) => inner.emit_parameters(xmit),
            AppKeyMessage::Update(inner) => inner.emit_parameters(xmit),
        }
    }
}

/// AppKey Add is an acknowledged message used to add an AppKey to the AppKey List
/// on a node and bind it to the NetKey identified by NetKeyIndex.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct AppKeyAddMessage {
    /// Index of the NetKey and index of the AppKey.
    pub indexes: NetKeyAppKeyIndexesPair,
    /// AppKey value.
    pub app_key: ApplicationKey,
}

impl AppKeyAddMessage {
    fn emit_parameters<const N: usize>(
        &self,
        _xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        todo!();
    }

    /// Returns network key index.
    pub fn net_key_index(&self) -> NetKeyIndex {
        self.indexes.0
    }

    /// Returns app key index.
    pub fn app_key_index(&self) -> AppKeyIndex {
        self.indexes.1
    }

    /// Returns app key.
    pub fn app_key(&self) -> ApplicationKey {
        self.app_key
    }
}

/// AppKey Delete is an acknowledged message used to delete an AppKey from the AppKey List on a node.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct AppKeyDeleteMessage {
    /// Index of the NetKey and index of the AppKey.
    pub indexes: NetKeyAppKeyIndexesPair,
}

impl AppKeyDeleteMessage {
    fn emit_parameters<const N: usize>(
        &self,
        _xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        todo!();
    }

    /// Returns network key index.
    pub fn net_key_index(&self) -> NetKeyIndex {
        self.indexes.0
    }

    /// Returns application key index.
    pub fn app_key_index(&self) -> AppKeyIndex {
        self.indexes.1
    }
}

/// AppKey Get is an acknowledged message used to report all AppKeys bound to the NetKey.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct AppKeyGetMessage {
    /// Index of the NetKey.
    pub(crate) net_key_index: NetKeyIndex,
}

impl AppKeyGetMessage {
    fn emit_parameters<const N: usize>(
        &self,
        _xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        todo!()
    }
}

/// NetKey List is an unacknowledged message reporting all NetKeys known to the node.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct AppKeyListMessage {
    pub(crate) status: Status,
    pub(crate) net_key_index: NetKeyIndex,
    pub(crate) app_key_indexes: Vec<AppKeyIndex, 10>,
}

impl AppKeyListMessage {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        xmit.push(self.status as u8)
            .map_err(|_| InsufficientBuffer)?;
        self.net_key_index.emit(xmit)?;

        /*
        for (i, app_key_index) in self.app_key_indexes.iter().enumerate() {
            if (i + 1) % 2 == 0 {
                app_key_index.0.emit_second(xmit)?;
            } else {
                app_key_index.0.emit_first(xmit)?;
            }
        }
         */
        for chunk in self.app_key_indexes.chunks(2) {
            if chunk.len() == 2 {
                KeyIndex::emit_two((&chunk[0].0, &chunk[1].0), xmit)?;
            } else {
                KeyIndex::emit_one(&chunk[0].0, xmit)?;
            }
        }

        Ok(())
    }
}

/// AppKey Status is an unacknowledged message used to report a status for the requesting message,
/// based on the NetKey Index identifying the NetKey on the NetKey List and on the AppKey Index identifying
/// the AppKey on the AppKey List.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct AppKeyStatusMessage {
    /// Status Code for the requesting message.
    pub status: Status,
    /// Index of the NetKey and index of the AppKey.
    pub indexes: NetKeyAppKeyIndexesPair,
}

impl From<AppKeyStatusMessage> for AppKeyMessage {
    fn from(inner: AppKeyStatusMessage) -> Self {
        Self::Status(inner)
    }
}

impl AppKeyStatusMessage {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        xmit.push(self.status as u8)
            .map_err(|_| InsufficientBuffer)?;
        self.indexes.emit(xmit)?;
        Ok(())
    }

    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        let status: Status = parameters[0].try_into()?;
        let indexes = NetKeyAppKeyIndexesPair::parse(&parameters[1..=3])?;
        Ok(Self { status, indexes })
    }
}

/// AppKey Update is an acknowledged message used to update an AppKey value on the AppKey List on a node.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct AppKeyUpdateMessage {
    pub(crate) net_key_index: NetKeyIndex,
    pub(crate) app_key_index: AppKeyIndex,
    pub(crate) app_key: [u8; 16],
}

impl AppKeyUpdateMessage {
    fn emit_parameters<const N: usize>(
        &self,
        _xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        todo!()
    }
}
