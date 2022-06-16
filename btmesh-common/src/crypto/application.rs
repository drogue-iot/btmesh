use crate::crypto;
use crate::crypto::nonce::{ApplicationNonce, DeviceNonce};
use ccm::aead::Error;

pub fn try_decrypt_application_key(
    application_key: [u8; 16],
    nonce: ApplicationNonce,
    bytes: &mut [u8],
    mic: &[u8],
    additional_data: Option<&[u8]>,
) -> Result<(), Error> {
    crypto::aes_ccm_decrypt_detached(&application_key, &*nonce, bytes, mic, additional_data)
}

pub fn try_decrypt_device_key(
    device_key: [u8; 16],
    nonce: DeviceNonce,
    bytes: &mut [u8],
    mic: &[u8],
) -> Result<(), Error> {
    crypto::aes_ccm_decrypt_detached(&device_key, &*nonce, bytes, mic, None)
}
