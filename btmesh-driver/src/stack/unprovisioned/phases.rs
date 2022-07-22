use super::auth_value::{determine_auth_value, AuthValue};
use super::transcript::Transcript;
use crate::DriverError;
use btmesh_common::crypto::device::DeviceKey;
use btmesh_common::crypto::{
    aes_cmac,
    provisioning::{encrypt_data, prck, prdk, prsk, prsn, try_decrypt_data},
    s1,
};
use btmesh_common::ParseError;
use btmesh_pdu::provisioning::{
    Capabilities, Confirmation, Data, Invite, ProvisioningData, PublicKey, Random, Start,
};
use heapless::Vec;
use p256::elliptic_curve::ecdh::diffie_hellman;
use p256::SecretKey;
use rand_core::{CryptoRng, RngCore};

#[derive(Default)]
pub struct Phase<S> {
    transcript: Transcript,
    data: Option<ProvisioningData>,
    state: S,
}
#[derive(Default)]
pub struct Beaconing {
    capabilities: Capabilities,
}
#[derive(Default)]
pub struct Invitation {
    auth_value: AuthValue,
    private: Option<SecretKey>,
}
#[derive(Default)]
pub struct KeyExchange {
    auth_value: AuthValue,
    private: Option<SecretKey>,
    shared_secret: Option<[u8; 32]>,
    random_provisioner: [u8; 16],
}
#[derive(Default)]
pub struct Authentication {
    auth_value: AuthValue,
    shared_secret: [u8; 32],
    confirmation: Option<[u8; 16]>,
    random_device: [u8; 16],
    random_provisioner: [u8; 16],
}
pub struct DataDistribution {
    shared_secret: [u8; 32],
    random_device: [u8; 16],
    random_provisioner: [u8; 16],
}

impl Phase<Beaconing> {
    pub fn new(capabilities: Capabilities) -> Self {
        Phase {
            state: Beaconing { capabilities },
            ..Default::default()
        }
    }
    pub fn invite(&mut self, invitation: &Invite) -> Result<Capabilities, DriverError> {
        let capabilities = self.state.capabilities.clone();
        self.transcript.add_invite(invitation)?;
        self.transcript.add_capabilities(&capabilities)?;
        Ok(capabilities)
    }
}

impl Phase<Invitation> {
    pub fn new(invitation: &Invite, data: ProvisioningData) -> Result<Self, DriverError> {
        let mut result = Self {
            data: Some(data),
            ..Default::default()
        };
        result.transcript.add_invite(invitation)?;
        Ok(result)
    }
    pub fn start<RNG: RngCore + CryptoRng>(
        &mut self,
        start: &Start,
        rng: &mut RNG,
    ) -> Result<(), DriverError> {
        self.transcript.add_start(start)?;
        self.state.auth_value = determine_auth_value(rng, start)?;
        Ok(())
    }
    pub fn capabilities<RNG: RngCore + CryptoRng>(
        &mut self,
        capabilities: &Capabilities,
        rng: &mut RNG,
    ) -> Result<(Start, PublicKey), DriverError> {
        self.transcript.add_capabilities(capabilities)?;
        // TODO: derive Start from Capabilities
        let start = Start::default();
        self.start(&start, rng)?; // updates transcript and sets auth_value
        let public_key = self.create_key(rng)?;
        Ok((start, public_key))
    }
    fn create_key<RNG: RngCore + CryptoRng>(
        &mut self,
        rng: &mut RNG,
    ) -> Result<PublicKey, DriverError> {
        let private = SecretKey::random(rng);
        let public = private.public_key().try_into()?;
        self.state.private = Some(private);
        Ok(public)
    }
}

impl Phase<KeyExchange> {
    pub fn calculate_ecdh_provisioner<RNG: RngCore + CryptoRng>(
        &mut self,
        key: &PublicKey,
        rng: &mut RNG,
    ) -> Result<[u8; 16], DriverError> {
        let public = Self::validate(key)?;
        match self.state.private.take() {
            Some(private) => {
                self.calculate(&private, &public)?;
                let pk = private.public_key().try_into()?;
                self.transcript.add_pubkey_provisioner(&pk)?;
                self.transcript.add_pubkey_device(key)?;
                rng.fill_bytes(&mut self.state.random_provisioner);
                Ok(self.state.random_provisioner)
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
    pub fn device_confirmation<RNG: RngCore + CryptoRng>(
        &mut self,
        value: &Confirmation,
        rng: &mut RNG,
    ) -> Result<Confirmation, DriverError> {
        self.state.confirmation = Some(value.confirmation);
        rng.fill_bytes(&mut self.state.random_device);
        let confirmation = self.confirm(&self.state.random_device)?;
        Ok(Confirmation { confirmation })
    }
    pub fn provisioner_confirmation(
        &mut self,
        value: &Confirmation,
    ) -> Result<Random, DriverError> {
        self.state.confirmation = Some(value.confirmation);
        Ok(Random {
            random: self.state.random_provisioner,
        })
    }
    pub fn device_check(&mut self, value: &Random) -> Result<Random, DriverError> {
        let confirmation = self.confirm(&value.random)?;
        match self.state.confirmation {
            Some(v) if v == confirmation => (),
            _ => return Err(DriverError::CryptoError),
        }
        self.state.random_provisioner = value.random;
        Ok(Random {
            random: self.state.random_device,
        })
    }
    pub fn provisioner_check(&mut self, value: &Random) -> Result<(), DriverError> {
        let confirmation = self.confirm(&value.random)?;
        match self.state.confirmation {
            Some(v) if v == confirmation => (),
            _ => return Err(DriverError::CryptoError),
        }
        self.state.random_device = value.random;
        Ok(())
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
        let session_key = &prsk(&self.state.shared_secret, salt)?.into_bytes()[0..];
        let nonce = &prsn(&self.state.shared_secret, salt)?.into_bytes()[3..];

        let mut decrypted = [0; 25];
        decrypted.copy_from_slice(&data.encrypted);

        match try_decrypt_data(session_key, nonce, &mut decrypted, &data.mic) {
            Ok(_) => {
                let device_key = &*prdk(&self.state.shared_secret, salt)?.into_bytes();
                Ok((device_key.try_into()?, decrypted))
            }
            Err(_) => Err(DriverError::CryptoError),
        }
    }
    pub fn encrypt(&self) -> Result<Data, DriverError> {
        let mut salt = [0; 48];
        salt[0..16].copy_from_slice(&self.transcript.confirmation_salt()?.into_bytes());
        salt[16..32].copy_from_slice(&self.state.random_provisioner);
        salt[32..48].copy_from_slice(&self.state.random_device);
        let salt = &s1(&salt)?.into_bytes()[0..];
        let session_key = &prsk(&self.state.shared_secret, salt)?.into_bytes()[0..];
        let nonce = &prsn(&self.state.shared_secret, salt)?.into_bytes()[3..];

        let mut encrypted = [0; 25];
        let mut mic = [0; 8];

        if encrypt_data(session_key, nonce, &mut encrypted, &mut mic).is_err() {
            Err(DriverError::CryptoError)
        } else {
            Ok(Data { encrypted, mic })
        }
    }
}

impl From<Phase<Beaconing>> for Phase<Invitation> {
    fn from(p: Phase<Beaconing>) -> Phase<Invitation> {
        Phase {
            transcript: p.transcript,
            ..Default::default()
        }
    }
}

impl From<Phase<Invitation>> for Phase<KeyExchange> {
    fn from(p: Phase<Invitation>) -> Phase<KeyExchange> {
        Phase {
            transcript: p.transcript,
            data: p.data,
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
            data: p.data,
            state: Authentication {
                auth_value: p.state.auth_value,
                shared_secret: p.state.shared_secret.unwrap(),
                random_provisioner: p.state.random_provisioner,
                ..Default::default()
            },
        }
    }
}

impl From<Phase<Authentication>> for Phase<DataDistribution> {
    fn from(p: Phase<Authentication>) -> Phase<DataDistribution> {
        Phase {
            transcript: p.transcript,
            data: p.data,
            state: DataDistribution {
                shared_secret: p.state.shared_secret,
                random_device: p.state.random_device,
                random_provisioner: p.state.random_provisioner,
            },
        }
    }
}
