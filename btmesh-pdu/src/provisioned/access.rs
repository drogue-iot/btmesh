use crate::provisioned::{Message, System};
use btmesh_common::opcode::Opcode;
use btmesh_common::{InsufficientBuffer, ParseError};
use heapless::Vec;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[allow(dead_code)]
pub struct AccessMessage<S: System> {
    opcode: Opcode,
    parameters: Vec<u8, 379>,
    meta: S::AccessMetadata,
}

impl<S: System> From<AccessMessage<S>> for Message<S> {
    fn from(inner: AccessMessage<S>) -> Self {
        Self::Access(inner)
    }
}

#[allow(unused)]
impl<S: System> AccessMessage<S> {
    pub fn new<M: Into<S::AccessMetadata>>(
        opcode: Opcode,
        parameters: Vec<u8, 379>,
        meta: M,
    ) -> Self {
        Self {
            opcode,
            parameters,
            meta: meta.into(),
        }
    }

    pub fn opcode(&self) -> Opcode {
        self.opcode
    }

    pub fn parameters(&self) -> &[u8] {
        &self.parameters
    }

    pub fn meta(&self) -> &S::AccessMetadata {
        &self.meta
    }

    pub fn meta_mut(&mut self) -> &mut S::AccessMetadata {
        &mut self.meta
    }

    pub fn parse(data: &[u8], meta: S::AccessMetadata) -> Result<Self, ParseError> {
        let (opcode, parameters) = Opcode::split(data).ok_or(ParseError::InvalidPDUFormat)?;
        Ok(Self {
            opcode,
            parameters: Vec::from_slice(parameters)?,
            meta,
        })
    }

    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        self.opcode.emit(xmit)?;
        xmit.extend_from_slice(&self.parameters)?;
        Ok(())
    }
}

/*
opcode!( CONFIG_BEACON_GET 0x80, 0x09 );
opcode!( CONFIG_BEACON_SET 0x80, 0x0A );
opcode!( CONFIG_BEACON_STATUS 0x80, 0x0B );
opcode!( CONFIG_FRIEND_GET 0x80, 0x0F );
opcode!( CONFIG_FRIEND_SET 0x80, 0x10 );
opcode!( CONFIG_FRIEND_STATUS 0x80, 0x11 );
opcode!( CONFIG_GATT_PROXY_GET 0x80, 0x12 );
opcode!( CONFIG_GATT_PROXY_SET 0x80, 0x13 );
opcode!( CONFIG_GATT_PROXY_STATUS 0x80, 0x14 );
opcode!( CONFIG_HEARTBEAT_PUBLICATION_GET 0x80, 0x38 );
opcode!( CONFIG_HEARTBEAT_PUBLICATION_SET 0x80, 0x39 );
opcode!( CONFIG_HEARTBEAT_PUBLICATION_STATUS 0x06 );
opcode!( CONFIG_HEARTBEAT_SUBSCRIPTION_GET 0x80, 0x3A );
opcode!( CONFIG_HEARTBEAT_SUBSCRIPTION_SET 0x80, 0x3B );
opcode!( CONFIG_HEARTBEAT_SUBSCRIPTION_STATUS 0x80, 0x3C );
opcode!( CONFIG_KEY_REFRESH_PHASE_GET 0x80, 0x15 );
opcode!( CONFIG_KEY_REFRESH_PHASE_SET 0x80, 0x16 );
opcode!( CONFIG_KEY_REFRESH_PHASE_STATUS 0x80, 0x17 );
opcode!( CONFIG_LOW_POWER_NODE_POLLTIMEOUT_GET 0x80, 0x2D );
opcode!( CONFIG_LOW_POWER_NODE_POLLTIMEOUT_STATUS 0x80, 0x2E );
opcode!( CONFIG_NETKEY_ADD 0x80, 0x40);
opcode!( CONFIG_NETKEY_DELETE 0x80, 0x41);
opcode!( CONFIG_NETKEY_GET 0x80, 0x42);
opcode!( CONFIG_NETKEY_LIST 0x80, 0x43);
opcode!( CONFIG_NETKEY_STATUS 0x80, 0x44);
opcode!( CONFIG_NETKEY_UPDATE 0x80, 0x45);
opcode!( CONFIG_NETWORK_TRANSMIT_GET 0x80, 0x23);
opcode!( CONFIG_NETWORK_TRANSMIT_SET 0x80, 0x24);
opcode!( CONFIG_NETWORK_TRANSMIT_STATUS 0x80, 0x25);
opcode!( CONFIG_NODE_IDENTITY_GET 0x80, 0x46);
opcode!( CONFIG_NODE_IDENTITY_SET 0x80, 0x47);
opcode!( CONFIG_NODE_IDENTITY_STATUS 0x80, 0x48);
opcode!( CONFIG_RELAY_GET 0x80, 0x26);
opcode!( CONFIG_RELAY_SET 0x80, 0x27);
opcode!( CONFIG_RELAY_STATUS 0x80, 0x28);
opcode!( CONFIG_SIG_MODEL_APP_GET 0x80, 0x4B);
opcode!( CONFIG_SIG_MODEL_APP_LIST 0x80, 0x4C);
opcode!( CONFIG_SIG_MODEL_SUBSCRIPTION_GET 0x80, 0x29);
opcode!( CONFIG_SIG_MODEL_SUBSCRIPTION_LIST 0x80, 0x2A );
opcode!( CONFIG_VENDOR_MODEL_APP_GET 0x80, 0x4D );
opcode!( CONFIG_VENDOR_MODEL_APP_LIST 0x80, 0x4E );
opcode!( CONFIG_VENDOR_MODEL_SUBSCRIPTION_GET 0x80, 0x2B );
opcode!( CONFIG_VENDOR_MODEL_SUBSCRIPTION_LIST 0x80, 0x2C );

opcode!( HEALTH_ATTENTION_GET 0x80, 0x04 );
opcode!( HEALTH_ATTENTION_SET 0x80, 0x05 );
opcode!( HEALTH_ATTENTION_SET_UNACKNOWLEDGED 0x80, 0x06 );
opcode!( HEALTH_ATTENTION_STATUS 0x80, 0x07 );
opcode!( HEALTH_CURRENT_STATUS 0x04 );
opcode!( HEALTH_FAULT_CLEAR 0x80, 0x2F );
opcode!( HEALTH_FAULT_CLEAR_UNACKNOWLEDGED 0x80, 0x30 );
opcode!( HEALTH_FAULT_GET 0x80, 0x31 );
opcode!( HEALTH_FAULT_STATUS 0x05 );
opcode!( HEALTH_FAULT_TEST 0x80, 0x32 );
opcode!( HEALTH_FAULT_TEST_UNACKNOWLEDGED 0x80, 0x33 );
opcode!( HEALTH_PERIOD_GET 0x80, 0x34 );
opcode!( HEALTH_PERIOD_SET 0x80, 0x35 );
opcode!( HEALTH_PERIOD_SET_UNACKNOWLEDGED 0x80, 0x36 );
opcode!( HEALTH_PERIOD_STATUS 0x80, 0x37 );
 */
