use crate::address::LabelUuid;
use crate::crypto::nonce::{ApplicationNonce, DeviceNonce};
use crate::mic::TransMic;
use crate::{crypto, Aid};
use ccm::aead::Error;
use cmac::crypto_mac::InvalidKeyLength;
use core::ops::Deref;

#[derive(Copy, Clone)]
pub struct ApplicationKey {
    application_key: [u8; 16],
    aid: Aid,
}

impl ApplicationKey {
    pub fn new(application_key: [u8; 16]) -> Result<Self, InvalidKeyLength> {
        let aid = crypto::k4(&application_key)?.into();

        Ok(Self {
            application_key,
            aid,
        })
    }

    pub fn aid(&self) -> Aid {
        self.aid
    }
}

impl Deref for ApplicationKey {
    type Target = [u8; 16];

    fn deref(&self) -> &Self::Target {
        &self.application_key
    }
}

pub fn try_decrypt_application_key(
    application_key: &ApplicationKey,
    nonce: ApplicationNonce,
    bytes: &mut [u8],
    mic: &TransMic,
    label_uuid: Option<LabelUuid>,
) -> Result<(), Error> {
    match label_uuid {
        None => {
            crypto::aes_ccm_decrypt_detached(&**application_key, &*nonce, bytes, mic.as_ref(), None)
        }
        Some(label_uuid) => crypto::aes_ccm_decrypt_detached(
            &**application_key,
            &*nonce,
            bytes,
            mic.as_ref(),
            Some(label_uuid.label_uuid()),
        ),
    }
}

pub fn encrypt_application_key(
    application_key: &ApplicationKey,
    nonce: ApplicationNonce,
    bytes: &mut [u8],
    mic: &mut TransMic,
    label_uuid: Option<LabelUuid>,
) -> Result<(), Error> {
    match label_uuid {
        None => {
            crypto::aes_ccm_encrypt_detached(&**application_key, &*nonce, bytes, mic.as_mut(), None)
        }
        Some(label_uuid) => crypto::aes_ccm_encrypt_detached(
            &**application_key,
            &*nonce,
            bytes,
            mic.as_mut(),
            Some(label_uuid.label_uuid()),
        ),
    }
}
