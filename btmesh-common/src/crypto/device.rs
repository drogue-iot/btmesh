use crate::crypto;
use crate::crypto::nonce::DeviceNonce;
use crate::mic::TransMic;
use ccm::aead::Error;
use core::ops::Deref;

#[derive(Copy, Clone)]
pub struct DeviceKey {
    device_key: [u8; 16],
}

impl DeviceKey {
    pub fn new(device_key: [u8; 16]) -> Self {
        Self { device_key }
    }
}

impl Deref for DeviceKey {
    type Target = [u8; 16];

    fn deref(&self) -> &Self::Target {
        &self.device_key
    }
}

impl TryFrom<&[u8]> for DeviceKey {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(DeviceKey::new(value.try_into().map_err(|_| Error)?))
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
