use crate::address::LabelUuid;
use crate::crypto;
use crate::crypto::nonce::{ApplicationNonce, DeviceNonce};
use ccm::aead::Error;
use crate::mic::TransMic;

pub fn try_decrypt_application_key(
    application_key: [u8; 16],
    nonce: ApplicationNonce,
    bytes: &mut [u8],
    mic: &TransMic,
    label_uuid: Option<LabelUuid>,
) -> Result<(), Error> {
    match label_uuid {
        None => crypto::aes_ccm_decrypt_detached(&application_key, &*nonce, bytes, mic.as_ref(), None),
        Some(label_uuid) => crypto::aes_ccm_decrypt_detached(
            &application_key,
            &*nonce,
            bytes,
            mic.as_ref(),
            Some(label_uuid.label_uuid()),
        ),
    }
}

pub fn encrypt_application_key(
    application_key: [u8; 16],
    nonce: ApplicationNonce,
    bytes: &mut [u8],
    mic: &mut TransMic,
    label_uuid: Option<LabelUuid>,
) -> Result<(), Error> {
    match label_uuid {
        None => {
            crypto::aes_ccm_encrypt_detached(&application_key, &*nonce, bytes, mic.as_mut(), None)
        }
        Some(label_uuid) => {
            crypto::aes_ccm_encrypt_detached(&application_key, &*nonce, bytes, mic.as_mut(), Some(label_uuid.label_uuid()))
        }
    }

}
