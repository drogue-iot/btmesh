use core::ops::ControlFlow;
use crate::{Driver, DriverError};
use btmesh_common::address::{Address, LabelUuid};
use btmesh_common::crypto::nonce::ApplicationNonce;
use btmesh_pdu::access::AccessMessage;
use btmesh_pdu::upper::access::UpperAccessPDU;
use btmesh_pdu::upper::UpperPDU;
use heapless::Vec;

#[derive(Default)]
pub struct UpperDriver<const N: usize = 20> {
    label_uuids: Vec<Option<LabelUuid>, N>,
}

impl UpperDriver {
    fn add_label_uuid(&mut self, label_uuid: LabelUuid) -> Result<(), DriverError> {
        if let Some(empty_slot) = self.label_uuids.iter_mut().find(|e| matches!(e, None)) {
            empty_slot.replace(label_uuid);
            Ok(())
        } else {
            Err(DriverError::InsufficientSpace)
        }
    }

    fn remove_label_uuid(&mut self, label_uuid: LabelUuid) {
        self.label_uuids
            .iter_mut()
            .filter(|e| matches!(e, Some(label_uuid)))
            .for_each(|slot| {
                slot.take();
            })
    }

    fn process_upper_pdu(&mut self, mut pdu: UpperPDU<Driver>) -> Result<(), DriverError>{
        self.apply_label_uuids(&mut pdu)?;
        match pdu {
            UpperPDU::Access(access) => {
                self.decrypt_access(access);
            }
            UpperPDU::Control(control) => {}
        }

        Ok(())
    }


    /// Apply potential candidate label-uuids if the destination of the PDU
    /// is a virtual-address.
    fn apply_label_uuids(&self, pdu: &mut UpperPDU<Driver>) -> Result<(), DriverError>{
        if let Address::Virtual(virtual_address) = pdu.meta().dst() {
            let result = self.label_uuids.iter().try_for_each(|slot| {
                if let Some(label_uuid) = slot {
                    if label_uuid.virtual_address() == virtual_address {
                        if let Err(err) = pdu.meta_mut().add_label_uuid( *label_uuid ) {
                            return ControlFlow::Break(err);
                        }
                    }
                }

                ControlFlow::Continue(())
            });

            match result {
                ControlFlow::Break(err) =>  Err(err),
                _ => Ok(()),
            }
        } else {
            Ok(())
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

            if pdu.meta().label_uuids().is_empty() {

            } else {

            }
        } else {
        }

        todo!()
    }
}
