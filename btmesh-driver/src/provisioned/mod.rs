use crate::provisioned::lower::LowerDriver;
use crate::provisioned::network::replay_protection::ReplayProtection;
use crate::provisioned::network::{DeviceInfo, NetworkDriver};
use crate::provisioned::upper::UpperDriver;
use crate::DriverError;
use btmesh_common::{Ivi, IvIndex, IvUpdateFlag};
use btmesh_pdu::network::NetworkPDU;
use btmesh_pdu::System;
use secrets::Secrets;
use system::{AccessMetadata, ApplicationKeyHandle, LowerMetadata, NetworkKeyHandle, NetworkMetadata, UpperMetadata};

pub mod lower;
pub mod network;
pub mod secrets;
pub mod upper;
pub mod system;

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

impl ProvisionedDriver {
    fn new(device_info: DeviceInfo, secrets: Secrets, network_state: NetworkState) -> Self {
        Self {
            secrets,
            network_state,
            upper: Default::default(),
            lower: Default::default(),
            network: NetworkDriver::new(device_info),
        }
    }

    fn receive(&mut self, data: &[u8]) -> Result<(), DriverError> {
        let network_pdu = NetworkPDU::parse(data)?;
        let iv_index = self
            .network_state
            .iv_index_state
            .accepted_iv_index(network_pdu.ivi());
        if let Some(cleartext_network_pdu) = self.try_decrypt_network_pdu(&network_pdu, iv_index)? {
            let (block_ack, upper_pdu) =
                self.process_cleartext_network_pdu(&cleartext_network_pdu)?;

            if let Some(block_ack) = block_ack {

            }

            if let Some(upper_pdu) = upper_pdu {
                let access_message = self.process_upper_pdu(upper_pdu)?;
            }
        }

        Ok(())
    }
}

impl System for ProvisionedDriver {
    type NetworkKeyHandle = NetworkKeyHandle;
    type ApplicationKeyHandle = ApplicationKeyHandle;
    type NetworkMetadata = NetworkMetadata;
    type LowerMetadata = LowerMetadata;
    type UpperMetadata = UpperMetadata;
    type AccessMetadata = AccessMetadata;
}
