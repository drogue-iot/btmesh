use crate::{Driver, DriverError};
use btmesh_common::crypto::nonce::ApplicationNonce;
use btmesh_pdu::access::AccessMessage;
use btmesh_pdu::upper::access::UpperAccessPDU;
use btmesh_pdu::upper::UpperPDU;

pub struct UpperDriver {}

impl UpperDriver {
    fn process_upper_pdu(&mut self, pdu: UpperPDU<Driver>) {
        match pdu {
            UpperPDU::Access(access) => {
                self.decrypt_access(access);
            }
            UpperPDU::Control(control) => {}
        }
    }

    fn decrypt_access(
        &mut self,
        pdu: UpperAccessPDU<Driver>,
    ) -> Result<AccessMessage<Driver>, DriverError> {
        if let Some(aid) = pdu.meta().aid() {
            let nonce = ApplicationNonce::new(
                pdu.transmic().szmic(),
                pdu.meta().seq(),
                pdu.meta().src(),
                pdu.meta().dst(),
                pdu.meta().iv_index(),
            );
        } else {
        }

        todo!()
    }
}
