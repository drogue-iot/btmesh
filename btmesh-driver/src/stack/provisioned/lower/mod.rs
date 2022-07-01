mod inbound_segmentation;
mod outbound_segmentation;

use crate::stack::provisioned::lower::inbound_segmentation::InboundSegmentation;
use crate::stack::provisioned::lower::outbound_segmentation::OutboundSegmentation;
use crate::stack::provisioned::sequence::Sequence;
use crate::stack::provisioned::system::{LowerMetadata, UpperMetadata};
use crate::stack::provisioned::ProvisionedStack;
use crate::DriverError;
use btmesh_common::mic::SzMic;
use btmesh_common::Ttl;
use btmesh_pdu::lower::{BlockAck, LowerPDU, UnsegmentedLowerPDU};
use btmesh_pdu::network::CleartextNetworkPDU;
use btmesh_pdu::upper::access::UpperAccessPDU;
use btmesh_pdu::upper::control::UpperControlPDU;
use btmesh_pdu::upper::UpperPDU;
use heapless::Vec;

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
    ) -> Result<(Option<BlockAck>, Option<UpperPDU<ProvisionedStack>>), DriverError> {
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
                let (block_ack, upper_pdu) = self.lower.inbound_segmentation.process(inner)?;
                Ok((Some(block_ack), upper_pdu))
            }
        }
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
