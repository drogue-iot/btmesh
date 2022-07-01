use btmesh_common::{NetworkId, Uuid};

#[derive(Copy, Clone)]
pub enum Beacon {
    Unprovisioned(Uuid),
    Provisioned(NetworkId),
    Secure, /* (NetworkId?) */
}
