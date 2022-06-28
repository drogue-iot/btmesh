use crate::crypto;
use crate::crypto::nonce::DeviceNonce;
use crate::mic::TransMic;
use ccm::aead::Error;
use core::ops::{Deref, DerefMut};

#[derive(Copy, Clone)]
pub struct DeviceKey {
    device_key: [u8; 16],
}

impl Deref for DeviceKey {
    type Target = [u8; 16];

    fn deref(&self) -> &Self::Target {
        &self.device_key
    }
}

pub fn try_decrypt_device_key(
    device_key: &DeviceKey,
    nonce: &DeviceNonce,
    bytes: &mut [u8],
    mic: &TransMic,
) -> Result<(), Error> {
    crypto::aes_ccm_decrypt_detached(&**device_key, &**nonce, bytes, mic.as_ref(), None)
}

pub fn encrypt_device_key(
    device_key: &DeviceKey,
    nonce: &DeviceNonce,
    bytes: &mut [u8],
    mic: &mut TransMic,
) -> Result<(), Error> {
    crypto::aes_ccm_encrypt_detached(&**device_key, &**nonce, bytes, mic.as_mut(), None)
}
