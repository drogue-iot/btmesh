use crate::crypto::network::PrivacyKey;
use crate::IvIndex;
use aes::cipher::Block;
use aes::{Aes128, BlockEncrypt, NewBlockCipher};
use ccm::aead::generic_array::GenericArray;
use ccm::aead::{AeadInPlace, Error, NewAead};
use ccm::consts::U13;
use ccm::consts::U4;
use ccm::consts::U8;
use ccm::Ccm;
use cmac::crypto_mac::{InvalidKeyLength, Key, Output};
use cmac::{Cmac, Mac, NewMac};
use core::convert::TryInto;
use heapless::Vec;

pub mod application;
pub mod device;
pub mod network;
pub mod nonce;
pub mod provisioning;

pub const ZERO: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

pub fn s1(input: &[u8]) -> Result<Output<Cmac<Aes128>>, InvalidKeyLength> {
    aes_cmac(&ZERO, input)
}

pub fn aes_cmac(key: &[u8], input: &[u8]) -> Result<Output<Cmac<Aes128>>, InvalidKeyLength> {
    let key = Key::<Cmac<Aes128>>::from_slice(key);
    let mut mac = Cmac::<Aes128>::new(key);
    mac.update(input);
    Ok(mac.finalize())
}

pub fn k1(n: &[u8], salt: &[u8], p: &[u8]) -> Result<Output<Cmac<Aes128>>, InvalidKeyLength> {
    let t = aes_cmac(salt, n)?;
    let t = t.into_bytes();
    aes_cmac(&t, p)
}

pub fn k2(n: &[u8], p: &[u8]) -> Result<(u8, [u8; 16], [u8; 16]), InvalidKeyLength> {
    let salt = s1(b"smk2")?;
    let t = &aes_cmac(&salt.into_bytes(), n)?.into_bytes();

    let mut input: Vec<u8, 64> = Vec::new();
    input.extend_from_slice(p).map_err(|_| InvalidKeyLength)?;
    input.push(0x01).map_err(|_| InvalidKeyLength)?;
    let t1 = &aes_cmac(t, &input)?.into_bytes();

    let nid = t1[15] & 0x7F;

    input.truncate(0);
    input.extend_from_slice(t1).map_err(|_| InvalidKeyLength)?;
    input.extend_from_slice(p).map_err(|_| InvalidKeyLength)?;
    input.push(0x02).map_err(|_| InvalidKeyLength)?;

    let t2 = aes_cmac(t, &input)?.into_bytes();

    let encryption_key = t2;

    input.truncate(0);
    input.extend_from_slice(&t2).map_err(|_| InvalidKeyLength)?;
    input.extend_from_slice(p).map_err(|_| InvalidKeyLength)?;
    input.push(0x03).map_err(|_| InvalidKeyLength)?;

    let t3 = aes_cmac(t, &input)?.into_bytes();
    let privacy_key = t3;

    Ok((
        nid,
        encryption_key.try_into().map_err(|_| InvalidKeyLength)?,
        privacy_key.try_into().map_err(|_| InvalidKeyLength)?,
    ))
}

pub fn e(key: &PrivacyKey, mut data: [u8; 16]) -> Result<[u8; 16], InvalidKeyLength> {
    let key = GenericArray::<u8, <Aes128 as NewBlockCipher>::KeySize>::from_slice(key.as_ref());
    let cipher = Aes128::new_from_slice(key).map_err(|_| InvalidKeyLength)?;

    let cipher_block = Block::<Aes128>::from_mut_slice(&mut data);
    cipher.encrypt_block(cipher_block);
    Ok(data)
}

type AesCcm32bitMac = Ccm<Aes128, U4, U13>;
type AesCcm64bitMac = Ccm<Aes128, U8, U13>;

pub(crate) fn aes_ccm_decrypt_detached(
    key: &[u8],
    nonce: &[u8],
    data: &mut [u8],
    mic: &[u8],
    additional_data: Option<&[u8]>,
) -> Result<(), Error> {
    let key = GenericArray::<u8, <Aes128 as NewBlockCipher>::KeySize>::from_slice(key);
    match mic.len() {
        4 => {
            let ccm = AesCcm32bitMac::new(key);
            ccm.decrypt_in_place_detached(
                nonce.into(),
                additional_data.unwrap_or(&[]),
                data,
                mic.into(),
            )
        }
        8 => {
            let ccm = AesCcm64bitMac::new(key);
            ccm.decrypt_in_place_detached(
                nonce.into(),
                additional_data.unwrap_or(&[]),
                data,
                mic.into(),
            )
        }
        _ => Err(Error),
    }
}

pub(crate) fn aes_ccm_encrypt_detached(
    key: &[u8],
    nonce: &[u8],
    data: &mut [u8],
    mic: &mut [u8],
    additional_data: Option<&[u8]>,
) -> Result<(), Error> {
    let key = GenericArray::<u8, <Aes128 as NewBlockCipher>::KeySize>::from_slice(key);
    match mic.len() {
        4 => {
            let ccm = AesCcm32bitMac::new(key);
            let tag =
                ccm.encrypt_in_place_detached(nonce.into(), additional_data.unwrap_or(&[]), data)?;
            for (i, b) in mic.iter_mut().enumerate() {
                *b = tag[i];
            }
            Ok(())
        }
        8 => {
            let ccm = AesCcm64bitMac::new(key);
            let tag =
                ccm.encrypt_in_place_detached(nonce.into(), additional_data.unwrap_or(&[]), data)?;
            for (i, b) in mic.iter_mut().enumerate() {
                *b = tag[i];
            }
            Ok(())
        }
        _ => Err(Error),
    }
}

const ID64: [u8; 5] = [b'i', b'd', b'6', b'4', 0x01];

pub fn k3(n: &[u8; 16]) -> Result<[u8; 8], InvalidKeyLength> {
    let salt = s1(b"smk3")?;
    let t = aes_cmac(&salt.into_bytes(), n)?;
    let result = aes_cmac(&t.into_bytes(), &ID64)?.into_bytes();
    if result.len() < 8 {
        Err(InvalidKeyLength)
    } else {
        Ok(result[result.len() - 8..]
            .try_into()
            .map_err(|_| InvalidKeyLength)?)
    }
}

const ID6: [u8; 4] = [b'i', b'd', b'6', 0x01];

pub fn k4(n: &[u8]) -> Result<u8, InvalidKeyLength> {
    let salt = s1(b"smk4")?;
    let t = aes_cmac(&salt.into_bytes(), n)?;
    let result = aes_cmac(&t.into_bytes(), &ID6)?.into_bytes();
    if let Some(last) = result.last() {
        Ok(last & 0b00111111)
    } else {
        Err(InvalidKeyLength)
    }
}

pub fn privacy_plaintext(iv_index: IvIndex, encrypted_and_mic: &[u8]) -> [u8; 16] {
    let mut privacy_plaintext = [0; 16];

    // 0x0000000000
    privacy_plaintext[0] = 0;
    privacy_plaintext[1] = 0;
    privacy_plaintext[2] = 0;
    privacy_plaintext[3] = 0;
    privacy_plaintext[4] = 0;

    // IV index
    let iv_index_bytes = iv_index.to_be_bytes();
    privacy_plaintext[5] = iv_index_bytes[0];
    privacy_plaintext[6] = iv_index_bytes[1];
    privacy_plaintext[7] = iv_index_bytes[2];
    privacy_plaintext[8] = iv_index_bytes[3];

    // Privacy Random
    privacy_plaintext[9] = encrypted_and_mic[0];
    privacy_plaintext[10] = encrypted_and_mic[1];
    privacy_plaintext[11] = encrypted_and_mic[2];
    privacy_plaintext[12] = encrypted_and_mic[3];
    privacy_plaintext[13] = encrypted_and_mic[4];
    privacy_plaintext[14] = encrypted_and_mic[5];
    privacy_plaintext[15] = encrypted_and_mic[6];

    privacy_plaintext
}

pub fn pecb_xor(pecb: [u8; 16], bytes: [u8; 6]) -> [u8; 6] {
    let mut output = [0; 6];
    for (i, b) in bytes.iter().enumerate() {
        output[i] = pecb[i] ^ *b;
    }
    output
}

#[cfg(test)]
mod tests {

    #[test]
    pub fn s1() {
        // 8.1.1 s1 SALT generation function
        let result = super::s1(b"test").unwrap();

        assert_eq!(
            &*result.into_bytes(),
            [
                0xb7, 0x3c, 0xef, 0xbd, 0x64, 0x1e, 0xf2, 0xea, 0x59, 0x8c, 0x2b, 0x6e, 0xfb, 0x62,
                0xf7, 0x9c
            ]
        );
    }

    #[test]
    pub fn k1() {
        // 8.1.2 k1 function
        let n = [
            0x32, 0x16, 0xd1, 0x50, 0x98, 0x84, 0xb5, 0x33, 0x24, 0x85, 0x41, 0x79, 0x2b, 0x87,
            0x7f, 0x98,
        ];

        let salt = [
            0x2b, 0xa1, 0x4f, 0xfa, 0x0d, 0xf8, 0x4a, 0x28, 0x31, 0x93, 0x8d, 0x57, 0xd2, 0x76,
            0xca, 0xb4,
        ];

        let p = [
            0x5a, 0x09, 0xd6, 0x07, 0x97, 0xee, 0xb4, 0x47, 0x8a, 0xad, 0xa5, 0x9d, 0xb3, 0x35,
            0x2a, 0x0d,
        ];

        let result = super::k1(&n, &salt, &p).unwrap();

        assert_eq!(
            &*result.into_bytes(),
            [
                0xf6, 0xed, 0x15, 0xa8, 0x93, 0x4a, 0xfb, 0xe7, 0xd8, 0x3e, 0x8d, 0xcb, 0x57, 0xfc,
                0xf5, 0xd7
            ]
        );
    }

    #[test]
    pub fn k2() {
        // 8.1.3 k2 function (Master)

        let n = [
            0xf7, 0xa2, 0xa4, 0x4f, 0x8e, 0x8a, 0x80, 0x29, 0x06, 0x4f, 0x17, 0x3d, 0xdc, 0x1e,
            0x2b, 0x00,
        ];

        let p = [00];

        let (nid, encryption_key, privacy_key) = super::k2(&n, &p).unwrap();

        assert_eq!(nid, 0x7f);
        assert_eq!(
            encryption_key,
            [
                0x9f, 0x58, 0x91, 0x81, 0xa0, 0xf5, 0x0d, 0xe7, 0x3c, 0x80, 0x70, 0xc7, 0xa6, 0xd2,
                0x7f, 0x46
            ]
        );
        assert_eq!(
            privacy_key,
            [
                0x4c, 0x71, 0x5b, 0xd4, 0xa6, 0x4b, 0x93, 0x8f, 0x99, 0xb4, 0x53, 0x35, 0x16, 0x53,
                0x12, 0x4f
            ]
        );
    }

    #[test]
    fn k4() {
        let n = [
            0x32, 0x16, 0xd1, 0x50, 0x98, 0x84, 0xb5, 0x33, 0x24, 0x85, 0x41, 0x79, 0x2b, 0x87,
            0x7f, 0x98,
        ];
        let result = super::k4(&n).unwrap();

        assert_eq!(result, 0x38);
    }
}
