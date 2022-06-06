use crate::Driver;
use btmesh_common::Ctl;
use btmesh_pdu::lower::LowerPDU;
use btmesh_pdu::network::CleartextNetworkPDU;

impl Driver {
    fn process_cleartext_network_pdu(&self, network_pdu: &CleartextNetworkPDU<Self>) {
        let lower_pdu = LowerPDU::parse(network_pdu);
    }

    fn process_cleartext_access_network_pdu(&self, network_pdu: &CleartextNetworkPDU<Self>) {}

    fn process_cleartext_control_network_pdu(&self, network_pdu: &CleartextNetworkPDU<Self>) {}
}
