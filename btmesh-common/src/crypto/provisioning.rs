use crate::crypto;
use aes::Aes128;
use ccm::aead::Error;
use cmac::crypto_mac::{InvalidKeyLength, Output};
use cmac::Cmac;

pub fn try_decrypt_data(
    session_key: &[u8],
    session_nonce: &[u8],
    data: &mut [u8],
    mic: &[u8],
) -> Result<(), Error> {
    crypto::aes_ccm_decrypt_detached(session_key, session_nonce, data, mic, None)
}

pub fn encrypt_data(
    session_key: &[u8],
    session_nonce: &[u8],
    data: &mut [u8],
    mic: &mut [u8],
) -> Result<(), Error> {
    crypto::aes_ccm_encrypt_detached(session_key, session_nonce, data, mic, None)
}

pub fn prsk(secret: &[u8], salt: &[u8]) -> Result<Output<Cmac<Aes128>>, InvalidKeyLength> {
    crypto::k1(secret, salt, b"prsk")
}

pub fn prsn(secret: &[u8], salt: &[u8]) -> Result<Output<Cmac<Aes128>>, InvalidKeyLength> {
    crypto::k1(secret, salt, b"prsn")
}

pub fn prck(secret: &[u8], salt: &[u8]) -> Result<Output<Cmac<Aes128>>, InvalidKeyLength> {
    crypto::k1(secret, salt, b"prck")
}

pub fn prdk(secret: &[u8], salt: &[u8]) -> Result<Output<Cmac<Aes128>>, InvalidKeyLength> {
    crypto::k1(secret, salt, b"prdk")
}
