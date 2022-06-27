use ccm::aead::Error;
use crate::crypto;
use crate::crypto::nonce::DeviceNonce;

pub fn try_decrypt_device_key(
    device_key: [u8; 16],
    nonce: DeviceNonce,
    bytes: &mut [u8],
    mic: &[u8],
) -> Result<(), Error> {
    crypto::aes_ccm_decrypt_detached(&device_key, &*nonce, bytes, mic, None)
}

pub fn encrypt_device_key(
    device_key: [u8; 16],
    nonce: DeviceNonce,
    bytes: &mut [u8],
    mic: &mut [u8],
) -> Result<(), Error> {
    crypto::aes_ccm_encrypt_detached(&device_key, &*nonce, bytes, mic, None)
}