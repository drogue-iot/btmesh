use self::inbound::InboundSegments;
use self::outbound::OutboundSegments;
use crate::DriverError;
use btmesh_bearer::BearerError;
use btmesh_pdu::provisioning::generic::GenericProvisioningPDU;
use btmesh_pdu::provisioning::ProvisioningPDU;
use core::cell::RefCell;

mod inbound;
pub(crate) mod outbound;

pub struct Segmentation {
    inbound_segments: RefCell<Option<InboundSegments>>,
}

impl Default for Segmentation {
    fn default() -> Self {
        Self {
            inbound_segments: RefCell::new(None),
        }
    }
}

impl Segmentation {
    pub fn process_inbound(
        &self,
        pdu: &GenericProvisioningPDU,
    ) -> Result<Option<ProvisioningPDU>, DriverError> {
        match pdu {
            GenericProvisioningPDU::TransactionStart(transaction_start) => {
                if transaction_start.seg_n == 0 {
                    let pdu = ProvisioningPDU::parse(&transaction_start.data)?;
                    Ok(Some(pdu))
                } else {
                    let mut borrowed_segments = self.inbound_segments.borrow_mut();
                    if borrowed_segments.is_none() {
                        if let Ok(segments) =
                            InboundSegments::new(transaction_start.seg_n, &transaction_start.data)
                        {
                            borrowed_segments.replace(segments);
                        }
                    }
                    Ok(None)
                }
            }
            GenericProvisioningPDU::TransactionContinuation(transaction_continuation) => {
                let mut borrowed_segments = self.inbound_segments.borrow_mut();
                if let Some(segments) = &mut *borrowed_segments {
                    if let Ok(Some(provisioning_pdu)) = segments.receive(
                        transaction_continuation.segment_index,
                        &transaction_continuation.data,
                    ) {
                        borrowed_segments.take();
                        Ok(Some(provisioning_pdu))
                    } else {
                        Ok(None)
                    }
                } else {
                    // wait to see the TransactionStart again
                    Ok(None)
                }
            }
            GenericProvisioningPDU::TransactionAck => {
                // Not applicable for this role
                Ok(None)
            }
            _ => {
                // Shouldn't get here, but whatevs.
                Ok(None)
            }
        }
    }

    pub fn process_outbound(&self, pdu: &ProvisioningPDU) -> Result<OutboundSegments, BearerError> {
        OutboundSegments::new(pdu)
    }
}
