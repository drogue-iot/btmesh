use super::auth_value::{determine_auth_value, AuthValue};
use super::transcript::Transcript;
use crate::DriverError;
use btmesh_common::crypto::device::DeviceKey;
use btmesh_common::crypto::{
    aes_cmac,
    provisioning::{prck, prdk, prsk, prsn, try_decrypt_data},
    s1,
};
use btmesh_common::ParseError;
use btmesh_pdu::provisioning::{
    Capabilities, Confirmation, Data, Invite, ProvisioningPDU, PublicKey, Random, Start,
};
use heapless::Vec;
use p256::elliptic_curve::ecdh::diffie_hellman;
use p256::SecretKey;
use rand_core::{CryptoRng, RngCore};

pub struct Phase<S> {
    transcript: Transcript,
    state: S,
}

pub struct Beaconing {
    capabilities: Capabilities,
}
#[derive(Default)]
pub struct Invitation {
    auth_value: AuthValue,
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
    confirmation: Option<[u8; 16]>,
    random_device: Option<[u8; 16]>,
    random_provisioner: Option<[u8; 16]>,
}
pub struct DataDistribution {
    shared_secret: [u8; 32],
    random_device: [u8; 16],
    random_provisioner: [u8; 16],
}

impl Phase<Beaconing> {
    pub fn new(capabilities: Capabilities) -> Self {
        Phase {
            transcript: Transcript::default(),
            state: Beaconing { capabilities },
        }
    }
    pub fn invite(&mut self, invitation: &Invite) -> Result<ProvisioningPDU, DriverError> {
        let capabilities = self.state.capabilities.clone();
        self.transcript.add_invite(invitation)?;
        self.transcript.add_capabilities(&capabilities)?;
        Ok(ProvisioningPDU::Capabilities(capabilities))
    }
}

impl Phase<Invitation> {
    pub fn start<RNG: RngCore + CryptoRng>(
        &mut self,
        start: &Start,
        rng: &mut RNG,
    ) -> Result<(), DriverError> {
        self.transcript.add_start(start)?;
        self.state.auth_value = determine_auth_value(rng, start)?;
        Ok(())
    }
}

impl Phase<KeyExchange> {
    pub fn calculate_ecdh_provisioner(
        &mut self,
        key: &PublicKey,
    ) -> Result<PublicKey, DriverError> {
        let public = Self::validate(key)?;
        match self.state.private.take() {
            Some(private) => {
                self.calculate(&private, &public)?;
                let pk = private.public_key().try_into()?;
                self.transcript.add_pubkey_provisioner(&pk)?;
                self.transcript.add_pubkey_device(key)?;
                Ok(pk)
            }
            None => Err(DriverError::InvalidState),
        }
    }
    pub fn calculate_ecdh_device<RNG: RngCore + CryptoRng>(
        &mut self,
        key: &PublicKey,
        rng: &mut RNG,
    ) -> Result<PublicKey, DriverError> {
        let public = Self::validate(key)?;
        match self.state.private.take() {
            Some(_) => Err(DriverError::InvalidState),
            None => {
                let private = SecretKey::random(rng);
                self.calculate(&private, &public)?;
                let pk = private.public_key().try_into()?;
                self.transcript.add_pubkey_provisioner(key)?;
                self.transcript.add_pubkey_device(&pk)?;
                Ok(pk)
            }
        }
    }
    fn validate(pk: &PublicKey) -> Result<p256::PublicKey, DriverError> {
        match pk.try_into() {
            Ok(v) => Ok(v),
            Err(_) => Err(ParseError::InvalidValue.into()),
        }
    }
    fn calculate(
        &mut self,
        private: &SecretKey,
        public: &p256::PublicKey,
    ) -> Result<(), DriverError> {
        let secret = &diffie_hellman(private.to_nonzero_scalar(), public.as_affine());
        self.state.shared_secret = Some(secret.as_bytes()[0..].try_into()?);
        Ok(())
    }
}

impl Phase<Authentication> {
    pub fn store<RNG: RngCore + CryptoRng>(
        &mut self,
        value: &Confirmation,
        rng: &mut RNG,
    ) -> Result<ProvisioningPDU, DriverError> {
        self.state.confirmation = Some(value.confirmation);
        let mut random_device = [0; 16];
        rng.fill_bytes(&mut random_device);
        let confirmation = self.confirm(&random_device)?;
        self.state.random_device = Some(random_device);
        Ok(ProvisioningPDU::Confirmation(Confirmation { confirmation }))
    }
    pub fn check(&mut self, value: &Random) -> Result<ProvisioningPDU, DriverError> {
        let confirmation = self.confirm(&value.random)?;
        match self.state.confirmation {
            Some(v) if v == confirmation => (),
            _ => return Err(DriverError::CryptoError),
        }
        self.state.random_provisioner = Some(value.random);
        let random = self.state.random_device.ok_or(DriverError::CryptoError)?;
        Ok(ProvisioningPDU::Random(Random { random }))
    }
    pub(super) fn confirm(&self, random: &[u8]) -> Result<[u8; 16], DriverError> {
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
