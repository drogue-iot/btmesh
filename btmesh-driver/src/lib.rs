#![cfg_attr(not(test), no_std)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(associated_type_defaults)]
#![allow(dead_code)]

use btmesh_common::{IvIndex, Seq, Uuid};
use btmesh_pdu::PDU;

mod error;
pub mod stack;
pub mod unprovisioned;

use crate::stack::interface::NetworkInterfaces;
use crate::stack::provisioned::network::DeviceInfo;
use crate::stack::provisioned::secrets::Secrets;
use crate::stack::provisioned::sequence::Sequence;
use crate::stack::provisioned::system::UpperMetadata;
use crate::stack::provisioned::{NetworkState, ProvisionedStack};
use crate::stack::Stack;
pub use error::DriverError;

pub struct Driver<N: NetworkInterfaces> {
    stack: Stack,
    network: N,
}

impl<N: NetworkInterfaces> Driver<N> {
    pub fn new_provisioned(
        network: N,
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
            network,
        }
    }

    /// Perform a single end-to-end loop through the driver's processing logic.
    pub async fn process(&mut self) -> Result<(), DriverError> {
        let device_state = self.stack.device_state();

        let iv_index = todo!();

        let pdu = self.network.receive(&device_state).await?;
        match (&pdu, &mut self.stack) {
            (PDU::Provisioning(pdu), Stack::Unprovisioned(stack)) => {}
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

                    if let Some(message) = result.message {
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
