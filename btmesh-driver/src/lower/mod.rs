//mod old_inbound_segmentation;
mod inbound_segmentation;

use crate::lower::inbound_segmentation::InboundSegmentation;
use crate::{Driver, DriverError, LowerMetadata, UpperMetadata};
use btmesh_common::mic::SzMic;
use btmesh_pdu::lower::{BlockAck, LowerPDU, UnsegmentedLowerPDU};
use btmesh_pdu::network::CleartextNetworkPDU;
use btmesh_pdu::upper::access::UpperAccessPDU;
use btmesh_pdu::upper::control::UpperControlPDU;
use btmesh_pdu::upper::UpperPDU;

pub struct LowerDriver {
    inbound_segmentation: InboundSegmentation,
}

impl LowerDriver {
    /// Process a *cleartext* `NetworkPDU`, through hidden `LowerPDU`s, accommodating segmentation & reassembly,
    /// to produce an `UpperPDU` if sufficiently unsegmented or re-assembled.
    fn process_cleartext_network_pdu(
        &mut self,
        network_pdu: &CleartextNetworkPDU<Driver>,
    ) -> Result<(Option<BlockAck>, Option<UpperPDU<Driver>>), DriverError> {
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
                let (block_ack, upper_pdu) = self.inbound_segmentation.process(inner)?;
                let upper_pdu = if let Some(upper_pdu) = upper_pdu {
                    Some(upper_pdu)
                } else {
                    None
                };
                Ok((Some(block_ack), upper_pdu))
            }
        }
    }
}
