use super::pdu::ProvisioningPDU;

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
            ProvisioningPDU::Invite(_) => self,
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
    state: S,
}

impl Provisionee<Beaconing> {
    fn new() -> Self {
        Provisionee { state: Beaconing }
    }
}

impl From<Provisionee<Beaconing>> for Provisionee<Invitation> {
    fn from(_: Provisionee<Beaconing>) -> Provisionee<Invitation> {
        Provisionee { state: Invitation }
    }
}

impl From<Provisionee<Invitation>> for Provisionee<KeyExchange> {
    fn from(_: Provisionee<Invitation>) -> Provisionee<KeyExchange> {
        Provisionee { state: KeyExchange }
    }
}

impl From<Provisionee<KeyExchange>> for Provisionee<Authentication> {
    fn from(_: Provisionee<KeyExchange>) -> Provisionee<Authentication> {
        Provisionee {
            state: Authentication,
        }
    }
}

impl From<Provisionee<Authentication>> for Provisionee<DataDistribution> {
    fn from(_: Provisionee<Authentication>) -> Provisionee<DataDistribution> {
        Provisionee {
            state: DataDistribution,
        }
    }
}

struct Beaconing;
struct Invitation;
struct KeyExchange;
struct Authentication;
struct DataDistribution;
