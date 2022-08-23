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
    Capabilities, Confirmation, Data, ErrorCode, Failed, ProvisioningData, ProvisioningPDU,
    PublicKey, Random,
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
    Confirming(Phase<Confirming>),
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
            Self::Beaconing(_) => None,
            Self::Invitation(phase) => {
                Some(ProvisioningPDU::Capabilities(phase.capabilities.clone()))
            }
            Self::KeyExchange(_) => None,
            Self::Authentication(phase) => Some(ProvisioningPDU::PublicKey(phase.state.public_key)),
            Self::Confirming(phase) => Some(ProvisioningPDU::Confirmation(Confirmation {
                confirmation: phase.state.confirmation_device,
            })),
            Self::DataDistribution(phase) => Some(ProvisioningPDU::Random(Random {
                random: phase.random_device,
            })),
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
                phase.transcript.add_invite(invite)?;
                phase.transcript.add_capabilities(&phase.capabilities)?;
                Ok(Provisionee::Invitation(phase.into()))
            }
            // START
            (Provisionee::Invitation(mut phase), ProvisioningPDU::Start(start)) => {
                // TODO: spec says to set the "Attention Timer" to 0x00
                phase.transcript.add_start(start)?;
                phase.auth_value = determine_auth_value(rng, start)?;
                // TODO: actually let the device/app/thingy know what
                // it is so that it can blink/flash/accept input
                Ok(Provisionee::KeyExchange(phase.into()))
            }
            // PUBLIC KEY
            (Provisionee::KeyExchange(mut phase), ProvisioningPDU::PublicKey(peer_key)) => {
                match phase.calculate_ecdh(peer_key, rng) {
                    Ok(pk) => {
                        let mut next: Phase<Authentication> = phase.into();
                        next.state.public_key = pk;
                        Ok(Provisionee::Authentication(next))
                    }
                    Err(DriverError::InvalidFormat) => Provisionee::fail(ErrorCode::InvalidFormat),
                    Err(_) => Provisionee::fail(ErrorCode::UnexpectedError),
                }
            }
            // CONFIRMATION
            (Provisionee::Authentication(phase), ProvisioningPDU::Confirmation(value)) => {
                let mut next: Phase<Confirming> = phase.into();
                next.confirm(value, rng)?;
                Ok(Provisionee::Confirming(next))
            }
            // RANDOM
            (Provisionee::Confirming(mut phase), ProvisioningPDU::Random(value)) => {
                match phase.check(value) {
                    Ok(_) => Ok(Provisionee::DataDistribution(phase.into())),
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
    capabilities: Capabilities,
    auth_value: AuthValue,
    shared_secret: [u8; 32],
    random_provisioner: [u8; 16],
    random_device: [u8; 16],
    state: S,
}

impl<S> Phase<S> {
    fn confirmation(&self, random: &[u8]) -> Result<[u8; 16], DriverError> {
        let salt = self.transcript.confirmation_salt()?;
        let key = prck(&self.shared_secret, &salt.into_bytes())?;
        let mut bytes: Vec<u8, 32> = Vec::new();
        bytes.extend_from_slice(random)?;
        bytes.extend_from_slice(&self.auth_value.get_bytes())?;
        Ok(aes_cmac(&key.into_bytes(), &bytes)?.into_bytes().into())
    }
}

#[derive(Default)]
pub struct Beaconing {}
#[derive(Default)]
pub struct Invitation {}
#[derive(Default)]
pub struct KeyExchange {}
#[derive(Default)]
pub struct Authentication {
    public_key: PublicKey,
}
#[derive(Default)]
pub struct Confirming {
    confirmation_provider: [u8; 16],
    confirmation_device: [u8; 16],
}
#[derive(Default)]
pub struct DataDistribution {}

impl Phase<Beaconing> {
    pub fn new(capabilities: Capabilities) -> Self {
        Phase {
            capabilities,
            state: Beaconing {},
            ..Default::default()
        }
    }
}
impl From<Phase<Beaconing>> for Phase<Invitation> {
    fn from(p: Phase<Beaconing>) -> Self {
        Phase {
            transcript: p.transcript,
            capabilities: p.capabilities,
            ..Default::default()
        }
    }
}

impl From<Phase<Invitation>> for Phase<KeyExchange> {
    fn from(p: Phase<Invitation>) -> Self {
        Phase {
            transcript: p.transcript,
            auth_value: p.auth_value,
            state: KeyExchange {},
            ..Default::default()
        }
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
        self.shared_secret = secret.as_bytes()[0..].try_into()?;
        let pk = private.public_key().try_into()?;
        self.transcript.add_pubkey_provisioner(key)?;
        self.transcript.add_pubkey_device(&pk)?;
        Ok(pk)
    }
}
impl From<Phase<KeyExchange>> for Phase<Authentication> {
    fn from(p: Phase<KeyExchange>) -> Self {
        Phase {
            transcript: p.transcript,
            auth_value: p.auth_value,
            shared_secret: p.shared_secret,
            random_provisioner: p.random_provisioner,
            state: Authentication::default(),
            ..Default::default()
        }
    }
}

impl From<Phase<Authentication>> for Phase<Confirming> {
    fn from(p: Phase<Authentication>) -> Self {
        Phase {
            transcript: p.transcript,
            auth_value: p.auth_value,
            shared_secret: p.shared_secret,
            random_provisioner: p.random_provisioner,
            state: Confirming::default(),
            ..Default::default()
        }
    }
}

impl Phase<Confirming> {
    pub fn confirm<RNG: RngCore + CryptoRng>(
        &mut self,
        value: &Confirmation,
        rng: &mut RNG,
    ) -> Result<(), DriverError> {
        self.state.confirmation_provider = value.confirmation;
        rng.fill_bytes(&mut self.random_device);
        self.state.confirmation_device = self.confirmation(&self.random_device)?;
        Ok(())
    }
    pub fn check(&mut self, value: &Random) -> Result<(), DriverError> {
        let confirmation = self.confirmation(&value.random)?;
        if self.state.confirmation_provider != confirmation {
            return Err(DriverError::CryptoError);
        }
        self.random_provisioner = value.random;
        Ok(())
    }
}
impl From<Phase<Confirming>> for Phase<DataDistribution> {
    fn from(p: Phase<Confirming>) -> Self {
        Phase {
            transcript: p.transcript,
            shared_secret: p.shared_secret,
            random_provisioner: p.random_provisioner,
            random_device: p.random_device,
            state: DataDistribution {},
            ..Default::default()
        }
    }
}

impl Phase<DataDistribution> {
    pub fn decrypt(&self, data: &Data) -> Result<(DeviceKey, [u8; 25]), DriverError> {
        let mut salt = [0; 48];
        salt[0..16].copy_from_slice(&self.transcript.confirmation_salt()?.into_bytes());
        salt[16..32].copy_from_slice(&self.random_provisioner);
        salt[32..48].copy_from_slice(&self.random_device);
        let salt = &s1(&salt)?.into_bytes()[0..];
        let session_key = &prsk(&self.shared_secret, salt)?.into_bytes()[0..];
        let nonce = &prsn(&self.shared_secret, salt)?.into_bytes()[3..];

        let mut decrypted = [0; 25];
        decrypted.copy_from_slice(&data.encrypted);

        match try_decrypt_data(session_key, nonce, &mut decrypted, &data.mic) {
            Ok(_) => {
                let device_key = &*prdk(&self.shared_secret, salt)?.into_bytes();
                Ok((device_key.try_into()?, decrypted))
            }
            Err(_) => Err(DriverError::CryptoError),
        }
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
            Provisionee::Authentication(auth) => auth.confirmation(random).unwrap(),
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
