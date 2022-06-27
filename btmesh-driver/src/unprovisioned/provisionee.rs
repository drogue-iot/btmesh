use crate::DriverError;

use super::pdu::{Capabilities, ProvisioningPDU};
use super::transcript::Transcript;

enum Provisioning {
    Beaconing(Provisionee<Beaconing>),
    Invitation(Provisionee<Invitation>),
    KeyExchange(Provisionee<KeyExchange>),
    Authentication(Provisionee<Authentication>),
    DataDistribution(Provisionee<DataDistribution>),
}

impl Provisioning {
    fn next(self, pdu: ProvisioningPDU) -> Result<Self, DriverError> {
        match (self, pdu) {
            (Provisioning::Beaconing(mut device), ProvisioningPDU::Invite(invite)) => {
                device.transcript.add_invite(&invite)?;
                device.transcript.add_capabilities(&device.capabilities)?;
                // TODO: send a capabilities PDU or let caller do it?
                Ok(Provisioning::Invitation(device.into()))
            }
            (Provisioning::Invitation(mut device), ProvisioningPDU::Start(start)) => {
                device.transcript.add_start(&start)?;
                Ok(Provisioning::KeyExchange(device.into()))
            }
            (Provisioning::Invitation(mut device), ProvisioningPDU::PublicKey(key)) => {
                device.transcript.add_pubkey_provisioner(&key)?;
                Ok(Provisioning::KeyExchange(device.into()))
            }
            _ => todo!(),
        }
    }
}

struct Provisionee<S> {
    capabilities: Capabilities,
    transcript: Transcript,
    state: S,
}

impl Provisionee<Beaconing> {
    fn new(capabilities: Capabilities) -> Self {
        Provisionee {
            capabilities: capabilities,
            transcript: Transcript::default(),
            state: Beaconing,
        }
    }
}

impl From<Provisionee<Beaconing>> for Provisionee<Invitation> {
    fn from(p: Provisionee<Beaconing>) -> Provisionee<Invitation> {
        Provisionee {
            capabilities: p.capabilities,
            transcript: p.transcript,
            state: Invitation,
        }
    }
}

impl From<Provisionee<Invitation>> for Provisionee<KeyExchange> {
    fn from(p: Provisionee<Invitation>) -> Provisionee<KeyExchange> {
        Provisionee {
            capabilities: p.capabilities,
            transcript: p.transcript,
            state: KeyExchange,
        }
    }
}

impl From<Provisionee<KeyExchange>> for Provisionee<Authentication> {
    fn from(p: Provisionee<KeyExchange>) -> Provisionee<Authentication> {
        Provisionee {
            capabilities: p.capabilities,
            transcript: p.transcript,
            state: Authentication,
        }
    }
}

impl From<Provisionee<Authentication>> for Provisionee<DataDistribution> {
    fn from(p: Provisionee<Authentication>) -> Provisionee<DataDistribution> {
        Provisionee {
            capabilities: p.capabilities,
            transcript: p.transcript,
            state: DataDistribution,
        }
    }
}

struct Beaconing;
struct Invitation;
struct KeyExchange;
struct Authentication;
struct DataDistribution;
