
#[derive(Copy, Clone)]
pub enum Beacon {
    Unprovisioned,
    Provisioned/* (NetworkId) */,
    Secure/* (NetworkId?) */,
}
