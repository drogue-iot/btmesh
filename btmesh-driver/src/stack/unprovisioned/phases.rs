use super::auth_value::AuthValue;
use super::transcript::Transcript;
use crate::DriverError;
use btmesh_common::crypto::device::DeviceKey;
use btmesh_common::crypto::{
    aes_cmac,
    provisioning::{prck, prdk, prsk, prsn, try_decrypt_data},
    s1,
};
use btmesh_common::ParseError;
use btmesh_pdu::provisioning::{Capabilities, Data, PublicKey};
use heapless::Vec;
use p256::elliptic_curve::ecdh::diffie_hellman;
use p256::SecretKey;
use rand_core::{CryptoRng, RngCore};

pub struct Phase<S> {
    pub transcript: Transcript,
    pub state: S,
}

pub struct Beaconing {
    pub capabilities: Capabilities,
}
#[derive(Default)]
pub struct Invitation {
    pub auth_value: AuthValue,
}
#[derive(Default)]
pub struct KeyExchange {
    auth_value: AuthValue,
    private: Option<SecretKey>,
    shared_secret: Option<[u8; 32]>,
}
#[derive(Default)]
pub struct Authentication {
    auth_value: AuthValue,
    shared_secret: [u8; 32],
    pub confirmation: Option<[u8; 16]>,
    pub random_device: Option<[u8; 16]>,
    pub random_provisioner: Option<[u8; 16]>,
}
pub struct DataDistribution {
    shared_secret: [u8; 32],
    random_device: [u8; 16],
    random_provisioner: [u8; 16],
}

impl Phase<KeyExchange> {
    pub fn calculate_ecdh<RNG: RngCore + CryptoRng>(
        &mut self,
        key: &PublicKey,
        rng: &mut RNG,
    ) -> Result<Option<PublicKey>, DriverError> {
        let public: p256::PublicKey = match key.try_into() {
            Ok(v) => v,
            Err(_) => return Err(ParseError::InvalidValue.into()),
        };
        let (private, result) = match self.state.private.take() {
            Some(v) => {
                let provisioner_pk = &v.public_key().try_into()?;
                self.transcript.add_pubkey_provisioner(provisioner_pk)?;
                self.transcript.add_pubkey_device(key)?;
                (v, Ok(None))
            }
            None => {
                let v = SecretKey::random(rng);
                let device_pk = v.public_key().try_into()?;
                self.transcript.add_pubkey_provisioner(key)?;
                self.transcript.add_pubkey_device(&device_pk)?;
                (v, Ok(Some(device_pk)))
            }
        };
        let secret = &diffie_hellman(private.to_nonzero_scalar(), public.as_affine());
        self.state.shared_secret = Some(secret.as_bytes()[0..].try_into()?);
        result
    }
}

impl Phase<Authentication> {
    pub fn confirm(&self, random: &[u8]) -> Result<[u8; 16], DriverError> {
        let salt = self.transcript.confirmation_salt()?;
        let key = prck(&self.state.shared_secret, &*salt.into_bytes())?;
        let mut bytes: Vec<u8, 32> = Vec::new();
        bytes.extend_from_slice(random)?;
        bytes.extend_from_slice(&self.state.auth_value.get_bytes())?;
        Ok(aes_cmac(&key.into_bytes(), &bytes)?.into_bytes().into())
    }
}

impl Phase<DataDistribution> {
    pub fn decrypt(&self, data: &Data) -> Result<(DeviceKey, [u8; 25]), DriverError> {
        let mut salt = [0; 48];
        salt[0..16].copy_from_slice(&self.transcript.confirmation_salt()?.into_bytes());
        salt[16..32].copy_from_slice(&self.state.random_provisioner);
        salt[32..48].copy_from_slice(&self.state.random_device);
        let salt = &s1(&salt)?.into_bytes()[0..];
        let key = &prsk(&self.state.shared_secret, salt)?.into_bytes()[0..];
        let nonce = &prsn(&self.state.shared_secret, salt)?.into_bytes()[3..];

        let mut decrypted = [0; 25];
        decrypted.copy_from_slice(&data.encrypted);

        match try_decrypt_data(key, nonce, &mut decrypted, &data.mic, None) {
            Ok(_) => {
                let device_key = &*prdk(&self.state.shared_secret, salt)?.into_bytes();
                Ok((device_key.try_into()?, decrypted))
            }
            Err(_) => Err(DriverError::CryptoError),
        }
    }
}

impl From<Phase<Beaconing>> for Phase<Invitation> {
    fn from(p: Phase<Beaconing>) -> Phase<Invitation> {
        Phase {
            transcript: p.transcript,
            state: Invitation::default(),
        }
    }
}

impl From<Phase<Invitation>> for Phase<KeyExchange> {
    fn from(p: Phase<Invitation>) -> Phase<KeyExchange> {
        Phase {
            transcript: p.transcript,
            state: KeyExchange {
                auth_value: p.state.auth_value,
                ..Default::default()
            },
        }
    }
}

impl From<Phase<KeyExchange>> for Phase<Authentication> {
    fn from(p: Phase<KeyExchange>) -> Phase<Authentication> {
        Phase {
            transcript: p.transcript,
            state: Authentication {
                auth_value: p.state.auth_value,
                shared_secret: p.state.shared_secret.unwrap(),
                ..Default::default()
            },
        }
    }
}

impl From<Phase<Authentication>> for Phase<DataDistribution> {
    fn from(p: Phase<Authentication>) -> Phase<DataDistribution> {
        Phase {
            transcript: p.transcript,
            state: DataDistribution {
                shared_secret: p.state.shared_secret,
                random_device: p.state.random_device.unwrap(),
                random_provisioner: p.state.random_provisioner.unwrap(),
            },
        }
    }
}
