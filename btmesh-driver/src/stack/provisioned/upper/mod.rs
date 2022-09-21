use crate::stack::provisioned::sequence::Sequence;
use crate::stack::provisioned::system::{AccessMetadata, ControlMetadata, UpperMetadata};
use crate::stack::provisioned::{DriverError, ProvisionedStack};
use crate::Secrets;
use btmesh_common::address::{Address, LabelUuid};
use btmesh_common::crypto;
use btmesh_common::crypto::nonce::{ApplicationNonce, DeviceNonce};
use btmesh_common::mic::{SzMic, TransMic};
use btmesh_device::KeyHandle;
use btmesh_pdu::provisioned::access::AccessMessage;
use btmesh_pdu::provisioned::control::ControlMessage;
use btmesh_pdu::provisioned::upper::access::UpperAccessPDU;
use btmesh_pdu::provisioned::upper::UpperPDU;
use btmesh_pdu::provisioned::Message;
use core::ops::ControlFlow;
use heapless::Vec;

#[derive(Default)]
pub struct UpperDriver<const N: usize = 20> {
    label_uuids: Vec<Option<LabelUuid>, N>,
}

impl UpperDriver {}

impl ProvisionedStack {
    fn add_label_uuid(&mut self, label_uuid: LabelUuid) -> Result<(), DriverError> {
        if let Some(empty_slot) = self
            .upper
            .label_uuids
            .iter_mut()
            .find(|e| matches!(e, None))
        {
            empty_slot.replace(label_uuid);
            Ok(())
        } else {
            Err(DriverError::InsufficientSpace)
        }
    }

    fn remove_label_uuid(&mut self, label_uuid: LabelUuid) {
        self.upper
            .label_uuids
            .iter_mut()
            .filter(|e| {
                if let Some(inner) = e {
                    *inner == label_uuid
                } else {
                    false
                }
            })
            .for_each(|slot| {
                slot.take();
            })
    }

    pub fn process_inbound_upper_pdu(
        &mut self,
        secrets: &Secrets,
        pdu: &mut UpperPDU<ProvisionedStack>,
    ) -> Result<Message<ProvisionedStack>, DriverError> {
        self.apply_label_uuids(pdu)?;
        match pdu {
            UpperPDU::Access(access) => Ok(self.decrypt_access(secrets, access)?.into()),
            UpperPDU::Control(control) => Ok(ControlMessage::new(
                control.opcode(),
                control.parameters(),
                ControlMetadata::from_upper_control_pdu(control),
            )?
            .into()),
        }
    }

    pub fn process_outbound_message(
        &mut self,
        secrets: &Secrets,
        sequence: &Sequence,
        message: &Message<ProvisionedStack>,
    ) -> Result<UpperPDU<ProvisionedStack>, DriverError> {
        match message {
            Message::Access(access) => Ok(self.encrypt_access(secrets, sequence, access)?.into()),
            Message::Control(_control) => {
                todo!()
            }
        }
    }

    /// Apply potential candidate label-uuids if the destination of the PDU
    /// is a virtual-address.
    fn apply_label_uuids(&self, pdu: &mut UpperPDU<ProvisionedStack>) -> Result<(), DriverError> {
        if let Address::Virtual(virtual_address) = pdu.meta().dst() {
            let result = self.upper.label_uuids.iter().try_for_each(|slot| {
                if let Some(label_uuid) = slot {
                    if label_uuid.virtual_address() == virtual_address {
                        if let Err(err) = pdu.meta_mut().add_label_uuid(*label_uuid) {
                            return ControlFlow::Break(err);
                        }
                    }
                }

                ControlFlow::Continue(())
            });

            match result {
                ControlFlow::Break(err) => Err(err),
                _ => Ok(()),
            }
        } else {
            Ok(())
        }
    }

    fn encrypt_access(
        &mut self,
        secrets: &Secrets,
        sequence: &Sequence,
        message: &AccessMessage<ProvisionedStack>,
    ) -> Result<UpperAccessPDU<ProvisionedStack>, DriverError> {
        let seq_zero = sequence.next();

        let mut payload = Vec::<u8, 379>::new();
        message.emit(&mut payload)?;

        match message.meta().key_handle() {
            KeyHandle::Device => {
                let nonce = DeviceNonce::new(
                    SzMic::Bit32,
                    seq_zero,
                    message.meta().src(),
                    message.meta().dst(),
                    message.meta().iv_index(),
                );

                let device_key = secrets.device_key();

                let mut transmic = TransMic::new32();

                crypto::device::encrypt_device_key(
                    &device_key,
                    &nonce,
                    &mut payload,
                    &mut transmic,
                )
                .map_err(|_| DriverError::CryptoError)?;

                Ok(UpperAccessPDU::new(
                    &payload,
                    transmic,
                    UpperMetadata::from_access_message(message, seq_zero),
                )?)
            }
            KeyHandle::Network(_) => {
                todo!("network key handle")
            }
            KeyHandle::Application(key_handle) => {
                let nonce = ApplicationNonce::new(
                    SzMic::Bit32,
                    seq_zero,
                    message.meta().src(),
                    message.meta().dst(),
                    message.meta().iv_index(),
                );

                let application_key = secrets.application_key(key_handle)?;

                let mut transmic = TransMic::new32();

                crypto::application::encrypt_application_key(
                    &application_key,
                    nonce,
                    &mut payload,
                    &mut transmic,
                    message.meta().label_uuid(),
                )
                .map_err(|_| DriverError::CryptoError)?;

                Ok(UpperAccessPDU::new(
                    &payload,
                    transmic,
                    UpperMetadata::from_access_message(message, seq_zero),
                )?)
            }
        }
    }

    fn decrypt_access(
        &mut self,
        secrets: &Secrets,
        pdu: &UpperAccessPDU<ProvisionedStack>,
    ) -> Result<AccessMessage<ProvisionedStack>, DriverError> {
        if let Some(aid) = pdu.meta().aid() {
            // akf=true and an AID was provided.
            let nonce = ApplicationNonce::new(
                pdu.transmic().szmic(),
                pdu.meta().seq(),
                pdu.meta().src(),
                pdu.meta().dst(),
                pdu.meta().iv_index(),
            );

            let mut bytes: Vec<_, 380> = Vec::new();
            let mut decrypt_result = None;

            'outer: for application_key_handle in secrets.application_keys_by_aid(aid) {
                let application_key = secrets.application_key(application_key_handle)?;
                if pdu.meta().label_uuids().is_empty() {
                    bytes.clear();
                    bytes
                        .extend_from_slice(pdu.payload())
                        .map_err(|_| DriverError::InsufficientSpace)?;
                    if crypto::application::try_decrypt_application_key(
                        &application_key,
                        nonce,
                        &mut bytes,
                        &pdu.transmic(),
                        None,
                    )
                    .is_ok()
                    {
                        decrypt_result.replace((application_key_handle, None, bytes));
                        break 'outer;
                    }
                } else {
                    // try each label-uuid until success.
                    // while this is two nested loops, the probability of
                    // more than a single execution is exceedingly low,
                    // but never zero.
                    for label_uuid in pdu.meta().label_uuids() {
                        bytes.clear();
                        bytes
                            .extend_from_slice(pdu.payload())
                            .map_err(|_| DriverError::InsufficientSpace)?;
                        if crypto::application::try_decrypt_application_key(
                            &application_key,
                            nonce,
                            &mut bytes,
                            &pdu.transmic(),
                            Some(*label_uuid),
                        )
                        .is_ok()
                        {
                            decrypt_result.replace((
                                application_key_handle,
                                Some(*label_uuid),
                                bytes,
                            ));
                            break 'outer;
                        }
                    }
                }
            }

            if let Some((application_key_handle, label_uuid, bytes)) = decrypt_result {
                return Ok(AccessMessage::parse(
                    &bytes,
                    AccessMetadata::from_upper_access_pdu(
                        KeyHandle::Application(application_key_handle),
                        label_uuid,
                        pdu,
                    ),
                )?);
            }
        } else {
            let seq = if let Some(seq_auth) = pdu.meta().seq_auth() {
                seq_auth.into()
            } else {
                pdu.meta().seq()
            };

            let nonce = DeviceNonce::new(
                pdu.transmic().szmic(),
                seq,
                pdu.meta().src(),
                pdu.meta().dst(),
                pdu.meta().iv_index(),
            );

            let device_key = secrets.device_key();

            let mut bytes = Vec::<_, 380>::from_slice(pdu.payload())
                .map_err(|_| DriverError::InsufficientSpace)?;

            if crypto::device::try_decrypt_device_key(
                &device_key,
                &nonce,
                &mut bytes,
                &pdu.transmic(),
            )
            .is_ok()
            {
                return Ok(AccessMessage::parse(
                    &bytes,
                    AccessMetadata::from_upper_access_pdu(KeyHandle::Device, None, pdu),
                )?);
            }
        }

        Err(DriverError::InvalidPDU)
    }
}
