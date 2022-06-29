use crate::DriverError;

use super::pdu::{Capabilities, ProvisioningPDU};
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
    fn next(self, pdu: ProvisioningPDU) -> Result<(Self, Option<ProvisioningPDU>), DriverError> {
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
                Ok((Provisioning::KeyExchange(device.into()), None))
            }
            (Provisioning::KeyExchange(mut device), ProvisioningPDU::PublicKey(key)) => {
                device.transcript.add_pubkey_provisioner(&key)?;
                // TODO: invalid key (sec 5.4.3.1) should fail provisioning
                Ok((Provisioning::Authentication(device.into()), None))
            }
            (Provisioning::Authentication(device), ProvisioningPDU::Confirmation(_value)) => {
                // TODO: should we introduce a sub-state for Input OOB
                // to know when to send back an InputComplete PDU?

                // TODO: confirm the value and send back a Confirmation PDU
                Ok((Provisioning::Authentication(device), None))
            }
            (Provisioning::Authentication(device), ProvisioningPDU::Random(_value)) => {
                // TODO: should we introduce a sub-state for Input OOB
                // to know when to send back an InputCompletePDU?

                // check the value and send back a Random PDU
                Ok((Provisioning::DataDistribution(device.into()), None))
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
            state: Beaconing {
                capabilities: capabilities,
            },
        }
    }
}

impl From<Provisionee<Beaconing>> for Provisionee<Invitation> {
    fn from(p: Provisionee<Beaconing>) -> Provisionee<Invitation> {
        Provisionee {
            transcript: p.transcript,
            state: Invitation,
        }
    }
}

impl From<Provisionee<Invitation>> for Provisionee<KeyExchange> {
    fn from(p: Provisionee<Invitation>) -> Provisionee<KeyExchange> {
        Provisionee {
            transcript: p.transcript,
            state: KeyExchange,
        }
    }
}

impl From<Provisionee<KeyExchange>> for Provisionee<Authentication> {
    fn from(p: Provisionee<KeyExchange>) -> Provisionee<Authentication> {
        Provisionee {
            transcript: p.transcript,
            state: Authentication,
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
struct Invitation;
struct KeyExchange;
struct Authentication;
struct DataDistribution;
struct Complete;
