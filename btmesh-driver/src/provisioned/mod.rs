use crate::provisioned::lower::LowerDriver;
use crate::provisioned::network::replay_protection::ReplayProtection;
use crate::provisioned::network::{DeviceInfo, NetworkDriver};
use crate::provisioned::upper::UpperDriver;
use crate::DriverError;
use btmesh_common::{IvIndex, IvUpdateFlag, Ivi, Seq};
use btmesh_pdu::access::AccessMessage;
use btmesh_pdu::lower::BlockAck;
use btmesh_pdu::network::NetworkPDU;
use btmesh_pdu::upper::UpperPDU;
use btmesh_pdu::{Message, System};
use secrets::Secrets;
use system::{
    AccessMetadata, ApplicationKeyHandle, LowerMetadata, NetworkKeyHandle, NetworkMetadata,
    UpperMetadata,
};

pub mod lower;
pub mod network;
pub mod secrets;
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

pub struct ProvisionedDriver {
    network_state: NetworkState,
    secrets: Secrets,
    upper: UpperDriver,
    lower: LowerDriver,
    network: NetworkDriver,
}

struct ReceiveResult {
    block_ack: Option<BlockAck>,
    message: Option<Message<ProvisionedDriver>>,
}

impl TryFrom<(Option<BlockAck>, Option<Message<ProvisionedDriver>>)> for ReceiveResult {
    type Error = ();

    fn try_from(value: (Option<BlockAck>, Option<Message<ProvisionedDriver>>)) -> Result<Self, Self::Error> {
        match value {
            (None, None) => Err(()),
            _ => Ok(
                ReceiveResult { block_ack: value.0, message: value.1 }
            ),
        }
    }
}

impl ProvisionedDriver {
    fn new(device_info: DeviceInfo, secrets: Secrets, network_state: NetworkState) -> Self {
        Self {
            secrets,
            network_state,
            upper: UpperDriver::new(Seq::new(0)),
            lower: Default::default(),
            network: NetworkDriver::new(device_info),
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

    fn process_outbound(&mut self, message: &Message<ProvisionedDriver>) -> Result<(), DriverError> {
        let upper_pdu = self.process_outbound_message(message)?;

        todo!()
    }
}
