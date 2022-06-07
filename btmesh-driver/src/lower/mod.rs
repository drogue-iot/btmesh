mod inbound_segmentation;

use crate::{Driver, DriverError};
use btmesh_pdu::lower::LowerPDU;
use btmesh_pdu::network::CleartextNetworkPDU;

impl Driver {
    fn process_cleartext_network_pdu(
        &self,
        network_pdu: &CleartextNetworkPDU<Self>,
    ) -> Result<LowerPDU<Self>, DriverError> {
        Ok(LowerPDU::parse(network_pdu)?)
    }
}
