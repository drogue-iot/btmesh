use crate::provisioned::ProvisionedDriver;
use crate::DriverError;
use btmesh_common::address::{Address, LabelUuid, UnicastAddress};
use btmesh_common::crypto::application::Aid;
use btmesh_common::crypto::network::Nid;
use btmesh_common::{IvIndex, Seq, Ttl};
use btmesh_pdu::access::AccessMessage;
use btmesh_pdu::lower::{LowerPDU, SegmentedLowerPDU, UnsegmentedLowerPDU};
use btmesh_pdu::network::CleartextNetworkPDU;
use btmesh_pdu::upper::access::UpperAccessPDU;
use btmesh_pdu::upper::control::UpperControlPDU;
use btmesh_pdu::upper::UpperPDU;
use btmesh_pdu::System;
use hash32_derive::Hash32;
use heapless::Vec;

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd)]
pub enum KeyHandle {
    Device,
    Network(NetworkKeyHandle),
    Application(ApplicationKeyHandle),
}

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Hash32)]
pub struct NetworkKeyHandle(pub(crate) u8, pub(crate) Nid);

impl NetworkKeyHandle {
    pub fn nid(&self) -> Nid {
        self.1
    }
}

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Hash32)]
pub struct ApplicationKeyHandle(pub(crate) u8, pub(crate) Aid);

impl ApplicationKeyHandle {
    pub fn aid(&self) -> Aid {
        self.1
    }
}

#[derive(Copy, Clone)]
pub struct NetworkMetadata {
    iv_index: IvIndex,
    replay_protected: bool,
    should_relay: bool,
    local_element_index: Option<u8>,
    network_key_handle: NetworkKeyHandle,
}

impl NetworkMetadata {
    pub fn new(
        iv_index: IvIndex,
        local_element_index: Option<u8>,
        network_key: NetworkKeyHandle,
    ) -> Self {
        Self {
            iv_index,
            replay_protected: false,
            should_relay: false,
            local_element_index,
            network_key_handle: network_key,
        }
    }

    pub fn network_key_handle(&self) -> NetworkKeyHandle {
        self.network_key_handle
    }

    pub fn iv_index(&self) -> IvIndex {
        self.iv_index
    }

    pub fn replay_protected(&mut self, protected: bool) {
        self.replay_protected = protected;
    }

    pub fn should_relay(&mut self, relay: bool) {
        self.should_relay = relay;
    }

    pub fn local_element_index(&self) -> Option<u8> {
        self.local_element_index
    }

    pub fn from_upper_pdu(pdu: &UpperPDU<ProvisionedDriver>) -> Self {
        Self {
            iv_index: pdu.meta().iv_index(),
            replay_protected: false,
            should_relay: false,
            local_element_index: None,
            network_key_handle: pdu.meta().network_key_handle(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct LowerMetadata {
    network_key_handle: NetworkKeyHandle,
    iv_index: IvIndex,
    src: UnicastAddress,
    dst: Address,
    ttl: Ttl,
    seq: Seq,
}

impl LowerMetadata {
    pub fn new(
        network_key_handle: NetworkKeyHandle,
        iv_index: IvIndex,
        src: UnicastAddress,
        dst: Address,
        seq: Seq,
        ttl: Ttl,
    ) -> Self {
        Self {
            network_key_handle,
            iv_index,
            src,
            dst,
            seq,
            ttl,
        }
    }

    pub fn from_network_pdu(pdu: &CleartextNetworkPDU<ProvisionedDriver>) -> Self {
        Self {
            network_key_handle: pdu.meta().network_key_handle(),
            iv_index: pdu.meta().iv_index(),
            src: pdu.src(),
            dst: pdu.dst(),
            seq: pdu.seq(),
            ttl: pdu.ttl(),
        }
    }

    /*
    pub fn from_upper_pdu(pdu: &UpperPDU<ProvisionedDriver>) -> Self {
        Self {
            network_key_handle: pdu.meta().network_key_handle(),
            iv_index: pdu.meta().iv_index(),
            src: pdu.meta().src(),
            dst: pdu.meta().dst(),
            ttl: pdu.meta().ttl,
            seq: pdu.meta().seq(),
        }
    }
     */

    pub fn network_key_handle(&self) -> NetworkKeyHandle {
        self.network_key_handle
    }

    pub fn src(&self) -> UnicastAddress {
        self.src
    }

    pub fn dst(&self) -> Address {
        self.dst
    }

    pub fn seq(&self) -> Seq {
        self.seq
    }

    pub fn ttl(&self) -> Ttl {
        self.ttl
    }

    pub fn iv_index(&self) -> IvIndex {
        self.iv_index
    }
}

#[derive(Clone)]
pub struct UpperMetadata {
    network_key_handle: NetworkKeyHandle,
    iv_index: IvIndex,
    akf_aid: Option<Aid>,
    seq: Seq,
    src: UnicastAddress,
    dst: Address,
    ttl: Ttl,
    label_uuids: Vec<LabelUuid, 3>,
}

impl UpperMetadata {
    pub fn from_segmented_lower_pdu(pdu: &SegmentedLowerPDU<ProvisionedDriver>) -> Self {
        Self {
            network_key_handle: pdu.meta().network_key_handle(),
            iv_index: pdu.meta().iv_index(),
            akf_aid: if let SegmentedLowerPDU::Access(inner) = pdu {
                inner.aid()
            } else {
                None
            },
            seq: pdu.meta().seq(),
            src: pdu.meta().src(),
            dst: pdu.meta().dst(),
            ttl: pdu.meta().ttl(),
            label_uuids: Default::default(),
        }
    }

    pub fn from_unsegmented_lower_pdu(pdu: &UnsegmentedLowerPDU<ProvisionedDriver>) -> Self {
        Self {
            network_key_handle: pdu.meta().network_key_handle(),
            iv_index: pdu.meta().iv_index(),
            akf_aid: if let UnsegmentedLowerPDU::Access(inner) = pdu {
                inner.aid()
            } else {
                None
            },
            seq: pdu.meta().seq(),
            src: pdu.meta().src(),
            dst: pdu.meta().dst(),
            ttl: pdu.meta().ttl(),
            label_uuids: Default::default(),
        }
    }

    pub fn from_lower_pdu(pdu: &LowerPDU<ProvisionedDriver>) -> Self {
        match pdu {
            LowerPDU::Unsegmented(inner) => Self::from_unsegmented_lower_pdu(inner),
            LowerPDU::Segmented(inner) => Self::from_segmented_lower_pdu(inner),
        }
    }

    pub fn from_access_message(message: &AccessMessage<ProvisionedDriver>, seq: Seq) -> Self {
        Self {
            network_key_handle: message.meta().network_key_handle(),
            iv_index: message.meta().iv_index,
            akf_aid: match message.meta().key_handle() {
                KeyHandle::Device | KeyHandle::Network(_) => None,
                KeyHandle::Application(key_handle) => Some(key_handle.aid()),
            },
            seq,
            src: message.meta().src(),
            dst: message.meta().dst(),
            ttl: message.meta().ttl(),
            label_uuids: Default::default(),
        }
    }

    pub fn network_key_handle(&self) -> NetworkKeyHandle {
        self.network_key_handle
    }

    pub fn iv_index(&self) -> IvIndex {
        self.iv_index
    }

    pub fn aid(&self) -> Option<Aid> {
        self.akf_aid
    }

    pub fn seq(&self) -> Seq {
        self.seq
    }

    pub fn src(&self) -> UnicastAddress {
        self.src
    }

    pub fn dst(&self) -> Address {
        self.dst
    }

    pub fn ttl(&self) -> Ttl {
        self.ttl
    }

    pub fn label_uuids(&self) -> &[LabelUuid] {
        &*self.label_uuids
    }

    pub fn add_label_uuid(&mut self, label_uuid: LabelUuid) -> Result<(), DriverError> {
        self.label_uuids
            .push(label_uuid)
            .map_err(|_| DriverError::InsufficientSpace)?;
        Ok(())
    }
}

#[derive(Copy, Clone)]
pub struct AccessMetadata {
    network_key_handle: NetworkKeyHandle,
    iv_index: IvIndex,
    key_handle: KeyHandle,
    src: UnicastAddress,
    dst: Address,
    ttl: Ttl,
    label_uuid: Option<LabelUuid>,
}

impl AccessMetadata {
    pub fn from_upper_access_pdu(
        key_handle: KeyHandle,
        label_uuid: Option<LabelUuid>,
        pdu: &UpperAccessPDU<ProvisionedDriver>,
    ) -> Self {
        Self {
            network_key_handle: pdu.meta().network_key_handle(),
            iv_index: pdu.meta().iv_index,
            key_handle,
            src: pdu.meta().src(),
            dst: pdu.meta().dst(),
            ttl: pdu.meta().ttl(),
            label_uuid,
        }
    }

    pub fn network_key_handle(&self) -> NetworkKeyHandle {
        self.network_key_handle
    }

    pub fn iv_index(&self) -> IvIndex {
        self.iv_index
    }

    pub fn key_handle(&self) -> KeyHandle {
        self.key_handle
    }

    pub fn src(&self) -> UnicastAddress {
        self.src
    }

    pub fn dst(&self) -> Address {
        self.dst
    }

    pub fn ttl(&self) -> Ttl {
        self.ttl
    }

    pub fn label_uuid(&self) -> Option<LabelUuid> {
        self.label_uuid
    }
}

#[derive(Copy, Clone)]
pub struct ControlMetadata {}

impl ControlMetadata {
    pub fn from_upper_control_pdu(pdu: &UpperControlPDU<ProvisionedDriver>) -> Self {
        Self {}
    }
}

impl System for ProvisionedDriver {
    type NetworkKeyHandle = NetworkKeyHandle;
    type ApplicationKeyHandle = ApplicationKeyHandle;
    type NetworkMetadata = NetworkMetadata;
    type LowerMetadata = LowerMetadata;
    type UpperMetadata = UpperMetadata;
    type AccessMetadata = AccessMetadata;
    type ControlMetadata = ControlMetadata;
}
