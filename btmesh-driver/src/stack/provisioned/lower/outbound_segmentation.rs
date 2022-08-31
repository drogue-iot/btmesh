use crate::stack::provisioned::sequence::Sequence;
use crate::stack::provisioned::system::NetworkMetadata;
use crate::stack::provisioned::ProvisionedStack;
use crate::DriverError;
use btmesh_common::mic::SzMic;
use btmesh_common::{Ctl, InsufficientBuffer};
use btmesh_pdu::provisioned::lower::access::{SegmentedLowerAccessPDU, UnsegmentedLowerAccessPDU};
use btmesh_pdu::provisioned::lower::control::UnsegmentedLowerControlPDU;
use btmesh_pdu::provisioned::network::CleartextNetworkPDU;
use btmesh_pdu::provisioned::upper::control::UpperControlPDU;
use btmesh_pdu::provisioned::upper::UpperPDU;
use heapless::Vec;

const SEGMENTED_ACCESS_MTU: usize = 12;
const NONSEGMENTED_ACCESS_MUT: usize = 15;

const SEGMENT_LOWER_PDU_SIZE: usize = SEGMENTED_ACCESS_MTU + 4;

#[derive(Default)]
pub struct OutboundSegmentation {}

impl OutboundSegmentation {
    pub fn process<const N: usize>(
        &mut self,
        sequence: &Sequence,
        pdu: &UpperPDU<ProvisionedStack>,
        is_retransmit: bool,
    ) -> Result<Vec<CleartextNetworkPDU<ProvisionedStack>, N>, DriverError> {
        let meta = NetworkMetadata::from_upper_pdu(pdu);
        let mut result = Vec::new();

        match pdu {
            UpperPDU::Access(inner) => {
                let mut payload = Vec::<_, 380>::new();
                inner.emit(&mut payload)?;

                if payload.len() <= NONSEGMENTED_ACCESS_MUT {
                    let lower_pdu =
                        UnsegmentedLowerAccessPDU::<()>::new(inner.meta().aid(), &payload, ())?;

                    let mut transport_pdu = Vec::<_, 16>::new();
                    lower_pdu
                        .emit(&mut transport_pdu)
                        .map_err(|_| DriverError::InsufficientSpace)?;

                    result
                        .push(CleartextNetworkPDU::new(
                            pdu.meta().iv_index().ivi(),
                            pdu.meta().network_key_handle().nid(),
                            Ctl::Access,
                            pdu.meta().ttl(),
                            pdu.meta().seq(),
                            pdu.meta().src(),
                            pdu.meta().dst(),
                            &transport_pdu,
                            meta,
                        )?)
                        .map_err(|_| InsufficientBuffer)?;
                } else {
                    let seq_zero = inner.meta().seq().into();
                    let payload = payload.chunks(SEGMENTED_ACCESS_MTU);
                    let seg_n = payload.len() - 1;

                    for (seg_o, segment_m) in payload.enumerate() {
                        let seq = if !is_retransmit && seg_o == 0 {
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

                        let mut transport_pdu = Vec::<_, SEGMENT_LOWER_PDU_SIZE>::new();
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
                                &transport_pdu,
                                meta.clone(),
                            )?)
                            .map_err(|_| InsufficientBuffer)?;
                    }
                }
            }
            UpperPDU::Control(inner) => {
                result
                    .push(self.process_unsegmented_control(sequence, inner, meta)?)
                    .map_err(|_| DriverError::InsufficientSpace)?;
            }
        }
        Ok(result)
    }

    pub fn process_unsegmented_control(
        &self,
        sequence: &Sequence,
        pdu: &UpperControlPDU<ProvisionedStack>,
        meta: NetworkMetadata,
    ) -> Result<CleartextNetworkPDU<ProvisionedStack>, DriverError> {
        let lower_pdu = UnsegmentedLowerControlPDU::<()>::new(pdu.opcode(), pdu.parameters(), ())?;

        let mut transport_pdu = Vec::<_, 16>::new();
        lower_pdu
            .emit(&mut transport_pdu)
            .map_err(|_| DriverError::InsufficientSpace)?;

        Ok(CleartextNetworkPDU::new(
            pdu.meta().iv_index().ivi(),
            pdu.meta().network_key_handle().nid(),
            Ctl::Control,
            pdu.meta().ttl(),
            sequence.next(),
            pdu.meta().src(),
            pdu.meta().dst(),
            &transport_pdu,
            meta,
        )?)
    }
}
