//mod old_inbound_segmentation;
mod inbound_segmentation;

use crate::lower::inbound_segmentation::InboundSegmentation;
use crate::{Driver, DriverError};
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
        match &apply_metadata(network_pdu, LowerPDU::parse(network_pdu)?) {
            LowerPDU::Unsegmented(lower_pdu) => match lower_pdu {
                UnsegmentedLowerPDU::Access(access_pdu) => Ok((
                    None,
                    Some(UpperAccessPDU::parse(access_pdu.upper_pdu(), SzMic::Bit32)?.into()),
                )),
                UnsegmentedLowerPDU::Control(control_pdu) => Ok((
                    None,
                    Some(
                        UpperControlPDU::new(control_pdu.opcode(), control_pdu.parameters())?
                            .into(),
                    ),
                )),
            },
            LowerPDU::Segmented(lower_pdu) => {
                let (block_ack, upper_pdu) = self.inbound_segmentation.process(lower_pdu)?;
                Ok((Some(block_ack), upper_pdu))
            }
        }
    }
}

fn apply_metadata(
    network_pdu: &CleartextNetworkPDU<Driver>,
    mut lower_pdu: LowerPDU<Driver>,
) -> LowerPDU<Driver> {
    lower_pdu.meta_mut().apply(network_pdu);
    lower_pdu
}
