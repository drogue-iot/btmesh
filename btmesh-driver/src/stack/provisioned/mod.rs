use crate::stack::provisioned::lower::LowerDriver;
use crate::stack::provisioned::network::replay_protection::ReplayProtection;
use crate::stack::provisioned::network::{DeviceInfo, NetworkDriver};
use crate::stack::provisioned::sequence::Sequence;
use crate::stack::provisioned::upper::UpperDriver;
use crate::stack::provisioned::transmit_queue::TransmitQueue;
use crate::DriverError;
use btmesh_common::{IvIndex, IvUpdateFlag, Ivi, Seq, Ttl};
use btmesh_pdu::lower::BlockAck;
use btmesh_pdu::network::NetworkPDU;
use btmesh_pdu::Message;
use heapless::Vec;
use secrets::Secrets;

pub mod transmit_queue;
pub mod lower;
pub mod network;
pub mod secrets;
pub mod sequence;
pub mod system;
pub mod upper;

#[derive(Copy, Clone)]
pub struct IvIndexState {
    iv_index: IvIndex,
    iv_update_flag: IvUpdateFlag,
}

impl IvIndexState {
    pub fn accepted_iv_index(&self, ivi: Ivi) -> IvIndex {
        self.iv_index.accepted_iv_index(ivi)
    }

    pub fn transmission_iv_index(&self) -> IvIndex {
        self.iv_index.transmission_iv_index(self.iv_update_flag)
    }
}

pub struct NetworkState {
    iv_index_state: IvIndexState,
}

pub struct ProvisionedStack {
    network_state: NetworkState,
    secrets: Secrets,
    upper: UpperDriver,
    lower: LowerDriver,
    network: NetworkDriver,
    //
    transmit_queue: TransmitQueue,


}

struct ReceiveResult {
    block_ack: Option<BlockAck>,
    message: Option<Message<ProvisionedStack>>,
}

impl TryFrom<(Option<BlockAck>, Option<Message<ProvisionedStack>>)> for ReceiveResult {
    type Error = ();

    fn try_from(
        value: (Option<BlockAck>, Option<Message<ProvisionedStack>>),
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
        }
    }

    fn process_inbound(&mut self, data: &[u8]) -> Result<Option<ReceiveResult>, DriverError> {
        let network_pdu = NetworkPDU::parse(data)?;
        let iv_index = self
            .network_state
            .iv_index_state
            .accepted_iv_index(network_pdu.ivi());
        if let Some(cleartext_network_pdu) = self.try_decrypt_network_pdu(&network_pdu, iv_index)? {
            let (block_ack, upper_pdu) =
                self.process_inbound_cleartext_network_pdu(&cleartext_network_pdu)?;

            let message = if let Some(upper_pdu) = upper_pdu {
                Some(self.process_inbound_upper_pdu(upper_pdu)?)
            } else {
                None
            };

            Ok((block_ack, message).try_into().ok())
        } else {
            // nothing doing, bad result, nothing parsed, keep on truckin'
            Ok(None)
        }
    }

    fn process_outbound(
        &mut self,
        sequence: &Sequence,
        message: &Message<ProvisionedStack>,
    ) -> Result<Vec<NetworkPDU, 32>, DriverError> {
        let upper_pdu = self.process_outbound_message(sequence, message)?;
        let network_pdus = self.process_outbound_upper_pdu(sequence, &upper_pdu, false)?;
        self.transmit_queue.add(upper_pdu, network_pdus.len() as u8)?;

        let network_pdus = network_pdus.iter().map_while(|pdu| {
            self.encrypt_network_pdu(pdu).ok()
        }).collect();

        Ok(network_pdus)
    }
}
