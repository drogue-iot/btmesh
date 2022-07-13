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
    Capabilities, Confirmation, ErrorCode, Failed, ProvisioningData, ProvisioningPDU, PublicKey,
    Random,
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
    Failure,
}

impl Provisionee {
    pub fn new(capabilities: Capabilities) -> Self {
        Self::Beaconing(Phase {
            transcript: Transcript::default(),
            state: Beaconing { capabilities },
        })
    }

    pub fn in_progress(&self) -> bool {
        !matches!(
            self,
            Self::Beaconing(..) | Self::Complete(..) | Self::Failure
        )
    }

    pub fn next<RNG: RngCore + CryptoRng>(
        self,
        pdu: &ProvisioningPDU,
        rng: &mut RNG,
    ) -> Result<(Self, Option<ProvisioningPDU>), DriverError> {
        match (self, pdu) {
            (Provisionee::Beaconing(mut device), ProvisioningPDU::Invite(invite)) => {
                let capabilities = device.state.capabilities.clone();
                device.transcript.add_invite(invite)?;
                device.transcript.add_capabilities(&capabilities)?;
                Ok((
                    Provisionee::Invitation(device.into()),
                    Some(ProvisioningPDU::Capabilities(capabilities)),
                ))
            }
            (Provisionee::Invitation(mut device), ProvisioningPDU::Start(start)) => {
                // TODO: spec says to set the "Attention Timer" to 0x00
                device.transcript.add_start(start)?;
                device.state.auth_value = determine_auth_value(rng, start)?;
                // TODO: actually let the device/app/thingy know what
                // it is so that it can blink/flash/accept input
                Ok((Provisionee::KeyExchange(device.into()), None))
            }
            (Provisionee::KeyExchange(mut device), ProvisioningPDU::PublicKey(peer_key)) => {
                let public: p256::PublicKey = match peer_key.try_into() {
                    Ok(key) => key,
                    Err(_) => return Provisionee::fail(ErrorCode::InvalidFormat),
                };
                // TODO: logic may depend on which peer (provisioner or device) we are
                device.transcript.add_pubkey_provisioner(peer_key)?;
                let private = SecretKey::random(rng);
                let secret = &diffie_hellman(private.to_nonzero_scalar(), public.as_affine());
                device.state.shared_secret = Some(secret.as_bytes()[0..].try_into()?);
                let pk: PublicKey = private.public_key().try_into()?;
                device.transcript.add_pubkey_device(&pk)?;
                Ok((
                    Provisionee::Authentication(device.into()),
                    Some(ProvisioningPDU::PublicKey(pk)),
                ))
            }
            (Provisionee::Authentication(mut device), ProvisioningPDU::Confirmation(value)) => {
                device.state.confirmation = Some(value.confirmation);
                let mut random_device = [0; 16];
                rng.fill_bytes(&mut random_device);
                let confirmation = device.confirm(&random_device)?;
                device.state.random_device = Some(random_device);
                Ok((
                    Provisionee::Authentication(device),
                    Some(ProvisioningPDU::Confirmation(Confirmation { confirmation })),
                ))
            }
            (Provisionee::Authentication(mut device), ProvisioningPDU::Random(value)) => {
                let confirmation = device.confirm(&value.random)?;
                match device.state.confirmation {
                    Some(v) if v == confirmation => (),
                    _ => return Provisionee::fail(ErrorCode::ConfirmationFailed),
                }
                device.state.random_provisioner = Some(value.random);
                let device_random = device.state.random_device.ok_or(DriverError::CryptoError)?;
                Ok((
                    Provisionee::DataDistribution(device.into()),
                    Some(ProvisioningPDU::Random(Random {
                        random: device_random,
                    })),
                ))
            }
            (Provisionee::DataDistribution(device), ProvisioningPDU::Data(data)) => {
                let mut salt = [0; 48];
                salt[0..16].copy_from_slice(&device.transcript.confirmation_salt()?.into_bytes());
                salt[16..32].copy_from_slice(&device.state.random_provisioner);
                salt[32..48].copy_from_slice(&device.state.random_device);
                let salt = &s1(&salt)?.into_bytes()[0..];
                let key = &prsk(&device.state.shared_secret, salt)?.into_bytes()[0..];
                let nonce = &prsn(&device.state.shared_secret, salt)?.into_bytes()[3..];

                let mut decrypted = [0; 25];
                decrypted.copy_from_slice(&data.encrypted);

                match try_decrypt_data(key, nonce, &mut decrypted, &data.mic, None) {
                    Ok(_) => {
                        let device_key = &*prdk(&device.state.shared_secret, salt)?.into_bytes();
                        let device_key = DeviceKey::try_from(device_key)?;
                        Ok((
                            Provisionee::Complete(device_key, ProvisioningData::parse(&decrypted)?),
                            Some(ProvisioningPDU::Complete),
                        ))
                    }
                    Err(_) => Err(DriverError::CryptoError),
                }
            }
            (current_state, _) => {
                // if it's an invalid PDU, assume it's just a wayward PDU and ignore, don't break.
                Ok((current_state, None))
            }
        }
    }

    fn fail(error_code: ErrorCode) -> Result<(Provisionee, Option<ProvisioningPDU>), DriverError> {
        Ok((
            Provisionee::Failure,
            Some(ProvisioningPDU::Failed(Failed { error_code })),
        ))
    }
}

pub struct Phase<S> {
    transcript: Transcript,
    state: S,
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
                shared_secret: None,
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

pub struct Beaconing {
    capabilities: Capabilities,
}
#[derive(Default)]
pub struct Invitation {
    auth_value: AuthValue,
}
pub struct KeyExchange {
    auth_value: AuthValue,
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

impl Phase<Authentication> {
    fn confirm(&self, random: &[u8]) -> Result<[u8; 16], DriverError> {
        let salt = self.transcript.confirmation_salt()?;
        let key = prck(&self.state.shared_secret, &*salt.into_bytes())?;
        let mut bytes: Vec<u8, 32> = Vec::new();
        bytes.extend_from_slice(random)?;
        bytes.extend_from_slice(&self.state.auth_value.get_bytes())?;
        Ok(aes_cmac(&key.into_bytes(), &bytes)?.into_bytes().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use btmesh_pdu::provisioning::{Invite, Start};
    use rand_core::OsRng;

    #[test]
    fn invitation() {
        let size = 69;
        let caps = Capabilities {
            number_of_elements: size,
            ..Default::default()
        };
        let fsm = Provisionee::new(caps);
        assert!(matches!(fsm, Provisionee::Beaconing(_)));
        let pdu = ProvisioningPDU::Invite(Invite {
            attention_duration: 30,
        });
        let (fsm, pdu) = fsm.next(&pdu, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::Invitation(_)));
        match pdu {
            Some(ProvisioningPDU::Capabilities(c)) => assert_eq!(c.number_of_elements, size),
            _ => panic!("wrong pdu returned for invite"),
        }
    }

    #[test]
    fn valid_keyexchange() {
        let fsm = keyexchange();
        let private = SecretKey::random(OsRng);
        let pdu = ProvisioningPDU::PublicKey(PublicKey::try_from(private.public_key()).unwrap());
        let (fsm, pdu) = fsm.next(&pdu, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::Authentication(_)));
        assert!(matches!(pdu, Some(ProvisioningPDU::PublicKey(_))));
    }

    #[test]
    fn invalid_keyexchange() {
        let fsm = keyexchange();
        let (x, y) = ([0; 32], [0; 32]);
        let pdu = ProvisioningPDU::PublicKey(PublicKey { x, y });
        let (fsm, pdu) = fsm.next(&pdu, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::Failure));
        assert!(
            matches!(pdu, Some(ProvisioningPDU::Failed(e)) if matches!(e.error_code, ErrorCode::InvalidFormat))
        );
    }

    #[test]
    fn valid_confirmation() {
        let mut random = [0; 16];
        OsRng.fill_bytes(&mut random);
        let fsm = confirmation(&random);
        let pdu = ProvisioningPDU::Random(Random { random });
        let (fsm, _pdu) = fsm.next(&pdu, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::DataDistribution(_)));
    }

    #[test]
    fn invalid_confirmation() {
        let mut random = [0; 16];
        let fsm = confirmation(&random);
        // Use a different random to break confirmation...
        OsRng.fill_bytes(&mut random);
        let pdu = ProvisioningPDU::Random(Random { random });
        let (fsm, _pdu) = fsm.next(&pdu, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::Failure));
    }

    fn keyexchange() -> Provisionee {
        let fsm = Provisionee::new(Capabilities::default());
        let invite = ProvisioningPDU::Invite(Invite::default());
        let (fsm, _) = fsm.next(&invite, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::Invitation(_)));
        let start = ProvisioningPDU::Start(Start::default());
        let (fsm, pdu) = fsm.next(&start, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::KeyExchange(_)));
        assert!(matches!(pdu, None));
        fsm
    }

    fn confirmation(random: &[u8]) -> Provisionee {
        let fsm = keyexchange();
        let private = SecretKey::random(OsRng);
        let pdu = ProvisioningPDU::PublicKey(PublicKey::try_from(private.public_key()).unwrap());
        let (fsm, _pdu) = fsm.next(&pdu, &mut OsRng).unwrap();
        let confirmation = match &fsm {
            Provisionee::Authentication(ref auth) => auth.confirm(random).unwrap(),
            _ => panic!("wrong state returned"),
        };
        let pdu = ProvisioningPDU::Confirmation(Confirmation { confirmation });
        let (fsm, pdu) = fsm.next(&pdu, &mut OsRng).unwrap();
        match pdu {
            Some(ProvisioningPDU::Confirmation(c)) => assert_ne!(c.confirmation, confirmation),
            _ => panic!("wrong pdu returned"),
        }
        fsm
    }
}
