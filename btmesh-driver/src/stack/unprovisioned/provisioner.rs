use crate::DriverError;
use btmesh_common::crypto::{
    aes_cmac,
    provisioning::{encrypt_data, prck, prsk, prsn},
    s1,
};
use btmesh_pdu::provisioning::{
    Capabilities, Confirmation, Data, ErrorCode, Failed, Invite, ProvisioningData, ProvisioningPDU,
    PublicKey, Random, Start,
};
use heapless::Vec;
use p256::{elliptic_curve::ecdh::diffie_hellman, SecretKey};
use rand_core::{CryptoRng, RngCore};

use super::{
    auth_value::{determine_auth_value, AuthValue},
    transcript::Transcript,
};

pub enum Provisioner {
    Invitation(Phase<Invitation>),
    KeyExchange(Phase<KeyExchange>),
    Authentication(Phase<Authentication>),
    DataDistribution(Phase<DataDistribution>),
    Success,
    Failure(ResponsePDU),
}

impl Provisioner {
    pub fn new(data: ProvisioningData, attention_duration: u8) -> Result<Self, DriverError> {
        Ok(Self::Invitation(Phase::<Invitation>::new(
            data,
            attention_duration,
        )?))
    }

    pub fn response(&self) -> ResponsePDU {
        match self {
            Self::Invitation(phase) => phase.response.clone(),
            Self::KeyExchange(phase) => phase.response.clone(),
            Self::Authentication(phase) => phase.response.clone(),
            Self::DataDistribution(phase) => phase.response.clone(),
            Self::Failure(response) => response.clone(),
            Self::Success => ResponsePDU::None,
        }
    }

    pub fn next<RNG: RngCore + CryptoRng>(
        self,
        pdu: &ProvisioningPDU,
        rng: &mut RNG,
    ) -> Result<Self, DriverError> {
        match (self, pdu) {
            // CAPABILITIES
            (Provisioner::Invitation(mut phase), ProvisioningPDU::Capabilities(caps)) => {
                // TODO: This is when we know how many elements the
                // device has. How do we let the caller of this state
                // machine know that we need to reserve the
                // data.unicast_address passed to Provisioner::new
                // plus caps.number_of_elements?
                phase.capabilities(caps, rng)?;
                Ok(Provisioner::KeyExchange(phase.try_into()?))
            }
            // PUBLIC KEY
            (Provisioner::KeyExchange(mut phase), ProvisioningPDU::PublicKey(peer_key)) => {
                // TODO: OOB capabilities should determine whether we
                // return a Confirmation here or wait for the device
                // to send us an InputComplete

                match phase.calculate_ecdh(peer_key, rng) {
                    Ok(_) => Ok(Provisioner::Authentication(phase.try_into()?)),
                    Err(DriverError::InvalidFormat) => Provisioner::fail(ErrorCode::InvalidFormat),
                    Err(_) => Provisioner::fail(ErrorCode::UnexpectedError),
                }
            }
            // CONFIRMATION
            (Provisioner::Authentication(mut phase), ProvisioningPDU::Confirmation(value)) => {
                phase.confirmation(value)?;
                Ok(Provisioner::Authentication(phase))
            }
            // RANDOM
            (Provisioner::Authentication(mut phase), ProvisioningPDU::Random(value)) => {
                match phase.check(value) {
                    Ok(_) => Ok(Provisioner::DataDistribution(phase.try_into()?)),
                    Err(_) => Provisioner::fail(ErrorCode::ConfirmationFailed),
                }
            }
            // COMPLETE
            (Provisioner::DataDistribution(_), ProvisioningPDU::Complete) => {
                Ok(Provisioner::Success)
            }
            (current, _) => {
                // if it's an invalid PDU, assume it's just a wayward PDU and ignore, don't break.
                Ok(current)
            }
        }
    }
    fn fail(error_code: ErrorCode) -> Result<Provisioner, DriverError> {
        Ok(Provisioner::Failure(ResponsePDU::One(
            ProvisioningPDU::Failed(Failed { error_code }),
        )))
    }
}

#[derive(Default, Clone)]
pub enum ResponsePDU {
    Two([ProvisioningPDU; 2]),
    One(ProvisioningPDU),
    #[default]
    None,
}

impl<'a> IntoIterator for &'a ResponsePDU {
    type Item = &'a ProvisioningPDU;
    type IntoIter = core::slice::Iter<'a, ProvisioningPDU>;

    fn into_iter(self) -> core::slice::Iter<'a, ProvisioningPDU> {
        let slice = match self {
            ResponsePDU::None => &[],
            ResponsePDU::One(single) => core::slice::from_ref(single),
            ResponsePDU::Two(array) => array.as_slice(),
        };
        slice.iter()
    }
}

#[derive(Default)]
pub struct Phase<S> {
    pub response: ResponsePDU,
    transcript: Transcript,
    data: Option<ProvisioningData>,
    state: S,
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

impl Phase<Invitation> {
    pub fn new(data: ProvisioningData, attention_duration: u8) -> Result<Self, DriverError> {
        let mut result = Self {
            data: Some(data),
            ..Default::default()
        };
        let invitation = Invite { attention_duration };
        result.transcript.add_invite(&invitation)?;
        result.response = ResponsePDU::One(ProvisioningPDU::Invite(invitation));
        Ok(result)
    }
    pub fn capabilities<RNG: RngCore + CryptoRng>(
        &mut self,
        capabilities: &Capabilities,
        rng: &mut RNG,
    ) -> Result<(), DriverError> {
        self.transcript.add_capabilities(capabilities)?;
        // TODO: derive Start from Capabilities
        let start = Start::default();
        self.transcript.add_start(&start)?;
        self.state.auth_value = determine_auth_value(rng, &start)?;
        let private = SecretKey::random(rng);
        let public = private.public_key().try_into()?;
        self.state.private = Some(private);
        self.response = ResponsePDU::Two([
            ProvisioningPDU::Start(start),
            ProvisioningPDU::PublicKey(public),
        ]);
        Ok(())
    }
}

impl Phase<KeyExchange> {
    pub fn calculate_ecdh<RNG: RngCore + CryptoRng>(
        &mut self,
        key: &PublicKey,
        rng: &mut RNG,
    ) -> Result<[u8; 16], DriverError> {
        let public: p256::PublicKey = match key.try_into() {
            Ok(v) => Ok(v),
            Err(_) => Err(DriverError::InvalidFormat),
        }?;
        match &self.state.private {
            Some(private) => {
                let secret = &diffie_hellman(private.to_nonzero_scalar(), public.as_affine());
                self.state.shared_secret = Some(secret.as_bytes()[0..].try_into()?);
                let pk = private.public_key().try_into()?;
                self.transcript.add_pubkey_provisioner(&pk)?;
                self.transcript.add_pubkey_device(key)?;
                rng.fill_bytes(&mut self.state.random_provisioner);
                Ok(self.state.random_provisioner)
            }
            None => Err(DriverError::InvalidState),
        }
    }
}

impl Phase<Authentication> {
    pub fn confirmation(&mut self, value: &Confirmation) -> Result<(), DriverError> {
        self.state.confirmation = Some(value.confirmation);
        self.response = ResponsePDU::One(ProvisioningPDU::Random(Random {
            random: self.state.random_provisioner,
        }));
        Ok(())
    }
    pub fn check(&mut self, value: &Random) -> Result<(), DriverError> {
        let confirmation = self.confirm(&value.random)?;
        match self.state.confirmation {
            Some(v) if v == confirmation => (),
            _ => return Err(DriverError::CryptoError),
        }
        self.state.random_device = value.random;
        Ok(())
    }
    fn confirm(&self, random: &[u8]) -> Result<[u8; 16], DriverError> {
        let salt = self.transcript.confirmation_salt()?;
        let key = prck(&self.state.shared_secret, &salt.into_bytes())?;
        let mut bytes: Vec<u8, 32> = Vec::new();
        bytes.extend_from_slice(random)?;
        bytes.extend_from_slice(&self.state.auth_value.get_bytes())?;
        Ok(aes_cmac(&key.into_bytes(), &bytes)?.into_bytes().into())
    }
}

impl Phase<DataDistribution> {
    pub fn encrypt(&self) -> Result<Data, DriverError> {
        let mut salt = [0; 48];
        salt[0..16].copy_from_slice(&self.transcript.confirmation_salt()?.into_bytes());
        salt[16..32].copy_from_slice(&self.state.random_provisioner);
        salt[32..48].copy_from_slice(&self.state.random_device);
        let salt = &s1(&salt)?.into_bytes()[0..];
        let session_key = &prsk(&self.state.shared_secret, salt)?.into_bytes()[0..];
        let nonce = &prsn(&self.state.shared_secret, salt)?.into_bytes()[3..];

        let mut encrypted = Vec::<u8, 25>::new();
        self.data
            .ok_or(DriverError::InvalidState)?
            .emit(&mut encrypted)?;
        let mut mic = [0; 8];

        if encrypt_data(session_key, nonce, &mut encrypted, &mut mic).is_err() {
            Err(DriverError::CryptoError)
        } else {
            let encrypted = encrypted
                .into_array()
                .map_err(|_| DriverError::CryptoError)?;
            Ok(Data { encrypted, mic })
        }
    }
}

impl TryFrom<Phase<Invitation>> for Phase<KeyExchange> {
    type Error = DriverError;
    fn try_from(p: Phase<Invitation>) -> Result<Self, Self::Error> {
        Ok(Phase {
            transcript: p.transcript,
            data: p.data,
            response: p.response,
            state: KeyExchange {
                auth_value: p.state.auth_value,
                private: p.state.private,
                ..Default::default()
            },
        })
    }
}

impl TryFrom<Phase<KeyExchange>> for Phase<Authentication> {
    type Error = DriverError;
    fn try_from(p: Phase<KeyExchange>) -> Result<Self, Self::Error> {
        let mut phase = Phase {
            transcript: p.transcript,
            data: p.data,
            response: p.response,
            state: Authentication {
                auth_value: p.state.auth_value,
                shared_secret: p.state.shared_secret.unwrap(),
                random_provisioner: p.state.random_provisioner,
                ..Default::default()
            },
        };
        let confirmation = phase.confirm(&p.state.random_provisioner)?;
        phase.response =
            ResponsePDU::One(ProvisioningPDU::Confirmation(Confirmation { confirmation }));
        Ok(phase)
    }
}

impl TryFrom<Phase<Authentication>> for Phase<DataDistribution> {
    type Error = DriverError;
    fn try_from(p: Phase<Authentication>) -> Result<Self, Self::Error> {
        let mut phase = Phase {
            transcript: p.transcript,
            data: p.data,
            response: p.response,
            state: DataDistribution {
                shared_secret: p.state.shared_secret,
                random_device: p.state.random_device,
                random_provisioner: p.state.random_provisioner,
            },
        };
        phase.response = ResponsePDU::One(ProvisioningPDU::Data(phase.encrypt()?));
        Ok(phase)
    }
}

#[cfg(test)]
mod tests {
    use core::ops::Deref;

    use super::*;
    use crate::stack::unprovisioned::provisionee::Provisionee;
    use btmesh_common::{address::UnicastAddress, KeyRefreshFlag};
    use btmesh_pdu::provisioning::{Capabilities, ProvisioningPDU::Failed};
    use rand_core::OsRng;

    #[test]
    fn provision_device() {
        let rng = &mut OsRng;

        let fixture = ProvisioningData {
            unicast_address: UnicastAddress::new(0x00_0A).unwrap(),
            key_refresh_flag: KeyRefreshFlag(true),
            ..Default::default()
        };
        let mut provisioner = Provisioner::new(fixture, 60).unwrap();
        let mut device = Provisionee::new(Capabilities {
            number_of_elements: 1,
            ..Default::default()
        });
        loop {
            for pdu in provisioner.response().into_iter() {
                assert!(!matches!(pdu, Failed(_)), "Unexpected PDU: {:?}", pdu);
                device = match device.next(pdu, rng) {
                    Ok(provisionee) => {
                        if let Some(pdu) = provisionee.response() {
                            provisioner = match provisioner.next(&pdu, rng) {
                                Ok(p) => p,
                                Err(e) => panic!("provisoner error: {:?}", e),
                            }
                        }
                        provisionee
                    }
                    Err(e) => {
                        panic!("device error: {:?}", e);
                    }
                };
            }
            if !device.in_progress() {
                break;
            }
        }
        match device {
            Provisionee::Complete(key, result) => {
                assert_ne!(&[0; 16], key.deref());
                assert_eq!(fixture, result);
            }
            _ => panic!("wrong ending state"),
        }
    }
}
