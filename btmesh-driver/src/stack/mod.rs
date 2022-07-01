use crate::ProvisionedStack;

pub mod provisioned;

pub enum Stack {
    Unprovisioned,
    Provisioned(ProvisionedStack)
}