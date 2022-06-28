use crate::crypto;
use crate::crypto::nonce::DeviceNonce;
use crate::mic::TransMic;
use ccm::aead::Error;

pub fn try_decrypt_device_key(
    device_key: [u8; 16],
    nonce: DeviceNonce,
    bytes: &mut [u8],
    mic: &TransMic,
) -> Result<(), Error> {
    crypto::aes_ccm_decrypt_detached(&device_key, &*nonce, bytes, mic.as_ref(), None)
}

pub fn encrypt_device_key(
    device_key: [u8; 16],
    nonce: DeviceNonce,
    bytes: &mut [u8],
    mic: &mut TransMic,
) -> Result<(), Error> {
    crypto::aes_ccm_encrypt_detached(&device_key, &*nonce, bytes, mic.as_mut(), None)
}
