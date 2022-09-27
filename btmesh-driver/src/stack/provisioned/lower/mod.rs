mod inbound_segmentation;
mod outbound_segmentation;

use crate::stack::provisioned::lower::inbound_segmentation::InboundSegmentation;
use crate::stack::provisioned::lower::outbound_segmentation::OutboundSegmentation;
use crate::stack::provisioned::sequence::Sequence;
use crate::stack::provisioned::system::{LowerMetadata, NetworkMetadata, UpperMetadata};
use crate::stack::provisioned::ProvisionedStack;
use crate::storage::provisioned::subscriptions::Subscriptions;
use crate::{DriverError, Secrets, Watchdog};
use btmesh_common::address::UnicastAddress;
use btmesh_common::mic::SzMic;
use btmesh_common::{InsufficientBuffer, SeqZero};
use btmesh_pdu::provisioned::lower::{BlockAck, LowerPDU, UnsegmentedLowerPDU};
use btmesh_pdu::provisioned::network::{CleartextNetworkPDU, NetworkPDU};
use btmesh_pdu::provisioned::upper::access::UpperAccessPDU;
use btmesh_pdu::provisioned::upper::control::{ControlOpcode, UpperControlPDU};
use btmesh_pdu::provisioned::upper::UpperPDU;
use heapless::Vec;

#[derive(Default)]
pub struct LowerDriver {
    inbound_segmentation: InboundSegmentation,
    outbound_segmentation: OutboundSegmentation,
}

impl LowerDriver {
    pub fn expire_inbound(
        &mut self,
        seq_zero: &SeqZero,
        watchdog: &Watchdog,
    ) -> Option<(BlockAck, UpperMetadata)> {
        self.inbound_segmentation.expire_inbound(seq_zero, watchdog)
    }
}

impl ProvisionedStack {
    /// Process a *cleartext* `NetworkPDU`, through hidden `LowerPDU`s, accommodating segmentation & reassembly,
    /// to produce an `UpperPDU` if sufficiently unsegmented or re-assembled.
    #[allow(clippy::type_complexity)]
    pub fn process_inbound_cleartext_network_pdu(
        &mut self,
        network_pdu: &CleartextNetworkPDU<ProvisionedStack>,
        watchdog: &Watchdog,
        subscriptions: &Subscriptions,
    ) -> Result<
        (
            Option<(BlockAck, UpperMetadata)>,
            Option<UpperPDU<ProvisionedStack>>,
        ),
        DriverError,
    > {
        let lower_pdu = LowerPDU::parse(network_pdu, LowerMetadata::from_network_pdu(network_pdu))?;

        match &lower_pdu {
            LowerPDU::Unsegmented(inner) => match inner {
                UnsegmentedLowerPDU::Access(access_pdu) => Ok((
                    None,
                    Some(
                        UpperAccessPDU::parse(
                            access_pdu.upper_pdu(),
                            &SzMic::Bit32,
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
                if self.device_info().is_non_local_unicast(inner.meta().dst()) {
                    // unicast, but not to us.
                    Ok((None, None))
                } else {
                    let dst = inner.meta().dst();
                    // For group addresses, and we have subscriptions for the destination, process it.
                    if dst.is_unicast() || subscriptions.matches(dst) {
                        let result = self.lower.inbound_segmentation.process(inner, watchdog)?;
                        // We only ack local addresses
                        Ok((Some((result.block_ack, result.meta)), result.upper_pdu))
                    } else {
                        // We don't process segments not related to us. Relay behavior is done at a layer above us.
                        Ok((None, None))
                    }
                }
            }
        }
    }

    pub fn process_outbound_block_ack(
        &mut self,
        secrets: &Secrets,
        sequence: &Sequence,
        block_ack: BlockAck,
        meta: &UpperMetadata,
        src: &UnicastAddress,
    ) -> Result<Option<NetworkPDU>, DriverError> {
        let pdu = &block_ack_to_upper_pdu(sequence, block_ack, meta, src)?;
        let meta = NetworkMetadata::from_upper_control_pdu(pdu);

        let network_pdu = self
            .lower
            .outbound_segmentation
            .process_unsegmented_control(sequence, pdu, meta)?;

        Ok(Some(self.encrypt_network_pdu(secrets, &network_pdu)?))
    }

    pub fn process_outbound_upper_pdu<const N: usize>(
        &mut self,
        sequence: &Sequence,
        upper_pdu: &UpperPDU<ProvisionedStack>,
        is_retransmit: bool,
    ) -> Result<Vec<CleartextNetworkPDU<ProvisionedStack>, N>, DriverError> {
        self.lower
            .outbound_segmentation
            .process(sequence, upper_pdu, is_retransmit)
    }
}

fn block_ack_to_upper_pdu(
    sequence: &Sequence,
    block_ack: BlockAck,
    meta: &UpperMetadata,
    src: &UnicastAddress,
) -> Result<UpperControlPDU<ProvisionedStack>, InsufficientBuffer> {
    let mut parameters = [0; 6];

    let seq_zero = ((block_ack.seq_zero().value() & 0b0111111111111111) << 2).to_be_bytes();
    parameters[0] = seq_zero[0];
    parameters[1] = seq_zero[1];

    let block_ack = block_ack.value().to_be_bytes();
    parameters[2] = block_ack[0];
    parameters[3] = block_ack[1];
    parameters[4] = block_ack[2];
    parameters[5] = block_ack[3];

    let meta = UpperMetadata {
        network_key_handle: meta.network_key_handle(),
        iv_index: meta.iv_index(),
        local_element_index: None,
        akf_aid: meta.aid(),
        seq: sequence.next(),
        src: *src,
        dst: meta.src().into(),
        ttl: meta.ttl(),
        label_uuids: Vec::from_slice(meta.label_uuids())?,
        seq_auth: None,
        replay_seq: None,
    };

    UpperControlPDU::new(ControlOpcode::SegmentAcknowledgement, &parameters, meta)
}
