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
