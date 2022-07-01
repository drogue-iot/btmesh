use crate::DriverError;
use p256::elliptic_curve::ecdh::diffie_hellman;
use p256::SecretKey;
use rand_core::{CryptoRng, RngCore};

use super::auth_value::{determine_auth_value, AuthValue};
use super::pdu::{Capabilities, ProvisioningPDU, PublicKey, Random};
use super::transcript::Transcript;

enum Provisioning {
    Beaconing(Provisionee<Beaconing>),
    Invitation(Provisionee<Invitation>),
    KeyExchange(Provisionee<KeyExchange>),
    Authentication(Provisionee<Authentication>),
    DataDistribution(Provisionee<DataDistribution>),
    Complete(Provisionee<Complete>),
}

impl Provisioning {
    fn next(
        self,
        pdu: ProvisioningPDU,
        rng: impl RngCore + CryptoRng,
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
                device.state.shared_secret = Some(diffie_hellman(
                    private.to_nonzero_scalar(),
                    public.as_affine(),
                ));
                let pk: PublicKey = private.public_key().try_into()?;
                device.transcript.add_pubkey_device(&pk)?;
                Ok((
                    Provisioning::Authentication(device.into()),
                    Some(ProvisioningPDU::PublicKey(pk)),
                ))
            }
            (Provisioning::Authentication(device), ProvisioningPDU::Confirmation(_value)) => {
                // TODO: should we introduce a sub-state for Input OOB
                // to know when to send back an InputComplete PDU?

                // TODO: confirm the value and send back a Confirmation PDU
                Ok((Provisioning::Authentication(device), None))
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
            (Provisioning::DataDistribution(device), ProvisioningPDU::Data(_data)) => {
                // TODO: do something with the data!
                Ok((Provisioning::Complete(device.into()), None))
            }
            _ => Err(DriverError::InvalidState),
        }
    }
}

struct Provisionee<S> {
    transcript: Transcript,
    state: S,
}

impl Provisionee<Beaconing> {
    fn new(capabilities: Capabilities) -> Self {
        Provisionee {
            transcript: Transcript::default(),
            state: Beaconing { capabilities },
        }
    }
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
            state: DataDistribution,
        }
    }
}

impl From<Provisionee<DataDistribution>> for Provisionee<Complete> {
    fn from(p: Provisionee<DataDistribution>) -> Provisionee<Complete> {
        Provisionee {
            transcript: p.transcript,
            state: Complete,
        }
    }
}

struct Beaconing {
    capabilities: Capabilities,
}
struct Invitation {
    auth_value: Option<AuthValue>,
}
struct KeyExchange {
    auth_value: AuthValue,
    shared_secret: Option<p256::ecdh::SharedSecret>,
}
struct Authentication {
    auth_value: AuthValue,
    shared_secret: p256::ecdh::SharedSecret,
    random_device: Option<[u8; 16]>,
    random_provisioner: Option<[u8; 16]>,
}
struct DataDistribution;
struct Complete;
