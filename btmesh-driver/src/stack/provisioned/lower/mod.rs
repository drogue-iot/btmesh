mod inbound_segmentation;
mod outbound_segmentation;

use crate::stack::provisioned::lower::inbound_segmentation::InboundSegmentation;
use crate::stack::provisioned::lower::outbound_segmentation::OutboundSegmentation;
use crate::stack::provisioned::sequence::Sequence;
use crate::stack::provisioned::system::{LowerMetadata, UpperMetadata};
use crate::stack::provisioned::ProvisionedStack;
use crate::DriverError;
use btmesh_common::mic::SzMic;
use btmesh_pdu::provisioned::lower::{BlockAck, LowerPDU, UnsegmentedLowerPDU};
use btmesh_pdu::provisioned::network::{CleartextNetworkPDU, NetworkPDU};
use btmesh_pdu::provisioned::upper::access::UpperAccessPDU;
use btmesh_pdu::provisioned::upper::control::{ControlOpcode, UpperControlPDU};
use btmesh_pdu::provisioned::upper::UpperPDU;
use core::borrow::Borrow;
use core::ops::Deref;
use heapless::Vec;
use btmesh_common::InsufficientBuffer;

#[derive(Default)]
pub struct LowerDriver {
    inbound_segmentation: InboundSegmentation,
    outbound_segmentation: OutboundSegmentation,
}

impl ProvisionedStack {
    /// Process a *cleartext* `NetworkPDU`, through hidden `LowerPDU`s, accommodating segmentation & reassembly,
    /// to produce an `UpperPDU` if sufficiently unsegmented or re-assembled.
    pub fn process_inbound_cleartext_network_pdu(
        &mut self,
        network_pdu: &CleartextNetworkPDU<ProvisionedStack>,
    ) -> Result<(Option<(BlockAck, UpperMetadata)>, Option<UpperPDU<ProvisionedStack>>), DriverError> {
        let lower_pdu = LowerPDU::parse(network_pdu, LowerMetadata::from_network_pdu(network_pdu))?;

        match &lower_pdu {
            LowerPDU::Unsegmented(inner) => match inner {
                UnsegmentedLowerPDU::Access(access_pdu) => Ok((
                    None,
                    Some(
                        UpperAccessPDU::parse(
                            access_pdu.upper_pdu(),
                            SzMic::Bit32,
                            UpperMetadata::from_unsegmented_lower_pdu(inner),
                        )?
                        .into(),
                    ),
                )),
                UnsegmentedLowerPDU::Control(control_pdu) => Ok((
                    None,
                    Some(
                        UpperControlPDU::new(
                            control_pdu.opcode(),
                            control_pdu.parameters(),
                            UpperMetadata::from_unsegmented_lower_pdu(inner),
                        )?
                        .into(),
                    ),
                )),
            },
            LowerPDU::Segmented(inner) => {
                let ((block_ack, meta), upper_pdu) = self.lower.inbound_segmentation.process(inner)?;
                Ok((Some((block_ack, meta)), upper_pdu))
            }
        }
    }

    pub fn process_outbound_block_ack(
        &mut self,
        sequence: &Sequence,
        block_ack: BlockAck,
        meta: UpperMetadata,
    ) -> Result<Vec<NetworkPDU, 32>, DriverError> {
        let network_pdus =
            self.process_outbound_upper_pdu(sequence, &block_ack_to_upper_pdu(block_ack, meta)?, false)?;

        let network_pdus = network_pdus
            .iter()
            .map_while(|pdu| self.encrypt_network_pdu(pdu).ok())
            .collect();

        Ok(network_pdus)
    }

    pub fn process_outbound_upper_pdu(
        &mut self,
        sequence: &Sequence,
        upper_pdu: &UpperPDU<ProvisionedStack>,
        is_retransmit: bool,
    ) -> Result<Vec<CleartextNetworkPDU<ProvisionedStack>, 32>, DriverError> {
        self.lower
            .outbound_segmentation
            .process(sequence, upper_pdu, is_retransmit)
    }
}

fn block_ack_to_upper_pdu(block_ack: BlockAck, meta: UpperMetadata) -> Result<UpperPDU<ProvisionedStack>, InsufficientBuffer> {
    let mut parameters = [0; 6];

    let seq_zero = (block_ack.seq_zero().value() << 2).to_be_bytes();
    parameters[0] = seq_zero[0];
    parameters[1] = seq_zero[1];

    let block_ack = block_ack.value().to_be_bytes();
    parameters[2] = block_ack[0];
    parameters[3] = block_ack[1];
    parameters[4] = block_ack[2];
    parameters[5] = block_ack[3];

    Ok(UpperControlPDU::new( ControlOpcode::SegmentAcknowledgement, &parameters, meta)?.into())
}
