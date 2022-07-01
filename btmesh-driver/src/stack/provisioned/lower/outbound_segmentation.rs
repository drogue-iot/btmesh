use crate::stack::provisioned::sequence::Sequence;
use crate::stack::provisioned::system::NetworkMetadata;
use crate::stack::provisioned::ProvisionedStack;
use crate::DriverError;
use btmesh_common::mic::SzMic;
use btmesh_common::{Ctl, InsufficientBuffer, Ttl};
use btmesh_pdu::lower::access::SegmentedLowerAccessPDU;
use btmesh_pdu::network::CleartextNetworkPDU;
use btmesh_pdu::upper::UpperPDU;
use heapless::Vec;

const SEGMENTED_ACCESS_MTU: usize = 12;
const NONSEGMENTED_ACCESS_MUT: usize = 15;

#[derive(Default)]
pub struct OutboundSegmentation {}

impl OutboundSegmentation {
    pub fn process(
        &mut self,
        sequence: &Sequence,
        pdu: &UpperPDU<ProvisionedStack>,
        is_retransmit: bool,
    ) -> Result<Vec<CleartextNetworkPDU<ProvisionedStack>, 32>, DriverError> {
        let meta = NetworkMetadata::from_upper_pdu(pdu);
        let mut result = Vec::new();

        match pdu {
            UpperPDU::Access(inner) => {
                if inner.payload().len() <= NONSEGMENTED_ACCESS_MUT {
                    result
                        .push(CleartextNetworkPDU::new(
                            pdu.meta().iv_index().ivi(),
                            pdu.meta().network_key_handle().nid(),
                            Ctl::Access,
                            pdu.meta().ttl(),
                            pdu.meta().seq(),
                            pdu.meta().src(),
                            pdu.meta().dst(),
                            inner.payload(),
                            meta,
                        )?)
                        .map_err(|_| InsufficientBuffer)?;
                } else {
                    let seq_zero = inner.meta().seq().into();
                    let payload = inner.payload().chunks(SEGMENTED_ACCESS_MTU);
                    let seg_n = payload.len() - 1;

                    for (seg_o, segment_m) in payload.enumerate() {
                        let seq = if ! is_retransmit && seg_o == 0 {
                            pdu.meta().seq()
                        } else {
                            sequence.next()
                        };

                        // it's just a pass-through, so the `()`-centric System is perfectly good.
                        let lower_pdu = SegmentedLowerAccessPDU::<()>::new(
                            pdu.meta().aid(),
                            SzMic::Bit32,
                            seq_zero,
                            seg_o as u8,
                            seg_n as u8,
                            segment_m,
                            (),
                        )?;

                        let mut transport_pdu = Vec::<_, SEGMENTED_ACCESS_MTU>::new();
                        lower_pdu.emit(&mut transport_pdu)?;

                        result
                            .push(CleartextNetworkPDU::new(
                                pdu.meta().iv_index().ivi(),
                                pdu.meta().network_key_handle().nid(),
                                Ctl::Access,
                                pdu.meta().ttl(),
                                seq,
                                pdu.meta().src(),
                                pdu.meta().dst(),
                                &*transport_pdu,
                                meta,
                            )?)
                            .map_err(|_| InsufficientBuffer)?;
                    }
                }
            }
            UpperPDU::Control(inner) => {}
        }
        Ok(result)
    }
}
