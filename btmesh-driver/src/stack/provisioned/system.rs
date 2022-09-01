use crate::stack::provisioned::ProvisionedStack;
use crate::DriverError;
use btmesh_common::address::{Address, LabelUuid, UnicastAddress};
use btmesh_common::crypto::application::Aid;
use btmesh_common::{IvIndex, Seq, SeqAuth, SeqZero, Ttl};
use btmesh_device::{
    ApplicationKeyHandle, InboundMetadata, KeyHandle, NetworkKeyHandle, OutboundMetadata,
};
use btmesh_pdu::provisioned::access::AccessMessage;
use btmesh_pdu::provisioned::lower::{LowerPDU, SegmentedLowerPDU, UnsegmentedLowerPDU};
use btmesh_pdu::provisioned::network::CleartextNetworkPDU;
use btmesh_pdu::provisioned::upper::access::UpperAccessPDU;
use btmesh_pdu::provisioned::upper::control::UpperControlPDU;
use btmesh_pdu::provisioned::upper::UpperPDU;
use btmesh_pdu::provisioned::System;
use heapless::Vec;

#[derive(Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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

    pub fn is_replay_protected(&self) -> bool {
        self.replay_protected
    }

    pub fn should_relay(&mut self, relay: bool) {
        self.should_relay = relay;
    }

    pub fn is_relay(&self) -> bool {
        self.should_relay
    }

    pub fn local_element_index(&self) -> Option<u8> {
        self.local_element_index
    }

    pub fn from_upper_pdu(pdu: &UpperPDU<ProvisionedStack>) -> Self {
        Self {
            iv_index: pdu.meta().iv_index(),
            replay_protected: false,
            should_relay: false,
            local_element_index: pdu.meta().local_element_index(),
            network_key_handle: pdu.meta().network_key_handle(),
        }
    }

    pub fn from_upper_control_pdu(pdu: &UpperControlPDU<ProvisionedStack>) -> Self {
        Self {
            iv_index: pdu.meta().iv_index(),
            replay_protected: false,
            should_relay: false,
            local_element_index: pdu.meta().local_element_index(),
            network_key_handle: pdu.meta().network_key_handle(),
        }
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LowerMetadata {
    network_key_handle: NetworkKeyHandle,
    iv_index: IvIndex,
    local_element_index: Option<u8>,
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
            local_element_index: None,
            src,
            dst,
            seq,
            ttl,
        }
    }

    pub fn from_network_pdu(pdu: &CleartextNetworkPDU<ProvisionedStack>) -> Self {
        Self {
            network_key_handle: pdu.meta().network_key_handle(),
            iv_index: pdu.meta().iv_index(),
            local_element_index: pdu.meta().local_element_index(),
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

    pub fn local_element_index(&self) -> Option<u8> {
        self.local_element_index
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UpperMetadata {
    pub(crate) network_key_handle: NetworkKeyHandle,
    pub(crate) iv_index: IvIndex,
    pub(crate) local_element_index: Option<u8>,
    pub(crate) akf_aid: Option<Aid>,
    pub(crate) seq: Seq,
    pub(crate) src: UnicastAddress,
    pub(crate) dst: Address,
    pub(crate) ttl: Ttl,
    pub(crate) label_uuids: Vec<LabelUuid, 3>,
    pub(crate) seq_auth: Option<SeqAuth>,
    pub(crate) replay_seq: Option<Seq>,
}

impl UpperMetadata {
    fn calculate_seq_auth(iv_index: IvIndex, seq: Seq, seq_zero: SeqZero) -> SeqAuth {
        SeqAuth::new((iv_index.value() << 24) + Self::first_seq_number(seq, seq_zero).value())
    }

    fn first_seq_number(seq: Seq, seq_zero: SeqZero) -> Seq {
        if (seq.value() & 8191u32) < seq_zero.value() as u32 {
            Seq::new(seq.value() - ((seq.value() & 8191) - seq_zero.value() as u32) - (8191 + 1))
        } else {
            Seq::new(seq.value() - ((seq.value() & 8191) - seq_zero.value() as u32))
        }
    }

    pub fn from_segmented_lower_pdu(pdu: &SegmentedLowerPDU<ProvisionedStack>) -> Self {
        let iv_index = pdu.meta().iv_index();
        let seq_zero = pdu.seq_zero();
        let seq = pdu.meta().seq();
        let seq_auth = Self::calculate_seq_auth(iv_index, seq, seq_zero);

        Self {
            network_key_handle: pdu.meta().network_key_handle(),
            iv_index: pdu.meta().iv_index(),
            local_element_index: pdu.meta().local_element_index(),
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
            seq_auth: Some(seq_auth),
            replay_seq: Some(Self::first_seq_number(seq, seq_zero)),
        }
    }

    pub fn from_unsegmented_lower_pdu(pdu: &UnsegmentedLowerPDU<ProvisionedStack>) -> Self {
        Self {
            network_key_handle: pdu.meta().network_key_handle(),
            iv_index: pdu.meta().iv_index(),
            local_element_index: pdu.meta().local_element_index(),
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
            seq_auth: None,
            replay_seq: Some(pdu.meta().seq()),
        }
    }

    pub fn from_lower_pdu(pdu: &LowerPDU<ProvisionedStack>) -> Self {
        match pdu {
            LowerPDU::Unsegmented(inner) => Self::from_unsegmented_lower_pdu(inner),
            LowerPDU::Segmented(inner) => Self::from_segmented_lower_pdu(inner),
        }
    }

    pub fn from_access_message(message: &AccessMessage<ProvisionedStack>, seq: Seq) -> Self {
        Self {
            network_key_handle: message.meta().network_key_handle(),
            iv_index: message.meta().iv_index,
            local_element_index: None,
            akf_aid: match message.meta().key_handle() {
                KeyHandle::Device | KeyHandle::Network(_) => None,
                KeyHandle::Application(key_handle) => Some(key_handle.aid()),
            },
            seq,
            src: message.meta().src(),
            dst: message.meta().dst(),
            ttl: message.meta().ttl(),
            label_uuids: Default::default(),
            seq_auth: None,
            replay_seq: Some(seq),
        }
    }

    pub fn network_key_handle(&self) -> NetworkKeyHandle {
        self.network_key_handle
    }

    pub fn iv_index(&self) -> IvIndex {
        self.iv_index
    }

    pub fn local_element_index(&self) -> Option<u8> {
        self.local_element_index
    }

    pub fn aid(&self) -> Option<Aid> {
        self.akf_aid
    }

    pub fn seq(&self) -> Seq {
        self.seq
    }

    pub fn seq_auth(&self) -> Option<SeqAuth> {
        self.seq_auth
    }

    pub fn replay_seq(&self) -> Option<Seq> {
        self.replay_seq
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
        &self.label_uuids
    }

    pub fn add_label_uuid(&mut self, label_uuid: LabelUuid) -> Result<(), DriverError> {
        self.label_uuids
            .push(label_uuid)
            .map_err(|_| DriverError::InsufficientSpace)?;
        Ok(())
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AccessMetadata {
    pub(crate) network_key_handle: NetworkKeyHandle,
    pub(crate) iv_index: IvIndex,
    pub(crate) local_element_index: Option<u8>,
    pub(crate) key_handle: KeyHandle,
    pub(crate) src: UnicastAddress,
    pub(crate) dst: Address,
    pub(crate) ttl: Ttl,
    pub(crate) label_uuid: Option<LabelUuid>,
    pub(crate) replay_seq: Option<Seq>,
}

impl From<(UnicastAddress, OutboundMetadata, Ttl)> for AccessMetadata {
    fn from(tuple: (UnicastAddress, OutboundMetadata, Ttl)) -> Self {
        let src = tuple.0;
        let meta = tuple.1;
        let default_ttl = tuple.2;

        Self {
            network_key_handle: meta.network_key_handle(),
            iv_index: meta.iv_index(),
            local_element_index: None,
            key_handle: meta.key_handle(),
            src,
            dst: meta.dst(),
            ttl: meta.ttl().unwrap_or(default_ttl),
            label_uuid: None,
            replay_seq: None,
        }
    }
}

impl AccessMetadata {
    pub fn from_upper_access_pdu(
        key_handle: KeyHandle,
        label_uuid: Option<LabelUuid>,
        pdu: &UpperAccessPDU<ProvisionedStack>,
    ) -> Self {
        Self {
            network_key_handle: pdu.meta().network_key_handle(),
            iv_index: pdu.meta().iv_index,
            local_element_index: pdu.meta().local_element_index(),
            key_handle,
            src: pdu.meta().src(),
            dst: pdu.meta().dst(),
            ttl: pdu.meta().ttl(),
            label_uuid,
            replay_seq: Some(if let Some(replay_seq) = pdu.meta().replay_seq() {
                replay_seq
            } else {
                pdu.meta().seq()
            }),
        }
    }

    pub(crate) fn replay_seq(&self) -> Option<Seq> {
        self.replay_seq
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

    pub fn local_element_index(&self) -> Option<u8> {
        self.local_element_index
    }
}

impl From<&AccessMetadata> for InboundMetadata {
    fn from(meta: &AccessMetadata) -> Self {
        Self::new(
            meta.src,
            meta.dst,
            meta.ttl,
            meta.network_key_handle,
            meta.iv_index,
            meta.key_handle,
            meta.label_uuid,
        )
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ControlMetadata {}

impl ControlMetadata {
    pub fn from_upper_control_pdu(_pdu: &UpperControlPDU<ProvisionedStack>) -> Self {
        Self {}
    }
}

impl System for ProvisionedStack {
    type NetworkKeyHandle = NetworkKeyHandle;
    type ApplicationKeyHandle = ApplicationKeyHandle;
    type NetworkMetadata = NetworkMetadata;
    type LowerMetadata = LowerMetadata;
    type UpperMetadata = UpperMetadata;
    type AccessMetadata = AccessMetadata;
    type ControlMetadata = ControlMetadata;
}

#[cfg(feature = "defmt")]
impl ::defmt::Format for ProvisionedStack {
    fn format(&self, fmt: ::defmt::Formatter) {
        ::defmt::write!(fmt, "[Provisioned]")
    }
}
