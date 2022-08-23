use crate::crypto;
use crate::crypto::nonce::DeviceNonce;
use crate::mic::TransMic;
use ccm::aead::Error;
use core::ops::Deref;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DeviceKey {
    device_key: [u8; 16],
}

#[cfg(feature = "defmt")]
impl ::defmt::Format for DeviceKey {
    fn format(&self, fmt: ::defmt::Formatter) {
        ::defmt::write!(
            fmt,
            "0x{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            self.device_key[0],
            self.device_key[1],
            self.device_key[2],
            self.device_key[3],
            self.device_key[4],
            self.device_key[5],
            self.device_key[6],
            self.device_key[7],
            self.device_key[8],
            self.device_key[9],
            self.device_key[10],
            self.device_key[11],
            self.device_key[12],
            self.device_key[13],
            self.device_key[14],
            self.device_key[15],
        )
    }
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
    type Error = crate::ParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(DeviceKey::new(value.try_into()?))
    }
}

#[allow(clippy::explicit_auto_deref)]
pub fn try_decrypt_device_key(
    device_key: &DeviceKey,
    nonce: &DeviceNonce,
    bytes: &mut [u8],
    mic: &TransMic,
) -> Result<(), Error> {
    crypto::aes_ccm_decrypt_detached(&**device_key, &**nonce, bytes, mic.as_ref(), None)
}

#[allow(clippy::explicit_auto_deref)]
pub fn encrypt_device_key(
    device_key: &DeviceKey,
    nonce: &DeviceNonce,
    bytes: &mut [u8],
    mic: &mut TransMic,
) -> Result<(), Error> {
    crypto::aes_ccm_encrypt_detached(&**device_key, &**nonce, bytes, mic.as_mut(), None)
}
