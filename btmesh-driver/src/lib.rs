#![cfg_attr(not(test), no_std)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(associated_type_defaults)]
#![allow(dead_code)]

use btmesh_common::{Seq, Uuid};
use btmesh_pdu::provisioning::Capabilities;
use btmesh_pdu::PDU;
use rand_core::{CryptoRng, RngCore};

mod error;
pub mod stack;

mod util;

use crate::stack::interface::NetworkInterfaces;
use crate::stack::provisioned::network::DeviceInfo;
use crate::stack::provisioned::secrets::Secrets;
use crate::stack::provisioned::sequence::Sequence;
use crate::stack::provisioned::system::UpperMetadata;
use crate::stack::provisioned::{NetworkState, ProvisionedStack};
use crate::stack::unprovisioned::{ProvisioningState, UnprovisionedStack};
use crate::stack::Stack;
pub use error::DriverError;

pub struct Driver<N: NetworkInterfaces, R: RngCore + CryptoRng> {
    stack: Stack,
    network: N,
    rng: R,
}

impl<N: NetworkInterfaces, R: RngCore + CryptoRng> Driver<N, R> {
    pub fn new_unprovisioned(network: N, rng: R, capabilities: Capabilities, uuid: Uuid) -> Self {
        let num_elements = capabilities.number_of_elements;
        Self {
            stack: Stack::Unprovisioned(UnprovisionedStack::new(capabilities), num_elements, uuid),
            rng,
            network,
        }
    }

    pub fn new_provisioned(
        network: N,
        rng: R,
        device_info: DeviceInfo,
        secrets: Secrets,
        network_state: NetworkState,
        sequence: Sequence,
    ) -> Self {
        Self {
            stack: Stack::Provisioned(
                ProvisionedStack::new(device_info, secrets, network_state),
                sequence,
            ),
            rng,
            network,
        }
    }

    /// Perform a single end-to-end loop through the driver's processing logic.
    pub async fn process(&mut self) -> Result<(), DriverError> {
        let device_state = self.stack.device_state();

        let pdu = self.network.receive(&device_state).await?;
        match (&pdu, &mut self.stack) {
            (PDU::Provisioning(pdu), Stack::Unprovisioned(stack, num_elements, _uuid)) => {
                if let Some(provisioning_state) = stack.process(pdu, &mut self.rng)? {
                    match provisioning_state {
                        ProvisioningState::Response(pdu) => {
                            self.network.transmit(&PDU::Provisioning(pdu)).await?;
                        }
                        ProvisioningState::Data(device_key, provisioning_data) => {
                            let primary_unicast_addr = provisioning_data.unicast_address;
                            let device_info = DeviceInfo::new(primary_unicast_addr, *num_elements);
                            let secrets = (device_key, provisioning_data).into();
                            let network_state = provisioning_data.into();

                            self.stack = Stack::Provisioned(
                                ProvisionedStack::new(device_info, secrets, network_state),
                                Sequence::new(Seq::new(800)),
                            );
                        }
                    }
                }
            }
            (PDU::Network(pdu), Stack::Provisioned(stack, sequence)) => {
                if let Some(result) = stack.process_inbound_network_pdu(pdu)? {
                    if let Some((block_ack, meta)) = result.block_ack {
                        // send outbound block-ack
                        for network_pdu in
                            stack.process_outbound_block_ack(sequence, block_ack, meta)?
                        {
                            // don't error if we can't send.
                            self.network.transmit(&PDU::Network(network_pdu)).await.ok();
                        }
                    }

                    if let Some(_message) = result.message {
                        // dispatch to element(s)
                    }
                }
            }
            _ => {
                // PDU incompatible with stack state; ignore.
            }
        }
        Ok(())
    }
}

pub enum DeviceState {
    Unprovisioned { uuid: Uuid },
    Provisioned,
}
