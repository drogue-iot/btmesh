use crate::stack::provisioned::lower::LowerDriver;
use crate::stack::provisioned::network::{DeviceInfo, NetworkDriver};
use crate::stack::provisioned::sequence::Sequence;
use crate::stack::provisioned::transmit_queue::TransmitQueue;
use crate::stack::provisioned::upper::UpperDriver;
use crate::storage::provisioned::subscriptions::Subscriptions;
use crate::storage::provisioned::ProvisionedConfiguration;
use crate::{DriverError, UpperMetadata, Watchdog};
use btmesh_common::{IvIndex, IvUpdateFlag, Ivi, SeqZero};
use btmesh_pdu::provisioned::lower::BlockAck;
use btmesh_pdu::provisioned::network::{CleartextNetworkPDU, NetworkPDU};
use btmesh_pdu::provisioned::Message;
use btmesh_pdu::provisioning::ProvisioningData;
use core::cmp::Ordering;
use core::future::Future;
use embassy_time::{Duration, Timer};
use heapless::Vec;
use secrets::Secrets;

use crate::util::deadline::{Deadline, DeadlineFuture};
use btmesh_common::address::{Address, UnicastAddress};
use btmesh_device::CompletionToken;
use btmesh_pdu::provisioned::control::ControlMessage;
use btmesh_pdu::provisioned::upper::control::ControlOpcode;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub mod lower;
pub mod network;
pub mod secrets;
pub mod sequence;
pub mod system;
pub mod transmit_queue;
pub mod upper;

#[derive(Copy, Clone, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
pub struct IvIndexState {
    iv_index: IvIndex,
    iv_update_flag: IvUpdateFlag,
}

impl IvIndexState {
    pub fn new(iv_index: IvIndex, iv_update_flag: IvUpdateFlag) -> Self {
        Self {
            iv_index,
            iv_update_flag,
        }
    }

    pub fn accepted_iv_index(&self, ivi: Ivi) -> IvIndex {
        self.iv_index.accepted_iv_index(ivi)
    }

    pub fn transmission_iv_index(&self) -> IvIndex {
        self.iv_index.transmission_iv_index(self.iv_update_flag)
    }
}

#[derive(Copy, Clone, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
pub struct NetworkState {
    iv_index_state: IvIndexState,
}

impl NetworkState {
    pub fn display(&self) {
        info!("iv_index: {}", self.iv_index_state.iv_index);
        info!("iv_update_flag: {}", self.iv_index_state.iv_update_flag);
    }

    pub fn new(iv_index: IvIndex, iv_update_flag: IvUpdateFlag) -> Self {
        Self {
            iv_index_state: IvIndexState::new(iv_index, iv_update_flag),
        }
    }

    pub fn iv_index(&self) -> &IvIndexState {
        &self.iv_index_state
    }
}

impl From<ProvisioningData> for NetworkState {
    fn from(data: ProvisioningData) -> Self {
        Self {
            iv_index_state: data.into(),
        }
    }
}

impl From<ProvisioningData> for IvIndexState {
    fn from(data: ProvisioningData) -> Self {
        Self {
            iv_index: IvIndex::new(data.iv_index),
            iv_update_flag: data.iv_update_flag,
        }
    }
}

pub struct ProvisionedStack {
    network_state: NetworkState,
    upper: UpperDriver,
    lower: LowerDriver,
    network: NetworkDriver,
    //
    transmit_queue: TransmitQueue,
    beacon: Deadline,
}

impl From<&ProvisionedConfiguration> for ProvisionedStack {
    fn from(content: &ProvisionedConfiguration) -> Self {
        Self {
            network_state: *content.network_state(),
            upper: Default::default(),
            lower: Default::default(),
            network: NetworkDriver::new(*content.device_info()),
            transmit_queue: Default::default(),
            beacon: Deadline::new(Duration::from_secs(3), true),
        }
    }
}

pub struct ReceiveResult {
    pub block_ack: Option<(BlockAck, UpperMetadata)>,
    pub message: Option<Message<ProvisionedStack>>,
    pub dst: Address,
}

impl
    TryFrom<(
        Option<(BlockAck, UpperMetadata)>,
        Option<Message<ProvisionedStack>>,
        Address,
    )> for ReceiveResult
{
    type Error = ();

    fn try_from(
        value: (
            Option<(BlockAck, UpperMetadata)>,
            Option<Message<ProvisionedStack>>,
            Address,
        ),
    ) -> Result<Self, Self::Error> {
        match value {
            (None, None, _) => Err(()),
            _ => Ok(ReceiveResult {
                block_ack: value.0,
                message: value.1,
                dst: value.2,
            }),
        }
    }
}

impl ProvisionedStack {
    pub fn new(device_info: DeviceInfo, network_state: NetworkState) -> Self {
        Self {
            network_state,
            upper: Default::default(),
            lower: Default::default(),
            network: NetworkDriver::new(device_info),
            transmit_queue: Default::default(),
            beacon: Deadline::new(Duration::from_secs(3), true),
        }
    }

    pub fn has_ongoing_completion(&self) -> bool {
        self.transmit_queue.has_ongoing_completion()
    }

    pub fn network_state(&self) -> NetworkState {
        self.network_state
    }

    pub fn device_info(&self) -> DeviceInfo {
        self.network.device_info()
    }

    pub fn next_beacon_deadline(&self) -> Option<DeadlineFuture<'_>> {
        Some(self.beacon.next())
    }

    pub fn next_retransmit(&self) -> Option<impl Future<Output = ()>> {
        Some(Timer::after(Duration::from_millis(200)))
    }

    pub fn retransmit(
        &mut self,
        secrets: &Secrets,
        sequence: &Sequence,
    ) -> Result<Vec<NetworkPDU, 16>, DriverError> {
        let mut pdus = Vec::new();

        let upper_pdus: Vec<_, 8> = self.transmit_queue.iter().collect();

        for upper_pdu in upper_pdus {
            for network_pdu in self
                .process_outbound_upper_pdu::<8>(sequence, &upper_pdu, true)?
                .iter()
                .map_while(|pdu| self.encrypt_network_pdu(secrets, pdu).ok())
            {
                pdus.push(network_pdu)
                    .map_err(|_| DriverError::InsufficientSpace)?;
            }
        }
        Ok(pdus)
    }

    pub fn process_inbound_network_pdu(
        &mut self,
        secrets: &Secrets,
        network_pdu: &NetworkPDU,
        watchdog: &Watchdog,
        is_loopback: bool,
        subscriptions: &Subscriptions,
    ) -> Result<
        (
            Option<CleartextNetworkPDU<ProvisionedStack>>,
            Option<ReceiveResult>,
        ),
        DriverError,
    > {
        let iv_index = self
            .network_state
            .iv_index_state
            .accepted_iv_index(network_pdu.ivi());

        if let Some(mut cleartext_network_pdu) =
            self.try_decrypt_network_pdu(secrets, network_pdu, iv_index)?
        {
            // Ignore hearing ourselves
            if self
                .device_info()
                .is_local_unicast(Address::Unicast(cleartext_network_pdu.src()))
            {
                return Ok((None, None));
            }
            if cleartext_network_pdu.meta().is_replay_protected() {
                return Ok((None, None));
            }

            #[cfg(feature = "relay")]
            if !is_loopback {
                // do not relay loopback'd pdus.
                if self
                    .device_info()
                    .is_local_unicast(cleartext_network_pdu.dst())
                {
                    // do not relay if we're the actual destination
                    cleartext_network_pdu.meta_mut().should_relay(false)
                } else {
                    // see if the cache knows about it.
                    self.network
                        .network_message_cache
                        .check(&mut cleartext_network_pdu);
                }
            }

            let (block_ack_meta, mut upper_pdu) = self.process_inbound_cleartext_network_pdu(
                &cleartext_network_pdu,
                watchdog,
                subscriptions,
            )?;

            if let Some((block_ack, meta)) = &block_ack_meta {
                if let Some(replacement_block_ack) = self.network.replay_protection.check_upper_pdu(
                    meta,
                    block_ack,
                    upper_pdu.is_some(),
                ) {
                    // we have already seen it and fully ack'd it, so just keep ack'ing for now.
                    return Ok((
                        None,
                        (
                            Some((replacement_block_ack, meta.clone())),
                            None,
                            cleartext_network_pdu.dst(),
                        )
                            .try_into()
                            .ok(),
                    ));
                }
            }

            let message = if let Some(upper_pdu) = &mut upper_pdu {
                self.process_inbound_upper_pdu(secrets, upper_pdu).ok()
            } else {
                None
            };

            let dst = cleartext_network_pdu.dst();

            let relay_pdu = if cleartext_network_pdu.meta().is_relay() {
                Some(cleartext_network_pdu)
            } else {
                None
            };

            Ok((relay_pdu, (block_ack_meta, message, dst).try_into().ok()))
        } else {
            // nothing doing, bad result, nothing parsed, keep on truckin'
            Ok((None, None))
        }
    }

    // todo: remove this once we match more control messages.
    #[allow(clippy::single_match)]
    pub fn process_inbound_control(
        &mut self,
        message: &ControlMessage<ProvisionedStack>,
        watchdog: &Watchdog,
    ) -> Result<(), DriverError> {
        match message.opcode() {
            ControlOpcode::SegmentAcknowledgement => {
                if let Ok(block_ack) = message.try_into() {
                    self.transmit_queue.receive_ack(block_ack, watchdog)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    pub fn process_outbound(
        &mut self,
        secrets: &Secrets,
        sequence: &Sequence,
        message: &Message<ProvisionedStack>,
        completion_token: Option<CompletionToken>,
        watchdog: &Watchdog,
        retransmits: u8,
    ) -> Result<Vec<NetworkPDU, 8>, DriverError> {
        let upper_pdu = self.process_outbound_message(secrets, sequence, message)?;
        let network_pdus = self.process_outbound_upper_pdu::<8>(sequence, &upper_pdu, false)?;

        match network_pdus.len().cmp(&1) {
            Ordering::Less => { /* nothing */ }
            Ordering::Equal => {
                self.transmit_queue
                    .add_nonsegmented(upper_pdu, retransmits, completion_token)?;
            }
            Ordering::Greater => {
                self.transmit_queue.add_segmented(
                    upper_pdu,
                    network_pdus.len() as u8,
                    completion_token,
                    watchdog,
                    retransmits,
                )?;
            }
        }

        let network_pdus = network_pdus
            .iter()
            .map_while(|pdu| self.encrypt_network_pdu(secrets, pdu).ok())
            .collect();

        Ok(network_pdus)
    }

    pub fn process_outbound_relay_network_pdu(
        &mut self,
        secrets: &Secrets,
        pdu: &CleartextNetworkPDU<ProvisionedStack>,
    ) -> Result<Option<NetworkPDU>, DriverError> {
        let orig = pdu;
        let pdu = orig.relay()?;
        if let Some(pdu) = pdu {
            Ok(Some(self.encrypt_network_pdu(secrets, &pdu)?))
        } else {
            Ok(None)
        }
    }

    pub fn outbound_expiration(&mut self, seq_zero: &SeqZero) {
        self.transmit_queue.expire_outbound(seq_zero);
    }

    pub fn inbound_expiration(
        &mut self,
        secrets: &Secrets,
        sequence: &Sequence,
        seq_zero: &SeqZero,
        src: &UnicastAddress,
        watchdog: &Watchdog,
    ) -> Result<Option<NetworkPDU>, DriverError> {
        if let Some((block_ack, meta)) = self.lower.expire_inbound(seq_zero, watchdog) {
            // We only send acks for unicast addresses
            if meta.dst().is_unicast() {
                self.process_outbound_block_ack(secrets, sequence, block_ack, &meta, src)
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}
