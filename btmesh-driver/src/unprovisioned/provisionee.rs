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
    fn next(self, pdu: ProvisioningPDU) -> Self {
        match pdu {
            ProvisioningPDU::Invite(invite) => {
                if let Provisioning::Invitation(mut device) = self {
                    // TODO: How best to use these Results?
                    device.transcript.add_invite(&invite);
                    device.transcript.add_capabilities(&device.capabilities);
                    // TODO: send a capabilities PDU or let caller do it?
                    Provisioning::KeyExchange(device.into())
                } else {
                    self
                }
            }
            ProvisioningPDU::Capabilities(_) => self,
            ProvisioningPDU::Start(_) => self,
            ProvisioningPDU::PublicKey(_) => self,
            ProvisioningPDU::InputComplete => self,
            ProvisioningPDU::Confirmation(_) => self,
            ProvisioningPDU::Random(_) => self,
            ProvisioningPDU::Data(_) => self,
            ProvisioningPDU::Complete => self,
            ProvisioningPDU::Failed(_) => self,
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
