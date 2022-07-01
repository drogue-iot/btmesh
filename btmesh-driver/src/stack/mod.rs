use crate::ProvisionedStack;

pub mod interface;
pub mod provisioned;

pub enum Stack {
    Unprovisioned,
    Provisioned(ProvisionedStack),
}
