use crate::foundation::configuration::app_key::{
    AppKeyMessage, CONFIG_APPKEY_ADD, CONFIG_APPKEY_DELETE, CONFIG_APPKEY_GET, CONFIG_APPKEY_STATUS,
};
use crate::foundation::configuration::beacon::{
    BeaconMessage, CONFIG_BEACON_GET, CONFIG_BEACON_SET,
};
use crate::foundation::configuration::composition_data::{
    CompositionDataMessage, CONFIG_COMPOSITION_DATA_GET,
};
use crate::foundation::configuration::default_ttl::{
    DefaultTTLMessage, CONFIG_DEFAULT_TTL_GET, CONFIG_DEFAULT_TTL_SET,
};
use crate::foundation::configuration::model_app::{
    ModelAppMessage, CONFIG_MODEL_APP_BIND, CONFIG_MODEL_APP_STATUS, CONFIG_MODEL_APP_UNBIND,
};
use crate::foundation::configuration::model_publication::{
    ModelPublicationMessage, CONFIG_MODEL_PUBLICATION_GET, CONFIG_MODEL_PUBLICATION_SET,
    CONFIG_MODEL_PUBLICATION_STATUS, CONFIG_MODEL_PUBLICATION_VIRTUAL_ADDRESS_SET,
};

use crate::foundation::configuration::model_subscription::{
    ModelSubscriptionMessage, CONFIG_MODEL_SUBSCRIPTION_ADD, CONFIG_MODEL_SUBSCRIPTION_DELETE,
    CONFIG_MODEL_SUBSCRIPTION_DELETE_ALL, CONFIG_MODEL_SUBSCRIPTION_OVERWRITE,
    CONFIG_MODEL_SUBSCRIPTION_STATUS, CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_ADD,
    CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_DELETE,
    CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_OVERWRITE, CONFIG_SIG_MODEL_SUBSCRIPTION_GET,
    CONFIG_VENDOR_MODEL_SUBSCRIPTION_GET,
};

use crate::foundation::configuration::node_reset::{
    NodeResetMessage, CONFIG_NODE_RESET, CONFIG_NODE_RESET_STATUS,
};

use crate::foundation::configuration::relay::{
    RelayMessage, CONFIG_RELAY_GET, CONFIG_RELAY_SET, CONFIG_RELAY_STATUS,
};

use crate::{Message, Model};

use btmesh_common::opcode::Opcode;
use btmesh_common::{InsufficientBuffer, ModelIdentifier, ParseError};
use heapless::Vec;

pub mod app_key;
pub mod beacon;
pub mod composition_data;
pub mod default_ttl;
pub mod model_app;
pub mod model_publication;
pub mod model_subscription;
pub mod network_transmit;
pub mod node_reset;
pub mod relay;

pub const CONFIGURATION_SERVER: ModelIdentifier = ModelIdentifier::SIG(0x0000);
pub const CONFIGURATION_CLIENT: ModelIdentifier = ModelIdentifier::SIG(0x0001);

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub enum ConfigurationMessage {
    Beacon(BeaconMessage),
    DefaultTTL(DefaultTTLMessage),
    NodeReset(NodeResetMessage),
    CompositionData(CompositionDataMessage),
    AppKey(AppKeyMessage),
    ModelApp(ModelAppMessage),
    ModelPublication(ModelPublicationMessage),
    ModelSubscription(ModelSubscriptionMessage),
    Relay(RelayMessage),
}

impl Message for ConfigurationMessage {
    fn opcode(&self) -> Opcode {
        match self {
            ConfigurationMessage::Beacon(inner) => inner.opcode(),
            ConfigurationMessage::DefaultTTL(inner) => inner.opcode(),
            ConfigurationMessage::NodeReset(inner) => inner.opcode(),
            ConfigurationMessage::CompositionData(inner) => inner.opcode(),
            ConfigurationMessage::AppKey(inner) => inner.opcode(),
            ConfigurationMessage::ModelApp(inner) => inner.opcode(),
            ConfigurationMessage::ModelPublication(inner) => inner.opcode(),
            ConfigurationMessage::ModelSubscription(inner) => inner.opcode(),
            ConfigurationMessage::Relay(inner) => inner.opcode(),
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        match self {
            ConfigurationMessage::Beacon(inner) => inner.emit_parameters(xmit),
            ConfigurationMessage::DefaultTTL(inner) => inner.emit_parameters(xmit),
            ConfigurationMessage::NodeReset(inner) => inner.emit_parameters(xmit),
            ConfigurationMessage::CompositionData(inner) => inner.emit_parameters(xmit),
            ConfigurationMessage::AppKey(inner) => inner.emit_parameters(xmit),
            ConfigurationMessage::ModelApp(inner) => inner.emit_parameters(xmit),
            ConfigurationMessage::ModelPublication(inner) => inner.emit_parameters(xmit),
            ConfigurationMessage::ModelSubscription(inner) => inner.emit_parameters(xmit),
            ConfigurationMessage::Relay(inner) => inner.emit_parameters(xmit),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ConfigurationServer;

impl Model for ConfigurationServer {
    const IDENTIFIER: ModelIdentifier = CONFIGURATION_SERVER;
    const SUPPORTS_SUBSCRIPTION: bool = false;
    const SUPPORTS_PUBLICATION: bool = false;
    type Message = ConfigurationMessage;

    fn parse(opcode: &Opcode, parameters: &[u8]) -> Result<Option<Self::Message>, ParseError> {
        match *opcode {
            CONFIG_BEACON_GET => Ok(Some(ConfigurationMessage::Beacon(
                BeaconMessage::parse_get(parameters)?,
            ))),
            CONFIG_BEACON_SET => Ok(Some(ConfigurationMessage::Beacon(
                BeaconMessage::parse_set(parameters)?,
            ))),
            CONFIG_DEFAULT_TTL_GET => Ok(Some(ConfigurationMessage::DefaultTTL(
                DefaultTTLMessage::parse_get(parameters)?,
            ))),
            CONFIG_DEFAULT_TTL_SET => Ok(Some(ConfigurationMessage::DefaultTTL(
                DefaultTTLMessage::parse_set(parameters)?,
            ))),
            CONFIG_NODE_RESET => Ok(Some(ConfigurationMessage::NodeReset(
                NodeResetMessage::parse_reset(parameters)?,
            ))),
            CONFIG_COMPOSITION_DATA_GET => Ok(Some(ConfigurationMessage::CompositionData(
                CompositionDataMessage::parse_get(parameters)?,
            ))),
            // App Key
            CONFIG_APPKEY_ADD => Ok(Some(ConfigurationMessage::AppKey(
                AppKeyMessage::parse_add(parameters)?,
            ))),
            CONFIG_APPKEY_DELETE => Ok(Some(ConfigurationMessage::AppKey(
                AppKeyMessage::parse_delete(parameters)?,
            ))),
            CONFIG_APPKEY_GET => Ok(Some(ConfigurationMessage::AppKey(
                AppKeyMessage::parse_get(parameters)?,
            ))),
            // Model App
            CONFIG_MODEL_APP_BIND => Ok(Some(ConfigurationMessage::ModelApp(
                ModelAppMessage::parse_bind(parameters)?,
            ))),
            CONFIG_MODEL_APP_UNBIND => Ok(Some(ConfigurationMessage::ModelApp(
                ModelAppMessage::parse_unbind(parameters)?,
            ))),
            // Model Publication
            CONFIG_MODEL_PUBLICATION_SET => Ok(Some(ConfigurationMessage::ModelPublication(
                ModelPublicationMessage::parse_set(parameters)?,
            ))),
            CONFIG_MODEL_PUBLICATION_GET => Ok(Some(ConfigurationMessage::ModelPublication(
                ModelPublicationMessage::parse_get(parameters)?,
            ))),
            CONFIG_MODEL_PUBLICATION_VIRTUAL_ADDRESS_SET => {
                Ok(Some(ConfigurationMessage::ModelPublication(
                    ModelPublicationMessage::parse_virtual_address_set(parameters)?,
                )))
            }
            // Model Subscription
            CONFIG_MODEL_SUBSCRIPTION_ADD => Ok(Some(ConfigurationMessage::ModelSubscription(
                ModelSubscriptionMessage::parse_add(parameters)?,
            ))),
            CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_ADD => {
                Ok(Some(ConfigurationMessage::ModelSubscription(
                    ModelSubscriptionMessage::parse_virtual_address_add(parameters)?,
                )))
            }
            CONFIG_MODEL_SUBSCRIPTION_DELETE => Ok(Some(ConfigurationMessage::ModelSubscription(
                ModelSubscriptionMessage::parse_delete(parameters)?,
            ))),
            CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_DELETE => {
                Ok(Some(ConfigurationMessage::ModelSubscription(
                    ModelSubscriptionMessage::parse_virtual_address_delete(parameters)?,
                )))
            }
            CONFIG_MODEL_SUBSCRIPTION_OVERWRITE => {
                Ok(Some(ConfigurationMessage::ModelSubscription(
                    ModelSubscriptionMessage::parse_overwrite(parameters)?,
                )))
            }
            CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_OVERWRITE => {
                Ok(Some(ConfigurationMessage::ModelSubscription(
                    ModelSubscriptionMessage::parse_virtual_address_overwrite(parameters)?,
                )))
            }
            CONFIG_MODEL_SUBSCRIPTION_DELETE_ALL => {
                Ok(Some(ConfigurationMessage::ModelSubscription(
                    ModelSubscriptionMessage::parse_delete_all(parameters)?,
                )))
            }
            CONFIG_VENDOR_MODEL_SUBSCRIPTION_GET => {
                Ok(Some(ConfigurationMessage::ModelSubscription(
                    ModelSubscriptionMessage::parse_vendor_get(parameters)?,
                )))
            }
            CONFIG_SIG_MODEL_SUBSCRIPTION_GET => Ok(Some(ConfigurationMessage::ModelSubscription(
                ModelSubscriptionMessage::parse_sig_get(parameters)?,
            ))),

            // Relay
            CONFIG_RELAY_GET => Ok(Some(ConfigurationMessage::Relay(RelayMessage::parse_get(
                parameters,
            )?))),
            CONFIG_RELAY_SET => Ok(Some(ConfigurationMessage::Relay(RelayMessage::parse_set(
                parameters,
            )?))),
            _ => Ok(None),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ConfigurationClient;

impl Model for ConfigurationClient {
    const IDENTIFIER: ModelIdentifier = CONFIGURATION_CLIENT;
    const SUPPORTS_SUBSCRIPTION: bool = false;
    const SUPPORTS_PUBLICATION: bool = false;
    type Message = ConfigurationMessage;

    fn parse(opcode: &Opcode, parameters: &[u8]) -> Result<Option<Self::Message>, ParseError> {
        match *opcode {
            CONFIG_NODE_RESET_STATUS => Ok(Some(ConfigurationMessage::NodeReset(
                NodeResetMessage::parse_status(parameters)?,
            ))),
            CONFIG_APPKEY_STATUS => Ok(Some(ConfigurationMessage::AppKey(
                AppKeyMessage::parse_status(parameters)?,
            ))),
            CONFIG_MODEL_APP_STATUS => Ok(Some(ConfigurationMessage::ModelApp(
                ModelAppMessage::parse_status(parameters)?,
            ))),
            CONFIG_MODEL_PUBLICATION_STATUS => Ok(Some(ConfigurationMessage::ModelPublication(
                ModelPublicationMessage::parse_status(parameters)?,
            ))),
            CONFIG_MODEL_SUBSCRIPTION_STATUS => Ok(Some(ConfigurationMessage::ModelSubscription(
                ModelSubscriptionMessage::parse_status(parameters)?,
            ))),
            CONFIG_RELAY_STATUS => Ok(Some(ConfigurationMessage::Relay(
                RelayMessage::parse_status(parameters)?,
            ))),
            _ => Ok(None),
        }
    }
}

// ------------------------------------------------------------------------
// ------------------------------------------------------------------------

#[derive(PartialEq, Eq, PartialOrd, Copy, Clone, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct KeyIndex(u16);

#[cfg(feature = "defmt")]
impl defmt::Format for KeyIndex {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{}", self.0);
    }
}

impl KeyIndex {
    pub fn new(index: u16) -> Self {
        Self(index)
    }

    fn parse_one(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 2 {
            let byte1 = parameters[0];
            let byte2 = parameters[1] & 0b11110000 >> 4;
            let val = u16::from_be_bytes([byte2, byte1]);
            Ok(Self(val))
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    fn emit_one<const N: usize>(
        index: &KeyIndex,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        let bytes = index.0.to_be_bytes();
        let byte1 = bytes[1];
        let byte2 = bytes[0] << 4;
        xmit.push(byte1).map_err(|_| InsufficientBuffer)?;
        xmit.push(byte2).map_err(|_| InsufficientBuffer)?;
        Ok(())
    }

    fn parse_two(parameters: &[u8]) -> Result<(Self, Self), ParseError> {
        if parameters.len() >= 3 {
            let byte1 = parameters[0];
            let byte2 = (parameters[1] & 0b11110000) >> 4;

            let index1 = u16::from_be_bytes([byte1, byte2]);

            let byte1 = parameters[1] & 0b00001111;
            let byte2 = parameters[2];

            let index2 = u16::from_be_bytes([byte1, byte2]);
            Ok((Self(index2), Self(index1)))
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    fn emit_two<const N: usize>(
        indexes: (&KeyIndex, &KeyIndex),
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        let bytes = indexes.1 .0.to_be_bytes();
        let byte1 = bytes[0];
        xmit.push(byte1).map_err(|_| InsufficientBuffer)?;

        let byte2 = bytes[1] << 4;
        let bytes = indexes.0 .0.to_be_bytes();
        let byte1 = byte2 | bytes[0];

        xmit.push(byte1).map_err(|_| InsufficientBuffer)?;

        let byte2 = bytes[1];
        xmit.push(byte2).map_err(|_| InsufficientBuffer)?;

        Ok(())
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Eq, PartialEq, PartialOrd, Copy, Clone, Debug, Hash)]
pub struct NetKeyIndex(KeyIndex);

impl NetKeyIndex {
    pub fn new(index: u16) -> Self {
        Self(KeyIndex(index))
    }

    fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        KeyIndex::emit_one(&self.0, xmit)
    }
}

impl From<NetKeyIndex> for usize {
    fn from(index: NetKeyIndex) -> Self {
        index.0 .0 as usize
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for NetKeyIndex {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{}", self.0)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Eq, PartialOrd, Copy, Clone, Debug, Hash)]
pub struct AppKeyIndex(KeyIndex);

impl AppKeyIndex {
    pub fn new(index: u16) -> Self {
        Self(KeyIndex::new(index))
    }

    fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        KeyIndex::emit_one(&self.0, xmit)
    }
}

impl From<AppKeyIndex> for usize {
    fn from(index: AppKeyIndex) -> Self {
        index.0 .0 as usize
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for AppKeyIndex {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{}", self.0)
    }
}

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NetKeyAppKeyIndexesPair(NetKeyIndex, AppKeyIndex);

impl NetKeyAppKeyIndexesPair {
    fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        KeyIndex::emit_two((&self.0 .0, &self.1 .0), xmit).map_err(|_| InsufficientBuffer)?;
        Ok(())
    }

    pub fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() == 3 {
            let (net_key, app_key) = KeyIndex::parse_two(parameters)?;
            Ok(Self(NetKeyIndex(net_key), AppKeyIndex(app_key)))
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    pub fn net_key(&self) -> NetKeyIndex {
        self.0
    }

    pub fn app_key(&self) -> AppKeyIndex {
        self.1
    }
}

// ------------------------------------------------------------------------
// ------------------------------------------------------------------------
