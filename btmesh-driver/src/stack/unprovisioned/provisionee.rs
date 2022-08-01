use super::auth_value::{determine_auth_value, AuthValue};
use super::transcript::Transcript;
use crate::DriverError;
use btmesh_common::crypto::device::DeviceKey;
use btmesh_common::crypto::{
    aes_cmac,
    provisioning::{prck, prdk, prsk, prsn, try_decrypt_data},
    s1,
};
use btmesh_pdu::provisioning::{
    Capabilities, Confirmation, Data, ErrorCode, Failed, Invite, ProvisioningData, ProvisioningPDU,
    PublicKey, Random, Start,
};
use heapless::Vec;
use p256::elliptic_curve::ecdh::diffie_hellman;
use p256::SecretKey;
use rand_core::{CryptoRng, RngCore};

pub enum Provisionee {
    Beaconing(Phase<Beaconing>),
    Invitation(Phase<Invitation>),
    KeyExchange(Phase<KeyExchange>),
    Authentication(Phase<Authentication>),
    DataDistribution(Phase<DataDistribution>),
    Complete(DeviceKey, ProvisioningData),
    Failure(ErrorCode),
}

impl Provisionee {
    pub fn new(capabilities: Capabilities) -> Self {
        Self::Beaconing(Phase::<Beaconing>::new(capabilities))
    }

    pub fn in_progress(&self) -> bool {
        !matches!(
            self,
            Self::Beaconing(..) | Self::Complete(..) | Self::Failure(..)
        )
    }

    pub fn response(&self) -> Option<ProvisioningPDU> {
        match self {
            Self::Beaconing(phase) => phase.response.clone(),
            Self::Invitation(phase) => phase.response.clone(),
            Self::KeyExchange(phase) => phase.response.clone(),
            Self::Authentication(phase) => phase.response.clone(),
            Self::DataDistribution(phase) => phase.response.clone(),
            Self::Failure(ec) => Some(ProvisioningPDU::Failed(Failed {
                error_code: ec.clone(),
            })),
            Self::Complete(..) => Some(ProvisioningPDU::Complete),
        }
    }

    pub fn next<RNG: RngCore + CryptoRng>(
        self,
        pdu: &ProvisioningPDU,
        rng: &mut RNG,
    ) -> Result<Self, DriverError> {
        match (self, pdu) {
            // INVITE
            (Provisionee::Beaconing(mut phase), ProvisioningPDU::Invite(invite)) => {
                phase.invite(invite)?;
                Ok(Provisionee::Invitation(phase.try_into()?))
            }
            // START
            (Provisionee::Invitation(mut phase), ProvisioningPDU::Start(start)) => {
                // TODO: spec says to set the "Attention Timer" to 0x00
                phase.start(start, rng)?;
                // TODO: actually let the device/app/thingy know what
                // it is so that it can blink/flash/accept input
                Ok(Provisionee::KeyExchange(phase.try_into()?))
            }
            // PUBLIC KEY
            (Provisionee::KeyExchange(mut phase), ProvisioningPDU::PublicKey(peer_key)) => {
                match phase.calculate_ecdh(peer_key, rng) {
                    Ok(_) => Ok(Provisionee::Authentication(phase.try_into()?)),
                    Err(DriverError::InvalidFormat) => Provisionee::fail(ErrorCode::InvalidFormat),
                    Err(_) => Provisionee::fail(ErrorCode::UnexpectedError),
                }
            }
            // CONFIRMATION
            (Provisionee::Authentication(mut phase), ProvisioningPDU::Confirmation(value)) => {
                phase.confirmation(value, rng)?;
                Ok(Provisionee::Authentication(phase))
            }
            // RANDOM
            (Provisionee::Authentication(mut phase), ProvisioningPDU::Random(value)) => {
                match phase.check(value) {
                    Ok(_) => Ok(Provisionee::DataDistribution(phase.try_into()?)),
                    Err(_) => Provisionee::fail(ErrorCode::ConfirmationFailed),
                }
            }
            // DATA
            (Provisionee::DataDistribution(phase), ProvisioningPDU::Data(data)) => {
                let (device_key, decrypted) = phase.decrypt(data)?;
                let data = ProvisioningData::parse(&decrypted)?;
                Ok(Provisionee::Complete(device_key, data))
            }
            (current, _) => {
                // if it's an invalid PDU, assume it's just a wayward PDU and ignore, don't break.
                Ok(current)
            }
        }
    }

    fn fail(error_code: ErrorCode) -> Result<Provisionee, DriverError> {
        Ok(Provisionee::Failure(error_code))
    }
}

#[derive(Default)]
pub struct Phase<S> {
    transcript: Transcript,
    response: Option<ProvisioningPDU>,
    state: S,
}
#[derive(Default)]
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
    pub fn invite(&mut self, invitation: &Invite) -> Result<(), DriverError> {
        self.transcript.add_invite(invitation)?;
        Ok(())
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
    pub fn calculate_ecdh<RNG: RngCore + CryptoRng>(
        &mut self,
        key: &PublicKey,
        rng: &mut RNG,
    ) -> Result<PublicKey, DriverError> {
        let public: p256::PublicKey = match key.try_into() {
            Ok(v) => Ok(v),
            Err(_) => Err(DriverError::InvalidFormat),
        }?;
        let private = SecretKey::random(rng);
        let secret = &diffie_hellman(private.to_nonzero_scalar(), public.as_affine());
        self.state.shared_secret = Some(secret.as_bytes()[0..].try_into()?);
        let pk = private.public_key().try_into()?;
        self.transcript.add_pubkey_provisioner(key)?;
        self.transcript.add_pubkey_device(&pk)?;
        self.response = Some(ProvisioningPDU::PublicKey(pk));
        Ok(pk)
    }
}

impl Phase<Authentication> {
    pub fn confirmation<RNG: RngCore + CryptoRng>(
        &mut self,
        value: &Confirmation,
        rng: &mut RNG,
    ) -> Result<(), DriverError> {
        self.state.confirmation = Some(value.confirmation);
        rng.fill_bytes(&mut self.state.random_device);
        let confirmation = self.confirm(&self.state.random_device)?;
        self.response = Some(ProvisioningPDU::Confirmation(Confirmation { confirmation }));
        Ok(())
    }
    pub fn check(&mut self, value: &Random) -> Result<(), DriverError> {
        let confirmation = self.confirm(&value.random)?;
        match self.state.confirmation {
            Some(v) if v == confirmation => (),
            _ => return Err(DriverError::CryptoError),
        }
        self.state.random_provisioner = value.random;
        Ok(())
    }
    fn confirm(&self, random: &[u8]) -> Result<[u8; 16], DriverError> {
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
}

impl TryFrom<Phase<Beaconing>> for Phase<Invitation> {
    type Error = DriverError;
    fn try_from(mut p: Phase<Beaconing>) -> Result<Self, Self::Error> {
        p.transcript.add_capabilities(&p.state.capabilities)?;
        Ok(Phase {
            transcript: p.transcript,
            response: Some(ProvisioningPDU::Capabilities(p.state.capabilities)),
            ..Default::default()
        })
    }
}

impl TryFrom<Phase<Invitation>> for Phase<KeyExchange> {
    type Error = DriverError;
    fn try_from(p: Phase<Invitation>) -> Result<Self, Self::Error> {
        Ok(Phase {
            transcript: p.transcript,
            response: None,
            state: KeyExchange {
                auth_value: p.state.auth_value,
                ..Default::default()
            },
        })
    }
}

impl TryFrom<Phase<KeyExchange>> for Phase<Authentication> {
    type Error = DriverError;
    fn try_from(p: Phase<KeyExchange>) -> Result<Self, Self::Error> {
        Ok(Phase {
            transcript: p.transcript,
            response: p.response,
            state: Authentication {
                auth_value: p.state.auth_value,
                shared_secret: p.state.shared_secret.unwrap(),
                random_provisioner: p.state.random_provisioner,
                ..Default::default()
            },
        })
    }
}

impl TryFrom<Phase<Authentication>> for Phase<DataDistribution> {
    type Error = DriverError;
    fn try_from(p: Phase<Authentication>) -> Result<Self, Self::Error> {
        let random = p.state.random_device;
        Ok(Phase {
            transcript: p.transcript,
            response: Some(ProvisioningPDU::Random(Random { random })),
            state: DataDistribution {
                shared_secret: p.state.shared_secret,
                random_device: p.state.random_device,
                random_provisioner: p.state.random_provisioner,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use btmesh_pdu::provisioning::{Confirmation, Invite, PublicKey, Random, Start};
    use p256::SecretKey;
    use rand_core::OsRng;

    #[test]
    fn invitation() {
        let size = 69;
        let caps = Capabilities {
            number_of_elements: size,
            ..Default::default()
        };
        let mut fsm = Provisionee::new(caps);
        assert!(matches!(fsm, Provisionee::Beaconing(_)));
        let pdu = ProvisioningPDU::Invite(Invite {
            attention_duration: 30,
        });
        fsm = fsm.next(&pdu, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::Invitation(_)));
        match fsm.response() {
            Some(ProvisioningPDU::Capabilities(c)) => assert_eq!(c.number_of_elements, size),
            _ => panic!("wrong pdu returned for invite"),
        }
    }

    #[test]
    fn valid_keyexchange() {
        let mut fsm = keyexchange();
        let private = SecretKey::random(OsRng);
        let pdu = ProvisioningPDU::PublicKey(PublicKey::try_from(private.public_key()).unwrap());
        fsm = fsm.next(&pdu, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::Authentication(_)));
        assert!(matches!(
            fsm.response(),
            Some(ProvisioningPDU::PublicKey(_))
        ));
    }

    #[test]
    fn invalid_keyexchange() {
        let mut fsm = keyexchange();
        let (x, y) = ([0; 32], [0; 32]);
        let pdu = ProvisioningPDU::PublicKey(PublicKey { x, y });
        fsm = fsm.next(&pdu, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::Failure(..)));
        assert!(
            matches!(fsm.response(), Some(ProvisioningPDU::Failed(e)) if matches!(e.error_code, ErrorCode::InvalidFormat))
        );
    }

    #[test]
    fn valid_confirmation() {
        let mut random = [0; 16];
        OsRng.fill_bytes(&mut random);
        let mut fsm = confirmation(&random);
        let pdu = ProvisioningPDU::Random(Random { random });
        fsm = fsm.next(&pdu, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::DataDistribution(_)));
    }

    #[test]
    fn invalid_confirmation() {
        let mut random = [0; 16];
        let mut fsm = confirmation(&random);
        // Use a different random to break confirmation...
        OsRng.fill_bytes(&mut random);
        let pdu = ProvisioningPDU::Random(Random { random });
        fsm = fsm.next(&pdu, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::Failure(..)));
    }

    fn keyexchange() -> Provisionee {
        let mut fsm = Provisionee::new(Capabilities::default());
        let invite = ProvisioningPDU::Invite(Invite::default());
        fsm = fsm.next(&invite, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::Invitation(_)));
        let start = ProvisioningPDU::Start(Start::default());
        fsm = fsm.next(&start, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::KeyExchange(_)));
        assert!(matches!(fsm.response(), None));
        fsm
    }

    fn confirmation(random: &[u8]) -> Provisionee {
        let mut fsm = keyexchange();
        let private = SecretKey::random(OsRng);
        let pdu = ProvisioningPDU::PublicKey(PublicKey::try_from(private.public_key()).unwrap());
        fsm = fsm.next(&pdu, &mut OsRng).unwrap();
        let confirmation = match &fsm {
            Provisionee::Authentication(ref auth) => auth.confirm(random).unwrap(),
            _ => panic!("wrong state returned"),
        };
        let pdu = ProvisioningPDU::Confirmation(Confirmation { confirmation });
        fsm = fsm.next(&pdu, &mut OsRng).unwrap();
        match fsm.response() {
            Some(ProvisioningPDU::Confirmation(c)) => assert_ne!(c.confirmation, confirmation),
            _ => panic!("wrong pdu returned"),
        }
        fsm
    }
}
