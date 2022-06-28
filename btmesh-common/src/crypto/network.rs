use crate::crypto::nonce::NetworkNonce;
use crate::crypto::{aes_ccm_decrypt_detached, aes_ccm_encrypt_detached};
use crate::mic::InvalidLength;
use crate::{crypto, ParseError};
use ccm::aead::Error;
use cmac::crypto_mac::InvalidKeyLength;
use core::ops::Deref;
use hash32_derive::Hash32;

/// Network key identifier.
#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Debug, Hash32)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Nid(u8);

impl Nid {
    pub fn new(nid: u8) -> Self {
        Self(nid)
    }

    pub fn parse(nid: u8) -> Result<Nid, ParseError> {
        Ok(Self::new(nid))
    }
}

impl From<Nid> for u8 {
    fn from(nid: Nid) -> Self {
        nid.0
    }
}

impl From<u8> for Nid {
    fn from(val: u8) -> Self {
        Self(val)
    }
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum NetMic {
    Access([u8; 4]),
    Control([u8; 8]),
}

impl NetMic {
    pub fn new_access() -> Self {
        Self::Access([0; 4])
    }

    pub fn new_control() -> Self {
        Self::Control([0; 8])
    }

    pub fn parse(mic: &[u8]) -> Result<Self, InvalidLength> {
        if mic.len() == 4 {
            let mut result = Self::new_access();
            result.as_mut().copy_from_slice(mic);
            Ok(result)
        } else if mic.len() == 8 {
            let mut result = Self::new_control();
            result.as_mut().copy_from_slice(mic);
            Ok(result)
        } else {
            Err(InvalidLength)
        }
    }
}

impl AsRef<[u8]> for NetMic {
    fn as_ref(&self) -> &[u8] {
        match self {
            NetMic::Access(inner) => inner,
            NetMic::Control(inner) => inner,
        }
    }
}

impl AsMut<[u8]> for NetMic {
    fn as_mut(&mut self) -> &mut [u8] {
        match self {
            NetMic::Access(inner) => inner,
            NetMic::Control(inner) => inner,
        }
    }
}

#[derive(Default, Eq, PartialEq, Copy, Clone, Debug)]
pub struct EncryptionKey([u8; 16]);

impl EncryptionKey {
    pub fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

impl Deref for EncryptionKey {
    type Target = [u8; 16];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Default, Eq, PartialEq, Copy, Clone, Debug)]
pub struct PrivacyKey([u8; 16]);

impl PrivacyKey {
    pub fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

impl Deref for PrivacyKey {
    type Target = [u8; 16];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Copy, Clone)]
pub struct NetworkKey {
    privacy_key: PrivacyKey,
    encryption_key: EncryptionKey,
    nid: Nid,
}

impl NetworkKey {
    pub fn new(network_key: [u8; 16]) -> Result<Self, InvalidKeyLength> {
        let (nid, encryption_key, privacy_key) = crypto::k2(&network_key, &[0x00])?;

        Ok(Self {
            privacy_key: PrivacyKey(privacy_key),
            encryption_key: EncryptionKey(encryption_key),
            nid: Nid::new(nid),
        })
    }

    pub fn privacy_key(&self) -> PrivacyKey {
        self.privacy_key
    }

    pub fn encryption_key(&self) -> EncryptionKey {
        self.encryption_key
    }

    pub fn nid(&self) -> Nid {
        self.nid
    }
}

pub fn try_decrypt_network(
    network_key: &NetworkKey,
    nonce: &NetworkNonce,
    payload: &mut [u8],
    mic: &NetMic,
) -> Result<(), Error> {
    aes_ccm_decrypt_detached(
        &*network_key.encryption_key,
        &**nonce,
        payload,
        mic.as_ref(),
        None,
    )
}

pub fn encrypt_network(
    network_key: &NetworkKey,
    nonce: &NetworkNonce,
    payload: &mut [u8],
    mic: &mut NetMic,
) -> Result<(), Error> {
    aes_ccm_encrypt_detached(
        &*network_key.encryption_key,
        &**nonce,
        payload,
        mic.as_mut(),
        None,
    )
}

#[cfg(test)]
mod test {
    use crate::crypto::network::{EncryptionKey, NetworkKey, Nid, PrivacyKey};

    #[test]
    fn network_key_derivation() {
        // 8.2.2 Encryption and privacy keys (Master)
        let network_key = NetworkKey::new([
            0x7d, 0xd7, 0x36, 0x4c, 0xd8, 0x42, 0xad, 0x18, 0xc1, 0x7c, 0x2b, 0x82, 0x0c, 0x84,
            0xc3, 0xd6,
        ])
        .unwrap();

        let encryption_key = EncryptionKey::new([
            0x09, 0x53, 0xfa, 0x93, 0xe7, 0xca, 0xac, 0x96, 0x38, 0xf5, 0x88, 0x20, 0x22, 0x0a,
            0x39, 0x8e,
        ]);

        let privacy_key = PrivacyKey::new([
            0x8b, 0x84, 0xee, 0xde, 0xc1, 0x00, 0x06, 0x7d, 0x67, 0x09, 0x71, 0xdd, 0x2a, 0xa7,
            0x00, 0xcf,
        ]);

        assert_eq!(Nid::new(0x68), network_key.nid());
        assert_eq!(privacy_key, network_key.privacy_key());
        assert_eq!(encryption_key, network_key.encryption_key());
    }
}
