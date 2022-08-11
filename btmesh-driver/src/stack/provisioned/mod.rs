use crate::stack::provisioned::lower::LowerDriver;
use crate::stack::provisioned::network::replay_protection::ReplayProtection;
use crate::stack::provisioned::network::{DeviceInfo, NetworkDriver};
use crate::stack::provisioned::sequence::Sequence;
use crate::stack::provisioned::transmit_queue::TransmitQueue;
use crate::stack::provisioned::upper::UpperDriver;
use crate::storage::provisioned::ProvisionedConfiguration;
use crate::{DriverError, UpperMetadata};
use btmesh_common::{IvIndex, IvUpdateFlag, Ivi, Seq};
use btmesh_pdu::provisioned::lower::BlockAck;
use btmesh_pdu::provisioned::network::NetworkPDU;
use btmesh_pdu::provisioned::Message;
use btmesh_pdu::provisioning::ProvisioningData;
use core::future::Future;
use embassy_executor::time::{Duration, Timer};
use heapless::Vec;
use secrets::Secrets;

use crate::stack::provisioned::system::ControlMetadata;
use crate::util::deadline::{Deadline, DeadlineFuture};
use crate::DeviceState::Provisioned;
use btmesh_pdu::provisioned::control::ControlMessage;
use btmesh_pdu::provisioned::upper::control::{ControlOpcode, UpperControlPDU};
use btmesh_pdu::provisioned::upper::UpperPDU;
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
    pub(crate) fn new(iv_index: IvIndex, iv_update_flag: IvUpdateFlag) -> Self {
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

    pub(crate) fn new(iv_index: IvIndex, iv_update_flag: IvUpdateFlag) -> Self {
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
    secrets: Secrets,
    upper: UpperDriver,
    lower: LowerDriver,
    network: NetworkDriver,
    //
    transmit_queue: TransmitQueue,
    beacon: Deadline,
}

impl From<ProvisionedConfiguration> for ProvisionedStack {
    fn from(content: ProvisionedConfiguration) -> Self {
        Self {
            network_state: content.network_state(),
            secrets: content.secrets(),
            upper: Default::default(),
            lower: Default::default(),
            network: NetworkDriver::new(content.device_info()),
            transmit_queue: Default::default(),
            beacon: Deadline::new(Duration::from_secs(3), true),
        }
    }
}

pub struct ReceiveResult {
    pub block_ack: Option<(BlockAck, UpperMetadata)>,
    pub message: Option<Message<ProvisionedStack>>,
}

impl
    TryFrom<(
        Option<(BlockAck, UpperMetadata)>,
        Option<Message<ProvisionedStack>>,
    )> for ReceiveResult
{
    type Error = ();

    fn try_from(
        value: (
            Option<(BlockAck, UpperMetadata)>,
            Option<Message<ProvisionedStack>>,
        ),
    ) -> Result<Self, Self::Error> {
        match value {
            (None, None) => Err(()),
            _ => Ok(ReceiveResult {
                block_ack: value.0,
                message: value.1,
            }),
        }
    }
}

impl ProvisionedStack {
    pub fn new(device_info: DeviceInfo, secrets: Secrets, network_state: NetworkState) -> Self {
        Self {
            secrets,
            network_state,
            upper: Default::default(),
            lower: Default::default(),
            network: NetworkDriver::new(device_info),
            transmit_queue: Default::default(),
            beacon: Deadline::new(Duration::from_secs(3), true),
        }
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
        Some(Timer::after(Duration::from_millis(100)))
    }

    pub fn retransmit(&mut self, sequence: &Sequence) -> Result<Vec<NetworkPDU, 64>, DriverError> {
        let mut pdus = Vec::new();

        let upper_pdus: Vec<UpperPDU<ProvisionedStack>, 16> = self.transmit_queue.iter().collect();

        for upper_pdu in upper_pdus {
            for network_pdu in self
                .process_outbound_upper_pdu(sequence, &upper_pdu, true)?
                .iter()
                .map_while(|pdu| self.encrypt_network_pdu(pdu).ok())
            {
                pdus.push(network_pdu);
            }
        }

        Ok(pdus)
    }

    pub fn process_inbound_network_pdu(
        &mut self,
        network_pdu: &NetworkPDU,
    ) -> Result<Option<ReceiveResult>, DriverError> {
        let iv_index = self
            .network_state
            .iv_index_state
            .accepted_iv_index(network_pdu.ivi());

        if let Some(cleartext_network_pdu) = self.try_decrypt_network_pdu(network_pdu, iv_index)? {
            let (block_ack_meta, upper_pdu) =
                self.process_inbound_cleartext_network_pdu(&cleartext_network_pdu)?;

            let message = if let Some(upper_pdu) = upper_pdu {
                Some(self.process_inbound_upper_pdu(upper_pdu)?)
            } else {
                None
            };

            Ok((block_ack_meta, message).try_into().ok())
        } else {
            // nothing doing, bad result, nothing parsed, keep on truckin'
            Ok(None)
        }
    }

    pub(crate) fn secrets(&self) -> &Secrets {
        &self.secrets
    }

    pub fn process_inbound_control(&mut self, message: &ControlMessage<ProvisionedStack>) {
        match message.opcode() {
            ControlOpcode::SegmentAcknowledgement => {
                if let Ok(block_ack) = message.try_into() {
                    self.transmit_queue.receive_ack(block_ack);
                }
            }
            _ => {}
        }
    }

    pub fn process_outbound(
        &mut self,
        sequence: &Sequence,
        message: &Message<ProvisionedStack>,
    ) -> Result<Vec<NetworkPDU, 32>, DriverError> {
        let upper_pdu = self.process_outbound_message(sequence, message)?;
        let network_pdus = self.process_outbound_upper_pdu(sequence, &upper_pdu, false)?;
        self.transmit_queue
            .add(upper_pdu, network_pdus.len() as u8)?;

        let network_pdus = network_pdus
            .iter()
            .map_while(|pdu| self.encrypt_network_pdu(pdu).ok())
            .collect();

        Ok(network_pdus)
    }
}
