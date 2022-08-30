use crate::address::LabelUuid;
use crate::crypto::nonce::ApplicationNonce;
use crate::mic::TransMic;
use crate::{crypto, InsufficientBuffer, ParseError};
use ccm::aead::Error;
use cmac::crypto_mac::InvalidKeyLength;
use core::ops::Deref;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use heapless::Vec;

/// Application key identifier.
#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Aid(u8);

impl Aid {
    pub fn parse(akf_aid: u8) -> Result<Option<Self>, ParseError> {
        let akf = akf_aid & 0b01000000 != 0;
        if akf {
            let aid = akf_aid & 0b00111111;
            Ok(Some(Self(aid)))
        } else {
            Ok(None)
        }
    }

    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        let akf_aid = 0b01000000 | self.0 & 0b00111111;
        xmit.push(akf_aid)?;
        Ok(())
    }
}

impl From<Aid> for u8 {
    fn from(aid: Aid) -> Self {
        aid.0
    }
}

impl From<u8> for Aid {
    fn from(val: u8) -> Self {
        Self(val)
    }
}

#[derive(Copy, Clone, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ApplicationKey {
    application_key: [u8; 16],
    aid: Aid,
}

#[cfg(feature = "defmt")]
impl ::defmt::Format for ApplicationKey {
    fn format(&self, fmt: ::defmt::Formatter) {
        ::defmt::write!(
            fmt,
            "0x{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X} {}",
            self.application_key[0],
            self.application_key[1],
            self.application_key[2],
            self.application_key[3],
            self.application_key[4],
            self.application_key[5],
            self.application_key[6],
            self.application_key[7],
            self.application_key[8],
            self.application_key[9],
            self.application_key[10],
            self.application_key[11],
            self.application_key[12],
            self.application_key[13],
            self.application_key[14],
            self.application_key[15],
            self.aid,
        )
    }
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

#[allow(clippy::explicit_auto_deref)]
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

#[allow(clippy::explicit_auto_deref)]
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
