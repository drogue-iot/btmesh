use super::auth_value::{determine_auth_value, AuthValue};
use super::pdu::{
    Capabilities, Confirmation, ProvisioningData, ProvisioningPDU, PublicKey, Random,
};
use super::transcript::Transcript;
use crate::DriverError;
use btmesh_common::crypto::{
    aes_cmac,
    provisioning::{prck, prsk, prsn, try_decrypt_confirmation},
    s1,
};
use heapless::Vec;
use p256::elliptic_curve::ecdh::diffie_hellman;
use p256::SecretKey;
use rand_core::{CryptoRng, RngCore};

pub enum Provisioning {
    Beaconing(Provisionee<Beaconing>),
    Invitation(Provisionee<Invitation>),
    KeyExchange(Provisionee<KeyExchange>),
    Authentication(Provisionee<Authentication>),
    DataDistribution(Provisionee<DataDistribution>),
    Complete(ProvisioningData),
}

impl Provisioning {
    pub fn new(capabilities: Capabilities) -> Self {
        Self::Beaconing(Provisionee {
            transcript: Transcript::default(),
            state: Beaconing { capabilities },
        })
    }

    pub fn next(
        self,
        pdu: ProvisioningPDU,
        mut rng: impl RngCore + CryptoRng,
    ) -> Result<(Self, Option<ProvisioningPDU>), DriverError> {
        match (self, pdu) {
            (Provisioning::Beaconing(mut device), ProvisioningPDU::Invite(invite)) => {
                let capabilities = device.state.capabilities.clone();
                device.transcript.add_invite(&invite)?;
                device.transcript.add_capabilities(&capabilities)?;
                Ok((
                    Provisioning::Invitation(device.into()),
                    Some(ProvisioningPDU::Capabilities(capabilities)),
                ))
            }
            (Provisioning::Invitation(mut device), ProvisioningPDU::Start(start)) => {
                // TODO: spec says to set the "Attention Timer" to 0x00
                device.transcript.add_start(&start)?;
                device
                    .state
                    .auth_value
                    .replace(determine_auth_value(rng, &start)?);
                // TODO: actually let the device/app/thingy know what
                // it is so that it can blink/flash/accept input
                Ok((Provisioning::KeyExchange(device.into()), None))
            }
            (Provisioning::KeyExchange(mut device), ProvisioningPDU::PublicKey(peer_key)) => {
                // TODO: invalid key (sec 5.4.3.1) should fail provisioning (sec 5.4.4)
                device.transcript.add_pubkey_provisioner(&peer_key)?;
                let private = SecretKey::random(rng);
                let public: p256::PublicKey = peer_key.into();
                let secret = &diffie_hellman(private.to_nonzero_scalar(), public.as_affine());
                device.state.shared_secret = Some(secret.as_bytes()[0..].try_into()?);
                let pk: PublicKey = private.public_key().try_into()?;
                device.transcript.add_pubkey_device(&pk)?;
                Ok((
                    Provisioning::Authentication(device.into()),
                    Some(ProvisioningPDU::PublicKey(pk)),
                ))
            }
            (Provisioning::Authentication(mut device), ProvisioningPDU::Confirmation(_value)) => {
                // TODO: should we introduce a sub-state for Input OOB
                // to know when to send back an InputComplete PDU?

                // TODO: verify the confirmation from the provisioner, _value

                let mut random_device = [0; 16];
                rng.fill_bytes(&mut random_device);
                let salt = device.transcript.confirmation_salt()?;
                let key = prck(&device.state.shared_secret, &*salt.into_bytes())?;
                let mut bytes: Vec<u8, 32> = Vec::new();
                bytes.extend_from_slice(&random_device)?;
                bytes.extend_from_slice(&device.state.auth_value.get_bytes())?;
                let confirmation = aes_cmac(&key.into_bytes(), &bytes)?.into_bytes().into();
                device.state.random_device.replace(random_device);
                Ok((
                    Provisioning::Authentication(device),
                    Some(ProvisioningPDU::Confirmation(Confirmation { confirmation })),
                ))
            }
            (Provisioning::Authentication(mut device), ProvisioningPDU::Random(random)) => {
                device.state.random_provisioner.replace(random.random);
                let device_random = device.state.random_device.ok_or(DriverError::CryptoError)?;
                Ok((
                    Provisioning::DataDistribution(device.into()),
                    Some(ProvisioningPDU::Random(Random {
                        random: device_random,
                    })),
                ))
            }
            (Provisioning::DataDistribution(device), ProvisioningPDU::Data(mut data)) => {
                let mut salt = [0; 48];
                salt[0..16].copy_from_slice(&device.transcript.confirmation_salt()?.into_bytes());
                salt[16..32].copy_from_slice(&device.state.random_provisioner);
                salt[32..48].copy_from_slice(&device.state.random_device);
                let salt = &s1(&salt)?.into_bytes()[0..];
                let key = &prsk(&device.state.shared_secret, &salt)?.into_bytes()[0..];
                let nonce = &prsn(&device.state.shared_secret, &salt)?.into_bytes()[3..];

                match try_decrypt_confirmation(&key, &nonce, &mut data.encrypted, &data.mic, None) {
                    Ok(_) => Ok((
                        Provisioning::Complete(ProvisioningData::parse(&data.encrypted)?),
                        Some(ProvisioningPDU::Complete),
                    )),
                    Err(_) => Err(DriverError::CryptoError),
                }
            }
            _ => Err(DriverError::InvalidState),
        }
    }
}

pub struct Provisionee<S> {
    transcript: Transcript,
    state: S,
}

impl From<Provisionee<Beaconing>> for Provisionee<Invitation> {
    fn from(p: Provisionee<Beaconing>) -> Provisionee<Invitation> {
        Provisionee {
            transcript: p.transcript,
            state: Invitation { auth_value: None },
        }
    }
}

impl From<Provisionee<Invitation>> for Provisionee<KeyExchange> {
    fn from(p: Provisionee<Invitation>) -> Provisionee<KeyExchange> {
        Provisionee {
            transcript: p.transcript,
            state: KeyExchange {
                auth_value: p.state.auth_value.unwrap(),
                shared_secret: None,
            },
        }
    }
}

impl From<Provisionee<KeyExchange>> for Provisionee<Authentication> {
    fn from(p: Provisionee<KeyExchange>) -> Provisionee<Authentication> {
        Provisionee {
            transcript: p.transcript,
            state: Authentication {
                auth_value: p.state.auth_value,
                shared_secret: p.state.shared_secret.unwrap(),
                random_device: None,
                random_provisioner: None,
            },
        }
    }
}

impl From<Provisionee<Authentication>> for Provisionee<DataDistribution> {
    fn from(p: Provisionee<Authentication>) -> Provisionee<DataDistribution> {
        Provisionee {
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
pub struct Invitation {
    auth_value: Option<AuthValue>,
}
pub struct KeyExchange {
    auth_value: AuthValue,
    shared_secret: Option<[u8; 32]>,
}
pub struct Authentication {
    auth_value: AuthValue,
    shared_secret: [u8; 32],
    random_device: Option<[u8; 16]>,
    random_provisioner: Option<[u8; 16]>,
}
pub struct DataDistribution {
    shared_secret: [u8; 32],
    random_device: [u8; 16],
    random_provisioner: [u8; 16],
}
